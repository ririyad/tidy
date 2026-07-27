use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::{Result, TidyError};
use crate::vault::Vault;

use super::slug::short_hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleFrontMatter {
    pub title: String,
    pub url: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    pub fetched: String,
    pub tags: Vec<String>,
    pub word_count: u32,
    pub reading_time: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub excerpt: String,
    pub state: String,
    pub starred: bool,
    pub archived: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<FrontMatterHighlight>,
    pub content_hash: String,
    pub revision: u32,
    pub extraction: ExtractionInfo,
}

/// Durable highlight stored in article frontmatter (TextQuote-style anchors).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontMatterHighlight {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub suffix: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionInfo {
    pub engine: String,
    pub quality: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcomeStatus {
    Created,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct WriteOutcome {
    pub relative_path: String,
    #[allow(dead_code)]
    pub absolute_path: PathBuf,
    pub status: WriteOutcomeStatus,
}

pub fn write_article_file(
    vault: &Vault,
    source_slug: &str,
    stem: &str,
    frontmatter: &ArticleFrontMatter,
    markdown_body: &str,
) -> Result<WriteOutcome> {
    let relative = format!("Sources/{source_slug}/{stem}.md");
    let mut absolute = vault.root().join(&relative);

    if absolute.exists() {
        // Collision with a different URL — suffix a short hash.
        if let Ok(Some(existing)) = read_existing_frontmatter(&absolute) {
            if existing.url != frontmatter.url {
                let suffix = short_hash(&frontmatter.url);
                let relative = format!("Sources/{source_slug}/{stem}-{suffix}.md");
                absolute = vault.root().join(&relative);
                return write_at(vault, relative, absolute, frontmatter, markdown_body);
            }
        }
    }

    write_at(vault, relative, absolute, frontmatter, markdown_body)
}

fn write_at(
    vault: &Vault,
    relative: String,
    absolute: PathBuf,
    frontmatter: &ArticleFrontMatter,
    markdown_body: &str,
) -> Result<WriteOutcome> {
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)?;
    }

    let document = render_document(frontmatter, markdown_body)?;
    let existed = absolute.exists();
    if existed {
        let current = fs::read_to_string(&absolute)?;
        if current == document {
            return Ok(WriteOutcome {
                relative_path: relative,
                absolute_path: absolute,
                status: WriteOutcomeStatus::Unchanged,
            });
        }
    }

    let tmp = absolute.with_extension("md.tmp");
    fs::write(&tmp, document.as_bytes())?;
    fs::rename(&tmp, &absolute)?;

    // Ensure attachments root exists for Obsidian browsing comfort.
    let _ = vault.attachments_dir();

    Ok(WriteOutcome {
        relative_path: relative,
        absolute_path: absolute,
        status: if existed {
            WriteOutcomeStatus::Updated
        } else {
            WriteOutcomeStatus::Created
        },
    })
}

pub fn render_document(frontmatter: &ArticleFrontMatter, markdown_body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)
        .map_err(|error| TidyError::Message(format!("frontmatter serialize error: {error}")))?;
    // serde_yaml includes a trailing newline; gray-matter style wants --- fences.
    let yaml = yaml.trim_start_matches("---\n").trim_end().to_owned();
    Ok(format!(
        "---\n{yaml}\n---\n\n{}\n",
        markdown_body.trim_end()
    ))
}

pub fn read_existing_frontmatter(path: &Path) -> Result<Option<ArticleFrontMatter>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    parse_frontmatter(&text)
}

pub fn parse_frontmatter(text: &str) -> Result<Option<ArticleFrontMatter>> {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return Ok(None);
    }
    let rest = &trimmed[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| TidyError::Message("unterminated frontmatter".into()))?;
    let yaml = &rest[..end];
    let matter: ArticleFrontMatter = serde_yaml::from_str(yaml)
        .map_err(|error| TidyError::Message(format!("frontmatter parse error: {error}")))?;
    Ok(Some(matter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
    use tempfile::tempdir;

    #[test]
    fn writes_and_parses_round_trip() {
        let dir = tempdir().unwrap();
        let summary = Vault::initialize(dir.path()).unwrap();
        let vault = Vault::open(summary.path).unwrap();
        let fm = ArticleFrontMatter {
            title: "Hello".into(),
            url: "https://example.com/a".into(),
            source: "example-com".into(),
            author: Some("Ada".into()),
            published: Some("2026-01-02T00:00:00Z".into()),
            fetched: "2026-07-26T00:00:00Z".into(),
            tags: vec!["source/example-com".into()],
            word_count: 10,
            reading_time: 1,
            lang: Some("en".into()),
            excerpt: "Hello".into(),
            state: "unread".into(),
            starred: false,
            archived: false,
            highlights: vec![],
            content_hash: "sha256:abc".into(),
            revision: 1,
            extraction: ExtractionInfo {
                engine: "dom_smoothie".into(),
                quality: "ok".into(),
            },
        };
        let outcome =
            write_article_file(&vault, "example-com", "2026-01-02-hello", &fm, "Body").unwrap();
        assert!(matches!(outcome.status, WriteOutcomeStatus::Created));
        let parsed = read_existing_frontmatter(&outcome.absolute_path)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.title, "Hello");
        assert_eq!(parsed.revision, 1);
        assert!(parsed.highlights.is_empty());
        let with_hl = ArticleFrontMatter {
            highlights: vec![FrontMatterHighlight {
                id: "hl1".into(),
                text: "Body".into(),
                note: Some("note".into()),
                prefix: "".into(),
                suffix: "".into(),
                created_at: "2026-07-27T00:00:00Z".into(),
            }],
            ..parsed
        };
        write_article_file(&vault, "example-com", "2026-01-02-hello", &with_hl, "Body").unwrap();
        let again = read_existing_frontmatter(&outcome.absolute_path)
            .unwrap()
            .unwrap();
        assert_eq!(again.highlights.len(), 1);
        assert_eq!(again.highlights[0].note.as_deref(), Some("note"));
    }
}
