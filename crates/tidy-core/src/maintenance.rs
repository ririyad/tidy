use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::{Result, TidyError};
use crate::extract::render_markdown_html;
use crate::fetch::{FrontMatterHighlight, parse_frontmatter};
use crate::highlights::from_frontmatter;
use crate::index::{ArticleRecord, HighlightRow, Index};
use crate::vault::Vault;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupReport {
    pub destination: PathBuf,
    pub copied_files: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReindexReport {
    pub scanned: usize,
    pub upserted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub warnings: Vec<String>,
}

/// Copy vault content (Sources, attachments, config) into a timestamped folder.
pub fn backup_vault(vault: &Vault, destination_parent: impl AsRef<Path>) -> Result<BackupReport> {
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let dest = destination_parent
        .as_ref()
        .join(format!("tidy-backup-{stamp}"));
    fs::create_dir_all(&dest)?;

    let mut copied = 0usize;
    copied += copy_tree(&vault.root().join("Sources"), &dest.join("Sources"))?;
    copied += copy_tree(&vault.root().join("attachments"), &dest.join("attachments"))?;
    copied += copy_tree(&vault.root().join(".tidy"), &dest.join(".tidy"))?;

    Ok(BackupReport {
        destination: dest,
        copied_files: copied,
    })
}

/// Rebuild the SQLite index from markdown files under `Sources/`.
pub fn reindex_vault(vault: &Vault) -> Result<ReindexReport> {
    let index = Index::open(vault.database_path())?;
    let sources = index.list_sources()?;
    let mut report = ReindexReport {
        scanned: 0,
        upserted: 0,
        skipped: 0,
        failed: 0,
        warnings: Vec::new(),
    };

    let sources_root = vault.sources_dir();
    if !sources_root.exists() {
        return Ok(report);
    }

    for entry in walk_markdown(&sources_root)? {
        report.scanned += 1;
        match reindex_file(vault, &index, &sources, &entry) {
            Ok(true) => report.upserted += 1,
            Ok(false) => report.skipped += 1,
            Err(error) => {
                report.failed += 1;
                report
                    .warnings
                    .push(format!("{}: {error}", entry.display()));
            }
        }
    }

    Ok(report)
}

fn reindex_file(
    vault: &Vault,
    index: &Index,
    sources: &[crate::index::SourceRow],
    path: &Path,
) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    let Some(fm) = parse_frontmatter(&text)? else {
        return Ok(false);
    };
    let body = strip_frontmatter_body(&text).to_owned();
    let relative = path
        .strip_prefix(vault.root())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string());

    let source = sources.iter().find(|source| {
        let slug = source_slug_for_prefix(&source.url_prefix);
        slug == fm.source || relative.starts_with(&format!("Sources/{}/", fm.source))
    });

    let Some(source) = source else {
        return Err(TidyError::Message(format!(
            "no source matched frontmatter source `{}`",
            fm.source
        )));
    };

    let article_id = index.upsert_article(&ArticleRecord {
        source_id: source.id,
        url: fm.url.clone(),
        canonical_url: None,
        path: relative,
        title: fm.title.clone(),
        author: fm.author.clone(),
        published_at: fm.published.clone(),
        fetched_at: fm.fetched.clone(),
        word_count: fm.word_count as i64,
        excerpt: fm.excerpt.clone(),
        body: body.clone(),
        rendered_html: render_markdown_html(&body),
        content_hash: fm.content_hash.clone(),
        state: fm.state.clone(),
        starred: fm.starred,
        archived: fm.archived,
        revision: fm.revision as i64,
        quality: fm.extraction.quality.clone(),
        tags: fm.tags.clone(),
    })?;

    let highlights: Vec<HighlightRow> = fm
        .highlights
        .iter()
        .map(|item: &FrontMatterHighlight| from_frontmatter(article_id, item))
        .collect();
    index.replace_highlights(article_id, &highlights)?;
    Ok(true)
}

fn walk_markdown(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn copy_tree(from: &Path, to: &Path) -> Result<usize> {
    if !from.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    fn copy_inner(from: &Path, to: &Path, count: &mut usize) -> Result<()> {
        if from.is_dir() {
            fs::create_dir_all(to)?;
            for entry in fs::read_dir(from)? {
                let entry = entry?;
                let src = entry.path();
                let dest = to.join(entry.file_name());
                copy_inner(&src, &dest, count)?;
            }
        } else {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(from, to)?;
            *count += 1;
        }
        Ok(())
    }
    copy_inner(from, to, &mut count)?;
    Ok(count)
}

fn strip_frontmatter_body(text: &str) -> &str {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return text;
    }
    let rest = &trimmed[3..];
    if let Some(end) = rest.find("\n---") {
        let after = &rest[end + 4..];
        after.strip_prefix('\n').unwrap_or(after)
    } else {
        text
    }
}

fn source_slug_for_prefix(prefix: &str) -> String {
    if let Ok(url) = url::Url::parse(prefix) {
        crate::fetch::source_slug(&url)
    } else if let Ok(url) = crate::discover::parse_prefix(prefix) {
        crate::fetch::source_slug(&url)
    } else {
        prefix.to_owned()
    }
}
