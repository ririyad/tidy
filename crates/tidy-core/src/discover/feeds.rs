use scraper::{Html, Selector};
use url::Url;

use crate::discover::types::{DiscoveredUrl, DiscoverySource};
use crate::error::{Result, TidyError};
use crate::http::HttpClient;

pub struct FeedDiscovery {
    pub feed_url: Url,
    pub entries: Vec<DiscoveredUrl>,
}

const COMMON_FEED_PATHS: &[&str] = &[
    "feed",
    "rss.xml",
    "atom.xml",
    "index.xml",
    "feed.xml",
    "feed.json",
    "rss",
    "atom",
    "feeds/posts/default",
    "index.json",
];

pub async fn discover_feed_urls(
    client: &HttpClient,
    prefix: &Url,
    warnings: &mut Vec<String>,
) -> Result<Vec<Url>> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let push = |url: Url, candidates: &mut Vec<Url>, seen: &mut std::collections::HashSet<String>| {
        let key = url.as_str().to_owned();
        if seen.insert(key) {
            candidates.push(url);
        }
    };

    // From <link rel="alternate"> on the prefix page
    match client.get_text(prefix).await {
        Ok(html) => {
            for link in extract_alternate_feeds(prefix, &html) {
                push(link, &mut candidates, &mut seen);
            }
        }
        Err(error) => warnings.push(format!("prefix page fetch failed: {error}")),
    }

    // Common paths under the prefix and site origin
    for base in [prefix.clone(), origin_url(prefix)?] {
        for path in COMMON_FEED_PATHS {
            if let Ok(url) = base.join(path) {
                push(url, &mut candidates, &mut seen);
            }
        }
    }

    Ok(candidates)
}

pub fn extract_alternate_feeds(base: &Url, html: &str) -> Vec<Url> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse(r#"link[rel~="alternate"]"#) else {
        return Vec::new();
    };

    let mut urls = Vec::new();
    for element in document.select(&selector) {
        let type_attr = element.value().attr("type").unwrap_or("").to_ascii_lowercase();
        let href = match element.value().attr("href") {
            Some(href) => href,
            None => continue,
        };

        let looks_like_feed = type_attr.contains("rss")
            || type_attr.contains("atom")
            || type_attr.contains("json")
            || href.contains("rss")
            || href.contains("atom")
            || href.contains("feed");

        if !looks_like_feed {
            continue;
        }

        if let Ok(url) = base.join(href) {
            urls.push(url);
        }
    }
    urls
}

pub fn parse_feed_entries(feed_url: &Url, body: &[u8]) -> Result<Vec<DiscoveredUrl>> {
    let feed = feed_rs::parser::parse(body).map_err(|error| TidyError::Feed {
        url: feed_url.to_string(),
        message: error.to_string(),
    })?;

    let mut entries = Vec::new();
    for entry in feed.entries {
        let Some(link) = entry
            .links
            .iter()
            .find(|link| {
                link.rel.as_deref() == Some("alternate")
                    || link.media_type.as_deref().is_some_and(|m| m.starts_with("text/html"))
                    || link.rel.is_none()
            })
            .or_else(|| entry.links.first())
        else {
            continue;
        };

        let Ok(url) = Url::parse(&link.href).or_else(|_| feed_url.join(&link.href)) else {
            continue;
        };

        let title = entry.title.map(|text| text.content);
        let published = entry
            .published
            .or(entry.updated)
            .map(|dt| dt.to_rfc3339());

        entries.push(DiscoveredUrl {
            url,
            title,
            published,
            source: DiscoverySource::Feed,
        });
    }

    Ok(entries)
}

fn origin_url(url: &Url) -> Result<Url> {
    let mut origin = url.clone();
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    Ok(origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_alternate_link_feeds() {
        let html = r#"
            <html><head>
              <link rel="alternate" type="application/rss+xml" href="/blog/rss.xml">
              <link rel="stylesheet" href="/style.css">
            </head></html>
        "#;
        let base = Url::parse("https://example.com/blog/").unwrap();
        let feeds = extract_alternate_feeds(&base, html);
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].as_str(), "https://example.com/blog/rss.xml");
    }

    #[test]
    fn parses_atom_fixture() {
        let atom = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Example</title>
          <entry>
            <title>Hello</title>
            <link href="https://example.com/blog/hello"/>
            <updated>2026-01-02T00:00:00Z</updated>
          </entry>
        </feed>"#;
        let feed_url = Url::parse("https://example.com/blog/atom.xml").unwrap();
        let entries = parse_feed_entries(&feed_url, atom).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url.as_str(), "https://example.com/blog/hello");
        assert_eq!(entries[0].title.as_deref(), Some("Hello"));
    }
}
