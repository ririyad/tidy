use std::fs;
use std::path::Path;

use crate::error::{Result, TidyError};
use crate::fetch::{parse_frontmatter, render_document};
use crate::index::{ArticleDetail, Index};
use crate::vault::Vault;

#[derive(Debug, Clone, Default)]
pub struct ArticleStatePatch {
    pub state: Option<String>,
    pub starred: Option<bool>,
    pub archived: Option<bool>,
}

/// Update reading flags in SQLite and mirror durable fields into frontmatter.
pub fn apply_article_state(
    vault: &Vault,
    index: &Index,
    article_id: i64,
    patch: ArticleStatePatch,
) -> Result<ArticleDetail> {
    let Some(article) = index.get_article(article_id)? else {
        return Err(TidyError::Message(format!(
            "article {article_id} not found"
        )));
    };

    index.update_article_flags(
        article_id,
        patch.state.as_deref(),
        patch.starred,
        patch.archived,
    )?;

    let path = vault.root().join(&article.path);
    if path.exists() {
        patch_frontmatter_flags(&path, patch.state.as_deref(), patch.starred, patch.archived)?;
    }

    index
        .get_article(article_id)?
        .ok_or_else(|| TidyError::Message("article missing after update".into()))
}

fn patch_frontmatter_flags(
    path: &Path,
    state: Option<&str>,
    starred: Option<bool>,
    archived: Option<bool>,
) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let Some(mut frontmatter) = parse_frontmatter(&text)? else {
        return Ok(());
    };

    let body = strip_frontmatter_body(&text);
    if let Some(state) = state {
        frontmatter.state = state.to_owned();
    }
    if let Some(starred) = starred {
        frontmatter.starred = starred;
    }
    if let Some(archived) = archived {
        frontmatter.archived = archived;
    }

    let document = render_document(&frontmatter, body)?;
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, document.as_bytes())?;
    fs::rename(&tmp, path)?;
    Ok(())
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
