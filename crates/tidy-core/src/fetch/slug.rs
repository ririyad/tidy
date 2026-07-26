use sha2::{Digest, Sha256};
use url::Url;

/// Build a filesystem-safe source directory slug from a URL prefix.
pub fn source_slug(prefix: &Url) -> String {
    let host = prefix.host_str().unwrap_or("source").to_ascii_lowercase();
    let path = prefix
        .path()
        .trim_matches('/')
        .replace('/', "-")
        .to_ascii_lowercase();

    let raw = if path.is_empty() {
        host
    } else {
        format!("{host}-{path}")
    };

    sanitize_slug(&raw)
}

/// `YYYY-MM-DD-<slug>` stem for an article markdown file (without extension).
pub fn article_stem(title: &str, url: &Url, published: Option<&str>) -> String {
    let date = published
        .and_then(parse_date_prefix)
        .unwrap_or_else(|| "1970-01-01".into());
    let mut slug = slug::slugify(title);
    if slug.is_empty() {
        slug = slug::slugify(url.path()).replace('/', "-");
    }
    if slug.is_empty() {
        slug = short_hash(url.as_str());
    }
    slug = truncate(&slug, 80);
    format!("{date}-{slug}")
}

pub fn short_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    format!("{:x}", digest)[..8].to_owned()
}

fn sanitize_slug(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == '-' || ch == '_';
        if ok {
            out.push(ch.to_ascii_lowercase());
            prev_dash = ch == '-';
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "source".into()
    } else {
        truncate(&trimmed, 80)
    }
}

fn truncate(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        input.to_owned()
    } else {
        input.chars().take(max).collect()
    }
}

fn parse_date_prefix(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.len() >= 10 {
        let candidate = &trimmed[..10];
        if candidate.as_bytes()[4] == b'-' && candidate.as_bytes()[7] == b'-' {
            return Some(candidate.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_source_and_article_slugs() {
        let prefix = Url::parse("https://Example.com/blog/posts").unwrap();
        assert_eq!(source_slug(&prefix), "example-com-blog-posts");

        let url = Url::parse("https://example.com/blog/hello-world").unwrap();
        let stem = article_stem("Hello World!", &url, Some("2026-03-14T09:00:00Z"));
        assert_eq!(stem, "2026-03-14-hello-world");
    }
}
