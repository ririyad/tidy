use thiserror::Error;
use url::ParseError as UrlParseError;

use crate::vault::VaultError;

#[derive(Debug, Error)]
pub enum TidyError {
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error("invalid URL: {0}")]
    Url(#[from] UrlParseError),
    #[error("HTTP error fetching {url}: {message}")]
    Http { url: String, message: String },
    #[error("robots.txt forbids fetching {0}")]
    RobotsForbidden(String),
    #[error("failed to parse feed from {url}: {message}")]
    Feed { url: String, message: String },
    #[error("failed to parse sitemap from {url}: {message}")]
    Sitemap { url: String, message: String },
    #[error("failed to extract article from {url}: {message}")]
    Extract { url: String, message: String },
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

impl TidyError {
    pub fn http(url: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Http {
            url: url.into(),
            message: message.into(),
        }
    }

    pub fn extract(url: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Extract {
            url: url.into(),
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, TidyError>;
