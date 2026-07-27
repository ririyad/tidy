mod discover;
mod error;
mod extract;
mod fetch;
mod highlights;
mod http;
mod index;
mod maintenance;
mod overrides;
mod robots;
mod scheduler;
mod search;
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
    extract_article_with_overrides, render_markdown_html,
};
pub use fetch::{
    ArticleFrontMatter, FetchOptions, FetchProgress, FetchReport, FetchStatus, fetch,
    fetch_with_progress, source_slug,
};
pub use highlights::{
    HighlightInput, add_highlight, delete_highlight, list_highlights, update_highlight_note,
};
pub use http::{FetchResponse, HttpClient, HttpClientConfig, USER_AGENT};
pub use index::{
    ArticleDetail, ArticleFilter, ArticleListItem, ArticleRecord, FetchRunRow, HighlightRow, Index,
    SourceRecord, SourceRow,
};
pub use maintenance::{BackupReport, ReindexReport, backup_vault, reindex_vault};
pub use overrides::SourceOverrides;
pub use robots::{RobotsRules, parse_robots};
pub use scheduler::{
    DEFAULT_INTERVAL_MINUTES, ScheduleStatus, list_due_sources, list_run_history, schedule_status,
    source_is_due,
};
pub use search::{
    ArticleQuery, SmartViewFilter, SmartViewQuery, SmartViewRow, TagCount, parse_smart_view_query,
    prepare_fts_query,
};
pub use settings::{ReaderSettings, load_reader_settings, save_reader_settings};
pub use state::{ArticleStatePatch, apply_article_state};
pub use vault::{Vault, VaultError, VaultSummary, ensure_schema};

pub const APP_NAME: &str = "Tidy";
pub const VAULT_SCHEMA_VERSION: u32 = 2;
