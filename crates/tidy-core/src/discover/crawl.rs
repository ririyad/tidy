use std::collections::{HashSet, VecDeque};

use scraper::{Html, Selector};
use url::Url;

use crate::discover::prefix::{matches_prefix, same_host};
use crate::discover::types::{CrawlLimits, DiscoveredUrl, DiscoverySource};
use crate::error::Result;
use crate::http::HttpClient;

pub async fn crawl_prefix(
    client: &HttpClient,
    prefix: &Url,
    limits: &CrawlLimits,
    warnings: &mut Vec<String>,
) -> Result<Vec<DiscoveredUrl>> {
    let mut queue: VecDeque<(Url, u32)> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut discovered: Vec<DiscoveredUrl> = Vec::new();
    let mut pages_fetched = 0usize;

    queue.push_back((prefix.clone(), 0));

    while let Some((url, depth)) = queue.pop_front() {
        let key = canonicalize(&url);
        if !visited.insert(key) {
            continue;
        }
        if pages_fetched >= limits.page_cap {
            warnings.push(format!(
                "crawl page cap ({}) reached",
                limits.page_cap
            ));
            break;
        }

        if !client.is_allowed(&url).await.unwrap_or(true) {
            warnings.push(format!("robots blocked crawl of {url}"));
            continue;
        }

        let response = match client.get_bytes(&url).await {
            Ok(response) => response,
            Err(error) => {
                warnings.push(format!("crawl fetch {url}: {error}"));
                continue;
            }
        };
        pages_fetched += 1;

        let content_type = response
            .content_type
            .as_deref()
            .unwrap_or("text/html")
            .to_ascii_lowercase();
        if !content_type.contains("html") && !content_type.contains("xml") {
            continue;
        }

        let Ok(html) = String::from_utf8(response.body) else {
            continue;
        };

        // The prefix page itself is an index; deeper pages that match the
        // prefix are candidate articles (excluding obvious pagination/index paths).
        if depth > 0 && matches_prefix(&url, prefix) && looks_like_article(&url, prefix) {
            let title = extract_title(&html);
            discovered.push(DiscoveredUrl {
                url: url.clone(),
                title,
                published: None,
                source: DiscoverySource::Crawl,
            });
        }

        if depth >= limits.max_depth {
            continue;
        }

        for link in extract_links(&url, &html) {
            if !same_host(&link, prefix) {
                continue;
            }
            if !matches_prefix(&link, prefix) {
                continue;
            }
            if is_probably_asset(&link) {
                continue;
            }
            let link_key = canonicalize(&link);
            if visited.contains(&link_key) {
                continue;
            }
            queue.push_back((link, depth + 1));
        }
    }

    Ok(discovered)
}

fn extract_links(base: &Url, html: &str) -> Vec<Url> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let mut links = Vec::new();
    for element in document.select(&selector) {
        let Some(href) = element.value().attr("href") else {
            continue;
        };
        let href = href.trim();
        if href.is_empty()
            || href.starts_with('#')
            || href.starts_with("mailto:")
            || href.starts_with("javascript:")
        {
            continue;
        }
        if let Ok(url) = base.join(href) {
            let mut clean = url;
            clean.set_fragment(None);
            links.push(clean);
        }
    }
    links
}

fn extract_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("title").ok()?;
    document
        .select(&selector)
        .next()
        .map(|node| node.text().collect::<String>().trim().to_owned())
        .filter(|title| !title.is_empty())
}

fn looks_like_article(url: &Url, prefix: &Url) -> bool {
    let path = url.path().trim_end_matches('/');
    let prefix_path = prefix.path().trim_end_matches('/');
    if path == prefix_path || path == "/" {
        return false;
    }
    let lower = path.to_ascii_lowercase();
    let banned = [
        "/tag/",
        "/tags/",
        "/category/",
        "/categories/",
        "/author/",
        "/authors/",
        "/page/",
        "/pages/",
        "/search",
        "/feed",
        "/rss",
        "/atom",
    ];
    if banned.iter().any(|part| lower.contains(part)) {
        return false;
    }
    // Require at least one extra path segment beyond the prefix.
    let prefix_segments = prefix_path.split('/').filter(|s| !s.is_empty()).count();
    let path_segments = path.split('/').filter(|s| !s.is_empty()).count();
    path_segments > prefix_segments
}

fn is_probably_asset(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    [
        ".css", ".js", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".ico", ".pdf",
        ".zip", ".mp4", ".mp3", ".woff", ".woff2",
    ]
    .iter()
    .any(|ext| path.ends_with(ext))
}

fn canonicalize(url: &Url) -> String {
    let mut clean = url.clone();
    clean.set_fragment(None);
    // Drop trailing slash except for root
    let path = clean.path().to_owned();
    if path.len() > 1 && path.ends_with('/') {
        clean.set_path(path.trim_end_matches('/'));
    }
    clean.as_str().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_same_document_links() {
        let html = r#"<a href="/blog/one">One</a><a href="https://other.com/x">x</a>"#;
        let base = Url::parse("https://example.com/blog/").unwrap();
        let links = extract_links(&base, html);
        assert!(links.iter().any(|u| u.as_str() == "https://example.com/blog/one"));
    }

    #[test]
    fn article_heuristic_filters_indexes() {
        let prefix = Url::parse("https://example.com/blog/").unwrap();
        assert!(!looks_like_article(
            &Url::parse("https://example.com/blog/").unwrap(),
            &prefix
        ));
        assert!(looks_like_article(
            &Url::parse("https://example.com/blog/hello-world").unwrap(),
            &prefix
        ));
        assert!(!looks_like_article(
            &Url::parse("https://example.com/blog/tag/rust").unwrap(),
            &prefix
        ));
    }
}
