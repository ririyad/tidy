use quick_xml::events::Event;
use quick_xml::Reader;
use url::Url;

use crate::error::{Result, TidyError};
use crate::http::HttpClient;

/// Discover sitemap candidates. Prefer robots.txt Sitemap: directives that look
/// related to the prefix, then common paths on the origin/prefix.
pub async fn discover_sitemaps(
    client: &HttpClient,
    prefix: &Url,
    _warnings: &mut Vec<String>,
) -> Result<Vec<Url>> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut push = |url: Url| {
        let key = url.as_str().to_owned();
        if seen.insert(key) {
            candidates.push(url);
        }
    };

    if let Ok(Some(rules)) = client.robots_for(prefix).await {
        let mut related = Vec::new();
        let mut other = Vec::new();
        for sitemap in rules.sitemaps() {
            let Ok(url) = Url::parse(sitemap).or_else(|_| prefix.join(sitemap)) else {
                continue;
            };
            if sitemap_looks_related(&url, prefix) {
                related.push(url);
            } else {
                other.push(url);
            }
        }
        // Prefer related sitemaps; keep a few others as fallback.
        for url in related.into_iter().chain(other.into_iter().take(2)) {
            push(url);
        }
    }

    let origin = origin_url(prefix)?;
    // Only probe common paths when robots didn't already give us something useful.
    if candidates.is_empty() {
        for path in ["sitemap.xml", "sitemap_index.xml", "sitemap-index.xml"] {
            if let Ok(url) = origin.join(path) {
                let key = url.as_str().to_owned();
                if seen.insert(key) {
                    candidates.push(url);
                }
            }
            if let Ok(url) = prefix.join(path) {
                let key = url.as_str().to_owned();
                if seen.insert(key) {
                    candidates.push(url);
                }
            }
        }
    } else if let Ok(url) = origin.join("sitemap.xml") {
        let key = url.as_str().to_owned();
        if seen.insert(key) {
            candidates.push(url);
        }
    }

    Ok(candidates)
}

pub async fn parse_sitemap_urls(
    client: &HttpClient,
    sitemap_url: &Url,
    depth: u32,
    max_depth: u32,
    warnings: &mut Vec<String>,
) -> Result<Vec<Url>> {
    let response = client.get_bytes(sitemap_url).await?;
    let parsed = parse_sitemap_body(sitemap_url, &response.body)?;

    match parsed {
        SitemapBody::UrlSet(urls) => Ok(urls),
        SitemapBody::Index(children) => {
            if depth >= max_depth {
                warnings.push(format!(
                    "sitemap index depth cap reached at {sitemap_url}"
                ));
                return Ok(Vec::new());
            }
            let mut all = Vec::new();
            for child in children {
                match Box::pin(parse_sitemap_urls(
                    client, &child, depth + 1, max_depth, warnings,
                ))
                .await
                {
                    Ok(mut urls) => all.append(&mut urls),
                    Err(error) => warnings.push(format!("child sitemap {child}: {error}")),
                }
            }
            Ok(all)
        }
    }
}

enum SitemapBody {
    UrlSet(Vec<Url>),
    Index(Vec<Url>),
}

fn parse_sitemap_body(sitemap_url: &Url, body: &[u8]) -> Result<SitemapBody> {
    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_loc = false;
    let mut urls = Vec::new();
    let mut is_index = false;
    let mut is_urlset = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "sitemapindex" => is_index = true,
                    "urlset" => is_urlset = true,
                    "loc" => in_loc = true,
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if in_loc => {
                let text = t
                    .unescape()
                    .map_err(|error| TidyError::Sitemap {
                        url: sitemap_url.to_string(),
                        message: error.to_string(),
                    })?
                    .into_owned();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if let Ok(url) = Url::parse(trimmed).or_else(|_| sitemap_url.join(trimmed)) {
                        urls.push(url);
                    }
                }
                in_loc = false;
            }
            Ok(Event::End(e)) => {
                if local_name(e.name().as_ref()) == "loc" {
                    in_loc = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(TidyError::Sitemap {
                    url: sitemap_url.to_string(),
                    message: error.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    if is_index {
        Ok(SitemapBody::Index(urls))
    } else if is_urlset || !urls.is_empty() {
        Ok(SitemapBody::UrlSet(urls))
    } else {
        Err(TidyError::Sitemap {
            url: sitemap_url.to_string(),
            message: "unrecognized sitemap document".into(),
        })
    }
}

fn local_name(name: &[u8]) -> String {
    let full = String::from_utf8_lossy(name);
    full.rsplit('}').next().unwrap_or(&full).to_ascii_lowercase()
}

fn origin_url(url: &Url) -> Result<Url> {
    let mut origin = url.clone();
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    Ok(origin)
}

fn sitemap_looks_related(sitemap: &Url, prefix: &Url) -> bool {
    if sitemap.host_str() != prefix.host_str() {
        return false;
    }
    let prefix_path = prefix.path().trim_matches('/');
    if prefix_path.is_empty() {
        return true;
    }
    let first = prefix_path.split('/').next().unwrap_or("");
    let sitemap_path = sitemap.path().to_ascii_lowercase();
    sitemap_path.contains(&first.to_ascii_lowercase())
        || sitemap_path.contains("post")
        || sitemap_path.contains("blog")
        || sitemap_path.contains("article")
        || sitemap_path.contains("news")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_urlset() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/blog/a</loc></url>
          <url><loc>https://example.com/blog/b</loc></url>
        </urlset>"#;
        let base = Url::parse("https://example.com/sitemap.xml").unwrap();
        match parse_sitemap_body(&base, xml).unwrap() {
            SitemapBody::UrlSet(urls) => {
                assert_eq!(urls.len(), 2);
                assert_eq!(urls[0].as_str(), "https://example.com/blog/a");
            }
            SitemapBody::Index(_) => panic!("expected urlset"),
        }
    }

    #[test]
    fn parses_sitemap_index() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <sitemap><loc>https://example.com/post-sitemap.xml</loc></sitemap>
        </sitemapindex>"#;
        let base = Url::parse("https://example.com/sitemap_index.xml").unwrap();
        match parse_sitemap_body(&base, xml).unwrap() {
            SitemapBody::Index(urls) => {
                assert_eq!(urls.len(), 1);
            }
            SitemapBody::UrlSet(_) => panic!("expected index"),
        }
    }
}
