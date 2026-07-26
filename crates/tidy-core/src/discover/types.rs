use std::path::PathBuf;

use serde::Serialize;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoverySource {
    Feed,
    Sitemap,
    Crawl,
    None,
}

impl DiscoverySource {
    pub fn rank(self) -> u8 {
        match self {
            Self::Feed => 3,
            Self::Sitemap => 2,
            Self::Crawl => 1,
            Self::None => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredUrl {
    pub url: Url,
    pub title: Option<String>,
    pub published: Option<String>,
    pub source: DiscoverySource,
}

#[derive(Debug, Clone)]
pub struct CrawlLimits {
    pub max_depth: u32,
    pub page_cap: usize,
}

impl Default for CrawlLimits {
    fn default() -> Self {
        Self {
            max_depth: 2,
            page_cap: 200,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoverOptions {
    pub url_prefix: Url,
    pub limit: Option<usize>,
    pub cache_dir: Option<PathBuf>,
    pub limits: CrawlLimits,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoverReport {
    pub prefix: Url,
    pub urls: Vec<DiscoveredUrl>,
    pub feed_url: Option<Url>,
    pub sitemap_urls: Vec<Url>,
    pub primary_source: DiscoverySource,
    pub warnings: Vec<String>,
}
