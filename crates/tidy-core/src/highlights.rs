use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{Result, TidyError};
use crate::fetch::{FrontMatterHighlight, parse_frontmatter, render_document};
use crate::index::{HighlightRow, Index};
use crate::vault::Vault;

const CONTEXT_CHARS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightInput {
    pub text: String,
    pub note: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

/// Create a highlight, write SQLite + durable frontmatter.
pub fn add_highlight(
    vault: &Vault,
    index: &Index,
    article_id: i64,
    input: HighlightInput,
) -> Result<HighlightRow> {
    let text = input.text.trim().to_owned();
    if text.is_empty() {
        return Err(TidyError::Message("highlight text is required".into()));
    }

    let Some(article) = index.get_article(article_id)? else {
        return Err(TidyError::Message(format!(
            "article {article_id} not found"
        )));
    };

    let (prefix, suffix) = match (input.prefix, input.suffix) {
        (Some(prefix), Some(suffix)) => (prefix, suffix),
        _ => quote_context(&article.body, &text),
    };

    let note = input
        .note
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    let highlight = HighlightRow {
        id: new_highlight_id(article_id, &text),
        article_id,
        text,
        note,
        prefix,
        suffix,
        created_at: Utc::now().to_rfc3339(),
    };

    index.upsert_highlight(&highlight)?;
    sync_frontmatter_highlights(vault, &article.path, index, article_id)?;
    Ok(highlight)
}

pub fn update_highlight_note(
    vault: &Vault,
    index: &Index,
    highlight_id: &str,
    note: Option<String>,
) -> Result<HighlightRow> {
    let note = note
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    let Some(mut highlight) = index.get_highlight(highlight_id)? else {
        return Err(TidyError::Message(format!(
            "highlight {highlight_id} not found"
        )));
    };

    let Some(article) = index.get_article(highlight.article_id)? else {
        return Err(TidyError::Message(format!(
            "article {} not found",
            highlight.article_id
        )));
    };

    highlight.note = note;
    index.upsert_highlight(&highlight)?;
    sync_frontmatter_highlights(vault, &article.path, index, highlight.article_id)?;
    Ok(highlight)
}

pub fn delete_highlight(vault: &Vault, index: &Index, highlight_id: &str) -> Result<bool> {
    let Some(highlight) = index.get_highlight(highlight_id)? else {
        return Ok(false);
    };
    let Some(article) = index.get_article(highlight.article_id)? else {
        return Err(TidyError::Message(format!(
            "article {} not found",
            highlight.article_id
        )));
    };

    let deleted = index.delete_highlight(highlight_id)?;
    if deleted {
        sync_frontmatter_highlights(vault, &article.path, index, highlight.article_id)?;
    }
    Ok(deleted)
}

pub fn list_highlights(index: &Index, article_id: Option<i64>) -> Result<Vec<HighlightRow>> {
    index.list_highlights(article_id)
}

fn sync_frontmatter_highlights(
    vault: &Vault,
    relative_path: &str,
    index: &Index,
    article_id: i64,
) -> Result<()> {
    let path = vault.root().join(relative_path);
    if !path.exists() {
        return Ok(());
    }
    write_highlights_to_frontmatter(&path, &index.list_highlights(Some(article_id))?)
}

fn write_highlights_to_frontmatter(path: &Path, highlights: &[HighlightRow]) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let Some(mut frontmatter) = parse_frontmatter(&text)? else {
        return Ok(());
    };
    let body = strip_frontmatter_body(&text);
    frontmatter.highlights = highlights.iter().map(to_frontmatter).collect();
    let document = render_document(&frontmatter, body)?;
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, document.as_bytes())?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn to_frontmatter(row: &HighlightRow) -> FrontMatterHighlight {
    FrontMatterHighlight {
        id: row.id.clone(),
        text: row.text.clone(),
        note: row.note.clone(),
        prefix: row.prefix.clone(),
        suffix: row.suffix.clone(),
        created_at: row.created_at.clone(),
    }
}

pub fn from_frontmatter(article_id: i64, item: &FrontMatterHighlight) -> HighlightRow {
    HighlightRow {
        id: item.id.clone(),
        article_id,
        text: item.text.clone(),
        note: item.note.clone(),
        prefix: item.prefix.clone(),
        suffix: item.suffix.clone(),
        created_at: item.created_at.clone(),
    }
}

/// Derive TextQuote-style prefix/suffix from markdown body when the client omits them.
pub fn quote_context(body: &str, exact: &str) -> (String, String) {
    if let Some(index) = body.find(exact) {
        let prefix_start = index.saturating_sub(CONTEXT_CHARS);
        let suffix_end = (index + exact.len())
            .saturating_add(CONTEXT_CHARS)
            .min(body.len());
        (
            body[prefix_start..index].to_owned(),
            body[index + exact.len()..suffix_end].to_owned(),
        )
    } else {
        (String::new(), String::new())
    }
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

fn new_highlight_id(article_id: i64, text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    article_id.hash(&mut hasher);
    text.hash(&mut hasher);
    Utc::now().timestamp_nanos_opt().hash(&mut hasher);
    format!("hl{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_context_finds_surroundings() {
        let body = "alpha beta gamma delta epsilon";
        let (prefix, suffix) = quote_context(body, "gamma");
        assert!(prefix.ends_with("beta "));
        assert!(suffix.starts_with(" delta"));
    }
}
