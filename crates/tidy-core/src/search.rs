use serde::{Deserialize, Serialize};

use crate::error::{Result, TidyError};
use crate::index::ArticleFilter;

/// Saved smart-view rule (stored as JSON in `smart_views.query_json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SmartViewQuery {
    #[serde(default)]
    pub filter: SmartViewFilter,
    pub tag: Option<String>,
    pub query: Option<String>,
    pub source_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SmartViewFilter {
    #[default]
    Inbox,
    Unread,
    Starred,
    Archived,
    All,
    Review,
}

impl SmartViewFilter {
    pub fn to_article_filter(self) -> ArticleFilter {
        match self {
            Self::Inbox => ArticleFilter::Inbox,
            Self::Unread => ArticleFilter::Unread,
            Self::Starred => ArticleFilter::Starred,
            Self::Archived => ArticleFilter::Archived,
            Self::All => ArticleFilter::All,
            Self::Review => ArticleFilter::Review,
        }
    }

    pub fn from_article_filter(filter: ArticleFilter) -> Self {
        match filter {
            ArticleFilter::Inbox => Self::Inbox,
            ArticleFilter::Unread => Self::Unread,
            ArticleFilter::Starred => Self::Starred,
            ArticleFilter::Archived => Self::Archived,
            ArticleFilter::All => Self::All,
            ArticleFilter::Review => Self::Review,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmartViewRow {
    pub id: String,
    pub name: String,
    pub query: SmartViewQuery,
    pub position: i64,
}

#[derive(Debug, Clone)]
pub struct ArticleQuery {
    pub filter: ArticleFilter,
    pub source_id: Option<i64>,
    pub tag: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
}

impl Default for ArticleQuery {
    fn default() -> Self {
        Self {
            filter: ArticleFilter::Inbox,
            source_id: None,
            tag: None,
            search: None,
            limit: None,
        }
    }
}

/// Turn user input into a safe FTS5 MATCH expression (OR across tokens).
pub fn prepare_fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split_whitespace()
        .map(|token| token.trim_matches('"').trim())
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"", token.replace('"', "")))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" OR "))
    }
}

pub fn parse_smart_view_query(json: &str) -> Result<SmartViewQuery> {
    serde_json::from_str(json)
        .map_err(|error| TidyError::Message(format!("invalid smart view: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_query_or_tokens() {
        assert_eq!(
            prepare_fts_query("hello world").as_deref(),
            Some("\"hello\" OR \"world\"")
        );
        assert_eq!(prepare_fts_query("   ").as_deref(), None);
    }

    #[test]
    fn smart_view_roundtrip() {
        let query = SmartViewQuery {
            filter: SmartViewFilter::Starred,
            tag: Some("source/example".into()),
            query: Some("rust".into()),
            source_id: None,
        };
        let json = serde_json::to_string(&query).unwrap();
        let parsed = parse_smart_view_query(&json).unwrap();
        assert_eq!(parsed, query);
    }
}
