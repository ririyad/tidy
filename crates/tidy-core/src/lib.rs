mod discover;
mod error;
mod http;
mod robots;
mod vault;

pub use discover::{
    discover, parse_prefix, CrawlLimits, DiscoverOptions, DiscoverReport, DiscoveredUrl,
    DiscoverySource,
};
pub use error::TidyError;
pub use http::{FetchResponse, HttpClient, HttpClientConfig, USER_AGENT};
pub use robots::{parse_robots, RobotsRules};
pub use vault::{Vault, VaultError, VaultSummary};

pub const APP_NAME: &str = "Tidy";
pub const VAULT_SCHEMA_VERSION: u32 = 1;
