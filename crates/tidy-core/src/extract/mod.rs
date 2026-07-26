mod markdown;
mod metadata;
mod quality;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Result, TidyError};

pub use markdown::html_to_markdown;
pub use metadata::{merge_metadata, ArticleHints};
pub use quality::{assess_quality, ExtractionQuality};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityLabel {
    Ok,
    NeedsReview,
}

impl From<ExtractionQuality> for QualityLabel {
    fn from(value: ExtractionQuality) -> Self {
        match value {
            ExtractionQuality::Ok => Self::Ok,
            ExtractionQuality::NeedsReview => Self::NeedsReview,
        }
    }
}

impl QualityLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NeedsReview => "needs_review",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedArticle {
    pub title: String,
    pub author: Option<String>,
    pub published: Option<String>,
    pub lang: Option<String>,
    pub excerpt: String,
    pub html: String,
    pub markdown: String,
    pub text: String,
    pub word_count: usize,
    pub reading_time: u32,
    pub quality: QualityLabel,
    pub canonical_url: Option<String>,
    pub site_name: Option<String>,
    pub image: Option<String>,
}

/// Extract a readable article from raw HTML.
pub fn extract_article(html: &str, page_url: &Url, hints: &ArticleHints) -> Result<ExtractedArticle> {
    let mut reader = dom_smoothie::Readability::new(html, Some(page_url.as_str()), None)
        .map_err(|error| TidyError::extract(page_url.as_str(), error.to_string()))?;

    let json_ld = reader.parse_json_ld();
    let meta = reader.get_article_metadata(json_ld);

    let article = reader
        .parse()
        .map_err(|error| TidyError::extract(page_url.as_str(), error.to_string()))?;

    let content_html = article.content.to_string();
    let text = article.text_content.to_string();
    let markdown = html_to_markdown(&content_html)
        .map_err(|error| TidyError::extract(page_url.as_str(), error.to_string()))?;

    let merged = merge_metadata(hints, &meta, &article);
    let word_count = count_words(&text);
    let quality = assess_quality(html, &text, word_count).into();
    let excerpt = merged
        .excerpt
        .unwrap_or_else(|| make_excerpt(&text, 200));

    Ok(ExtractedArticle {
        title: merged.title,
        author: merged.author,
        published: merged.published,
        lang: merged.lang.or(article.lang),
        excerpt,
        html: content_html,
        markdown,
        text,
        word_count,
        reading_time: reading_time_minutes(word_count),
        quality,
        canonical_url: merged.url.or(article.url),
        site_name: merged.site_name.or(article.site_name),
        image: merged.image.or(article.image),
    })
}

pub fn count_words(text: &str) -> usize {
    text.split_whitespace().filter(|part| !part.is_empty()).count()
}

pub fn reading_time_minutes(word_count: usize) -> u32 {
    ((word_count as f64) / 200.0).ceil().max(1.0) as u32
}

pub fn make_excerpt(text: &str, max_chars: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }
    let mut excerpt = collapsed
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    excerpt.push('…');
    excerpt
}

pub fn content_hash(markdown_body: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(markdown_body.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn render_markdown_html(markdown: &str) -> String {
    let parser = pulldown_cmark::Parser::new(markdown);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_basic_article() {
        let html = r#"
        <html lang="en"><head>
          <title>Hello World — Example Blog</title>
          <meta property="og:title" content="Hello World">
          <meta name="author" content="Ada Lovelace">
          <script type="application/ld+json">
            {"@type":"Article","headline":"Hello World","datePublished":"2026-01-02T00:00:00Z"}
          </script>
        </head><body>
          <article>
            <h1>Hello World</h1>
            <p>This is a substantial enough paragraph for readability to keep around while testing extraction quality gates and markdown conversion for Tidy.</p>
            <p>A second paragraph keeps the word count comfortably above the review threshold used by the extractor.</p>
            <pre><code class="language-rust">fn main() {}</code></pre>
          </article>
        </body></html>
        "#;
        let url = Url::parse("https://example.com/blog/hello-world").unwrap();
        let article = extract_article(html, &url, &ArticleHints::default()).unwrap();
        assert!(article.title.to_lowercase().contains("hello"));
        assert!(article.word_count > 20);
        assert!(article.markdown.contains("```") || article.markdown.contains("fn main"));
        assert_eq!(article.quality, QualityLabel::Ok);
    }
}
