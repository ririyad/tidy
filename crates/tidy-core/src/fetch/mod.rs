mod images;
mod slug;
mod writer;

use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;
use url::Url;

use crate::discover::{discover, CrawlLimits, DiscoverOptions, DiscoveredUrl};
use crate::error::{Result, TidyError};
use crate::extract::{content_hash, extract_article, render_markdown_html, ArticleHints};
use crate::http::{HttpClient, HttpClientConfig};
use crate::index::{ArticleRecord, Index, SourceRecord};
use crate::vault::Vault;

pub use images::localize_images;
pub use slug::{article_stem, source_slug};
pub use writer::{
    read_existing_frontmatter, write_article_file, ArticleFrontMatter, ExtractionInfo,
    WriteOutcomeStatus,
};

#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub url_prefix: Url,
    pub vault: PathBuf,
    pub limit: Option<usize>,
    pub download_images: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchReport {
    pub source_id: i64,
    pub source_slug: String,
    pub discovered: usize,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub needs_review: usize,
    pub articles: Vec<FetchedArticleSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchedArticleSummary {
    pub url: String,
    pub path: String,
    pub title: String,
    pub status: FetchStatus,
    pub quality: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FetchStatus {
    Added,
    Updated,
    Skipped,
    Failed,
}

/// Discover posts under a prefix, extract them, write markdown, and index them.
pub async fn fetch(options: FetchOptions) -> Result<FetchReport> {
    let summary = Vault::initialize(&options.vault)?;
    let vault = Vault::open(&summary.path)?;
    let mut index = Index::open(vault.database_path())?;

    let slug = source_slug(&options.url_prefix);
    let source = index.upsert_source(&SourceRecord {
        url_prefix: options.url_prefix.as_str().to_owned(),
        title: slug.clone(),
        feed_url: None,
        discovery_mode: "auto".into(),
        interval_minutes: 360,
        backfill_policy: "ask".into(),
        enabled: true,
    })?;

    let mut client_config = HttpClientConfig::default();
    client_config.cache_dir = Some(vault.cache_dir());
    let client = HttpClient::new(client_config)?;

    let discovery = discover(DiscoverOptions {
        url_prefix: options.url_prefix.clone(),
        limit: options.limit,
        cache_dir: Some(vault.cache_dir()),
        limits: CrawlLimits::default(),
    })
    .await?;

    if let Some(feed) = &discovery.feed_url {
        index.set_source_feed_url(source.id, feed.as_str())?;
    }

    let run_id = index.start_fetch_run(source.id)?;
    let mut report = FetchReport {
        source_id: source.id,
        source_slug: slug.clone(),
        discovered: discovery.urls.len(),
        added: 0,
        updated: 0,
        skipped: 0,
        failed: 0,
        needs_review: 0,
        articles: Vec::new(),
        warnings: discovery.warnings.clone(),
    };

    for item in &discovery.urls {
        match fetch_one(
            &client,
            &vault,
            &mut index,
            source.id,
            &slug,
            item,
            options.download_images,
        )
        .await
        {
            Ok(summary) => {
                match summary.status {
                    FetchStatus::Added => report.added += 1,
                    FetchStatus::Updated => report.updated += 1,
                    FetchStatus::Skipped => report.skipped += 1,
                    FetchStatus::Failed => report.failed += 1,
                }
                if summary.quality == "needs_review" {
                    report.needs_review += 1;
                }
                report.articles.push(summary);
            }
            Err(error) => {
                report.failed += 1;
                report.warnings.push(format!("{}: {error}", item.url));
                report.articles.push(FetchedArticleSummary {
                    url: item.url.to_string(),
                    path: String::new(),
                    title: item.title.clone().unwrap_or_else(|| item.url.to_string()),
                    status: FetchStatus::Failed,
                    quality: "needs_review".into(),
                });
            }
        }
    }

    index.finish_fetch_run(
        run_id,
        "ok",
        report.discovered as i64,
        report.added as i64,
        report.updated as i64,
        report.skipped as i64,
        report.failed as i64,
    )?;
    index.touch_source_fetch(source.id)?;

    Ok(report)
}

async fn fetch_one(
    client: &HttpClient,
    vault: &Vault,
    index: &mut Index,
    source_id: i64,
    source_slug: &str,
    item: &DiscoveredUrl,
    download_images: bool,
) -> Result<FetchedArticleSummary> {
    let response = client.get_bytes(&item.url).await?;
    let html = String::from_utf8(response.body).map_err(|error| {
        TidyError::extract(
            item.url.as_str(),
            format!("response was not UTF-8: {error}"),
        )
    })?;

    let hints = ArticleHints {
        title: item.title.clone(),
        author: None,
        published: item.published.clone(),
        excerpt: None,
    };
    let extracted = extract_article(&html, &item.url, &hints)?;
    let stem = article_stem(
        &extracted.title,
        &item.url,
        extracted.published.as_deref(),
    );

    let markdown = if download_images {
        localize_images(
            client,
            vault,
            source_slug,
            &stem,
            &item.url,
            &extracted.markdown,
        )
        .await?
    } else {
        extracted.markdown.clone()
    };

    let hash = content_hash(&markdown);
    let existing = index.find_article_by_url(item.url.as_str())?;
    let existing_fm = existing
        .as_ref()
        .and_then(|record| {
            let path = vault.root().join(&record.path);
            read_existing_frontmatter(&path).ok().flatten()
        })
        .or_else(|| {
            let relative = format!("Sources/{source_slug}/{stem}.md");
            read_existing_frontmatter(&vault.root().join(relative))
                .ok()
                .flatten()
        });

    if let Some(fm) = &existing_fm {
        if fm.content_hash == hash {
            return Ok(FetchedArticleSummary {
                url: item.url.to_string(),
                path: existing
                    .as_ref()
                    .map(|record| record.path.clone())
                    .unwrap_or_else(|| format!("Sources/{source_slug}/{stem}.md")),
                title: fm.title.clone(),
                status: FetchStatus::Skipped,
                quality: fm.extraction.quality.clone(),
            });
        }
    }

    let now = Utc::now().to_rfc3339();
    let revision = existing_fm
        .as_ref()
        .map(|fm| fm.revision + 1)
        .unwrap_or(1);
    let frontmatter = ArticleFrontMatter {
        title: extracted.title.clone(),
        url: item.url.to_string(),
        source: source_slug.to_owned(),
        author: extracted.author.clone(),
        published: extracted.published.clone(),
        fetched: now.clone(),
        tags: vec![format!("source/{source_slug}")],
        word_count: extracted.word_count as u32,
        reading_time: extracted.reading_time,
        lang: extracted.lang.clone(),
        excerpt: extracted.excerpt.clone(),
        state: existing_fm
            .as_ref()
            .map(|fm| fm.state.clone())
            .unwrap_or_else(|| "unread".into()),
        starred: existing_fm.as_ref().map(|fm| fm.starred).unwrap_or(false),
        archived: existing_fm.as_ref().map(|fm| fm.archived).unwrap_or(false),
        content_hash: hash.clone(),
        revision,
        extraction: ExtractionInfo {
            engine: "dom_smoothie".into(),
            quality: extracted.quality.as_str().to_owned(),
        },
    };

    let outcome = write_article_file(vault, source_slug, &stem, &frontmatter, &markdown)?;
    let rendered = render_markdown_html(&markdown);

    index.upsert_article(&ArticleRecord {
        source_id,
        url: item.url.to_string(),
        canonical_url: extracted.canonical_url.clone(),
        path: outcome.relative_path.clone(),
        title: extracted.title.clone(),
        author: extracted.author.clone(),
        published_at: extracted.published.clone(),
        fetched_at: now,
        word_count: extracted.word_count as i64,
        excerpt: extracted.excerpt.clone(),
        body: markdown,
        rendered_html: rendered,
        content_hash: hash,
        state: frontmatter.state.clone(),
        starred: frontmatter.starred,
        archived: frontmatter.archived,
        revision: frontmatter.revision as i64,
        quality: extracted.quality.as_str().to_owned(),
        tags: frontmatter.tags.clone(),
    })?;

    let status = match outcome.status {
        WriteOutcomeStatus::Created => FetchStatus::Added,
        WriteOutcomeStatus::Updated => FetchStatus::Updated,
        WriteOutcomeStatus::Unchanged => FetchStatus::Skipped,
    };

    Ok(FetchedArticleSummary {
        url: item.url.to_string(),
        path: outcome.relative_path,
        title: frontmatter.title,
        status,
        quality: frontmatter.extraction.quality,
    })
}
