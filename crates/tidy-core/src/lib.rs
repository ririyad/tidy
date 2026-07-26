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
    discover, parse_prefix, CrawlLimits, DiscoverOptions, DiscoverReport, DiscoveredUrl,
    DiscoverySource,
};
pub use error::TidyError;
pub use extract::{
    content_hash, extract_article, render_markdown_html, ArticleHints, ExtractedArticle,
    QualityLabel,
};
pub use fetch::{
    fetch, fetch_with_progress, source_slug, ArticleFrontMatter, FetchOptions, FetchProgress,
    FetchReport, FetchStatus,
};
pub use http::{FetchResponse, HttpClient, HttpClientConfig, USER_AGENT};
pub use index::{
    ArticleDetail, ArticleFilter, ArticleListItem, ArticleRecord, Index, SourceRecord, SourceRow,
};
pub use robots::{parse_robots, RobotsRules};
pub use settings::{load_reader_settings, save_reader_settings, ReaderSettings};
pub use state::{apply_article_state, ArticleStatePatch};
pub use vault::{Vault, VaultError, VaultSummary};

pub const APP_NAME: &str = "Tidy";
pub const VAULT_SCHEMA_VERSION: u32 = 1;
