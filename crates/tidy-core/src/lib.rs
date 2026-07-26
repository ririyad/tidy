mod discover;
mod error;
mod extract;
mod fetch;
mod http;
mod index;
mod robots;
mod settings;
mod state;
mod vault;

pub use discover::{
    CrawlLimits, DiscoverOptions, DiscoverReport, DiscoveredUrl, DiscoverySource, discover,
    parse_prefix,
};
pub use error::TidyError;
pub use extract::{
    ArticleHints, ExtractedArticle, QualityLabel, content_hash, extract_article,
    render_markdown_html,
};
pub use fetch::{
    ArticleFrontMatter, FetchOptions, FetchProgress, FetchReport, FetchStatus, fetch,
    fetch_with_progress, source_slug,
};
pub use http::{FetchResponse, HttpClient, HttpClientConfig, USER_AGENT};
pub use index::{
    ArticleDetail, ArticleFilter, ArticleListItem, ArticleRecord, Index, SourceRecord, SourceRow,
};
pub use robots::{RobotsRules, parse_robots};
pub use settings::{ReaderSettings, load_reader_settings, save_reader_settings};
pub use state::{ArticleStatePatch, apply_article_state};
pub use vault::{Vault, VaultError, VaultSummary};

pub const APP_NAME: &str = "Tidy";
pub const VAULT_SCHEMA_VERSION: u32 = 1;
