use serde::{Deserialize, Serialize};

/// Per-source crawl/extraction overrides (stored as JSON on `sources.overrides_json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SourceOverrides {
    /// CSS selector for the main article body (skips readability when it matches).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_selector: Option<String>,
    /// CSS selector for the page title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_selector: Option<String>,
    /// CSS selector for “next page” / archive pagination links during crawl.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination_link_selector: Option<String>,
    /// Cap crawl pages for this source (overrides global page_cap when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pages: Option<usize>,
}

impl SourceOverrides {
    pub fn is_empty(&self) -> bool {
        self.content_selector.is_none()
            && self.title_selector.is_none()
            && self.pagination_link_selector.is_none()
            && self.max_pages.is_none()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn from_json(raw: &str) -> Self {
        serde_json::from_str(raw.trim()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_overrides() {
        let overrides = SourceOverrides {
            content_selector: Some("article.post".into()),
            title_selector: Some("h1".into()),
            pagination_link_selector: Some("a.next".into()),
            max_pages: Some(40),
        };
        let parsed = SourceOverrides::from_json(&overrides.to_json());
        assert_eq!(parsed, overrides);
    }
}
