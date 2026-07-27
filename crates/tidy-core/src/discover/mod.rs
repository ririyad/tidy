mod crawl;
mod feeds;
mod prefix;
mod sitemap;
mod types;

use std::collections::BTreeMap;

use url::Url;

use crate::error::{Result, TidyError};
use crate::http::{HttpClient, HttpClientConfig};

pub use types::{CrawlLimits, DiscoverOptions, DiscoverReport, DiscoveredUrl, DiscoverySource};

use feeds::{FeedDiscovery, discover_feed_urls, parse_feed_entries};
use prefix::{is_prefix_root, matches_prefix, normalize_prefix, scrub_tracking_params};
use sitemap::{discover_sitemaps, parse_sitemap_urls};

/// Discover article URLs under a user-supplied URL prefix.
///
/// Order: feed (link tags + common paths) → sitemap (robots + /sitemap.xml)
/// → HTML BFS crawl fallback.
pub async fn discover(options: DiscoverOptions) -> Result<DiscoverReport> {
    let mut prefix = normalize_prefix(&options.url_prefix)?;
    let mut client_config = HttpClientConfig::default();
    client_config.cache_dir = options.cache_dir.clone();
    let client = HttpClient::new(client_config)?;

    let mut warnings = Vec::new();

    // Follow redirects so prefixes like blog.example.com → example.com/blog stay coherent.
    match client.get_bytes(&prefix).await {
        Ok(response) => {
            if let Ok(resolved) = normalize_prefix(&response.url) {
                let before = prefix.as_str().trim_end_matches('/');
                let after = resolved.as_str().trim_end_matches('/');
                if before != after {
                    warnings.push(format!("prefix redirected: {} → {}", prefix, resolved));
                    prefix = resolved;
                }
            }
        }
        Err(error) => warnings.push(format!("prefix page fetch failed: {error}")),
    }

    let mut feed_url = None;
    let mut sitemap_urls = Vec::new();
    let mut found: BTreeMap<String, DiscoveredUrl> = BTreeMap::new();

    // 1. Feeds
    match try_feeds(&client, &prefix, &mut warnings).await {
        Ok(Some(discovery)) => {
            feed_url = Some(discovery.feed_url.clone());
            for entry in discovery.entries {
                if matches_prefix(&entry.url, &prefix) && !is_prefix_root(&entry.url, &prefix) {
                    insert_url(&mut found, entry);
                }
            }
        }
        Ok(None) => {}
        Err(error) => warnings.push(format!("feed discovery: {error}")),
    }

    let feed_hits = found.len();

    // 2. Sitemaps — always useful for backfill completeness.
    match try_sitemaps(&client, &prefix, &mut warnings).await {
        Ok(urls) => {
            sitemap_urls = urls.sitemap_documents.clone();
            for entry in urls.entries {
                if matches_prefix(&entry.url, &prefix) && !is_prefix_root(&entry.url, &prefix) {
                    insert_url(&mut found, entry);
                }
            }
        }
        Err(error) => warnings.push(format!("sitemap discovery: {error}")),
    }

    // 3. Crawl fallback when we still have nothing under the prefix
    let mut limits = options.limits.clone();
    if let Some(max_pages) = options.overrides.max_pages {
        limits.page_cap = max_pages.max(1);
    }
    let used_crawl = if found.is_empty() {
        match crawl::crawl_prefix(
            &client,
            &prefix,
            &limits,
            options.overrides.pagination_link_selector.as_deref(),
            &mut warnings,
        )
        .await
        {
            Ok(entries) => {
                for entry in entries {
                    if !is_prefix_root(&entry.url, &prefix) {
                        insert_url(&mut found, entry);
                    }
                }
                true
            }
            Err(error) => {
                warnings.push(format!("crawl fallback: {error}"));
                false
            }
        }
    } else {
        false
    };

    let mut urls: Vec<_> = found.into_values().collect();
    urls.sort_by(|a, b| {
        b.source
            .rank()
            .cmp(&a.source.rank())
            .then_with(|| match (&b.published, &a.published) {
                (Some(left), Some(right)) => left.cmp(right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.url.as_str().cmp(b.url.as_str()),
            })
            .then_with(|| a.url.as_str().cmp(b.url.as_str()))
    });

    if let Some(limit) = options.limit {
        urls.truncate(limit);
    }

    let primary_source = if feed_hits > 0 {
        DiscoverySource::Feed
    } else if urls.iter().any(|u| u.source == DiscoverySource::Sitemap) {
        DiscoverySource::Sitemap
    } else if used_crawl || urls.iter().any(|u| u.source == DiscoverySource::Crawl) {
        DiscoverySource::Crawl
    } else if !urls.is_empty() {
        urls[0].source
    } else {
        DiscoverySource::None
    };

    Ok(DiscoverReport {
        prefix,
        urls,
        feed_url,
        sitemap_urls,
        primary_source,
        warnings,
    })
}

async fn try_feeds(
    client: &HttpClient,
    prefix: &Url,
    warnings: &mut Vec<String>,
) -> Result<Option<FeedDiscovery>> {
    let candidates = discover_feed_urls(client, prefix, warnings).await?;
    for candidate in candidates {
        match client.get_bytes(&candidate).await {
            Ok(response) => match parse_feed_entries(&candidate, &response.body) {
                Ok(entries) if !entries.is_empty() => {
                    return Ok(Some(FeedDiscovery {
                        feed_url: candidate,
                        entries,
                    }));
                }
                Ok(_) => warnings.push(format!("empty feed at {candidate}")),
                Err(error) => warnings.push(format!("feed parse {candidate}: {error}")),
            },
            Err(error) => warnings.push(format!("feed fetch {candidate}: {error}")),
        }
    }
    Ok(None)
}

struct SitemapDiscovery {
    sitemap_documents: Vec<Url>,
    entries: Vec<DiscoveredUrl>,
}

async fn try_sitemaps(
    client: &HttpClient,
    prefix: &Url,
    warnings: &mut Vec<String>,
) -> Result<SitemapDiscovery> {
    let documents = discover_sitemaps(client, prefix, warnings).await?;
    let mut entries = Vec::new();
    let mut seen_docs = Vec::new();

    for doc in documents {
        match parse_sitemap_urls(client, &doc, 0, 3, warnings).await {
            Ok(urls) => {
                seen_docs.push(doc);
                for url in urls {
                    entries.push(DiscoveredUrl {
                        url,
                        title: None,
                        published: None,
                        source: DiscoverySource::Sitemap,
                    });
                }
            }
            Err(error) => {
                // Speculative sitemap paths 404 often; keep noise low.
                let message = error.to_string();
                if message.contains("404") {
                    // intentionally quiet
                } else {
                    warnings.push(format!("sitemap {doc}: {error}"));
                }
            }
        }
    }

    Ok(SitemapDiscovery {
        sitemap_documents: seen_docs,
        entries,
    })
}

fn insert_url(map: &mut BTreeMap<String, DiscoveredUrl>, mut entry: DiscoveredUrl) {
    scrub_tracking_params(&mut entry.url);
    let key = entry.url.as_str().to_owned();
    match map.get(&key) {
        Some(existing) => {
            // Prefer feed metadata over bare sitemap/crawl URLs.
            if existing.source.rank() < entry.source.rank() {
                map.insert(key, entry);
            } else if existing.title.is_none() && entry.title.is_some() {
                map.insert(key, entry);
            }
        }
        None => {
            map.insert(key, entry);
        }
    }
}

pub fn parse_prefix(input: &str) -> Result<Url> {
    let trimmed = input.trim();
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    let url = Url::parse(&with_scheme).map_err(TidyError::from)?;
    if url.host_str().is_none() {
        return Err(TidyError::Message(format!(
            "URL has no host: {with_scheme}"
        )));
    }
    Ok(url)
}
