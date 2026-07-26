mod cache;
mod rate_limit;

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use reqwest::{Client, StatusCode, header};
use tokio::sync::Mutex;
use url::Url;

use crate::error::{Result, TidyError};

pub use cache::HttpCache;
pub use rate_limit::HostRateLimiter;

pub const USER_AGENT: &str =
    "Tidy/0.1 (+https://github.com/ririyad/tidy; polite local-first reader)";

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub cache_dir: Option<PathBuf>,
    pub per_host_interval: Duration,
    pub per_host_concurrency: usize,
    pub global_concurrency: usize,
    pub request_timeout: Duration,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            per_host_interval: Duration::from_secs(1),
            per_host_concurrency: 2,
            global_concurrency: 6,
            request_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub url: Url,
    pub status: StatusCode,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub from_cache: bool,
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    cache: Option<HttpCache>,
    limiter: HostRateLimiter,
    robots_cache: Arc<Mutex<HashMap<String, Option<crate::robots::RobotsRules>>>>,
}

impl HttpClient {
    pub fn new(config: HttpClientConfig) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(config.request_timeout)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|error| TidyError::Message(format!("failed to build HTTP client: {error}")))?;

        let cache = config
            .cache_dir
            .as_ref()
            .map(|dir| HttpCache::open(dir))
            .transpose()?;

        Ok(Self {
            client,
            cache,
            limiter: HostRateLimiter::new(
                config.per_host_interval,
                config.per_host_concurrency,
                config.global_concurrency,
            ),
            robots_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn get_text(&self, url: &Url) -> Result<String> {
        let response = self.fetch(url).await?;
        String::from_utf8(response.body).map_err(|error| {
            TidyError::http(url.as_str(), format!("response was not UTF-8: {error}"))
        })
    }

    pub async fn get_bytes(&self, url: &Url) -> Result<FetchResponse> {
        self.fetch(url).await
    }

    /// Fetch a URL, consulting robots.txt for the host first.
    pub async fn fetch(&self, url: &Url) -> Result<FetchResponse> {
        if !self.is_allowed(url).await? {
            return Err(TidyError::RobotsForbidden(url.to_string()));
        }
        self.fetch_unchecked(url).await
    }

    /// Fetch without consulting robots.txt (used for robots.txt itself).
    pub async fn fetch_unchecked(&self, url: &Url) -> Result<FetchResponse> {
        let _permit = self.limiter.acquire(url).await;

        let cached = if let Some(cache) = &self.cache {
            cache.load(url)?
        } else {
            None
        };

        let mut request = self.client.get(url.clone());
        if let Some(entry) = &cached {
            if let Some(etag) = &entry.etag {
                request = request.header(header::IF_NONE_MATCH, etag);
            }
            if let Some(last_modified) = &entry.last_modified {
                request = request.header(header::IF_MODIFIED_SINCE, last_modified);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|error| TidyError::http(url.as_str(), error.to_string()))?;

        if response.status() == StatusCode::NOT_MODIFIED {
            if let Some(entry) = cached {
                return Ok(FetchResponse {
                    url: url.clone(),
                    status: StatusCode::NOT_MODIFIED,
                    body: entry.body,
                    content_type: entry.content_type,
                    etag: entry.etag,
                    last_modified: entry.last_modified,
                    from_cache: true,
                });
            }
        }

        let status = response.status();
        if !status.is_success() {
            return Err(TidyError::http(
                url.as_str(),
                format!("unexpected status {status}"),
            ));
        }

        let etag = header_string(response.headers(), header::ETAG);
        let last_modified = header_string(response.headers(), header::LAST_MODIFIED);
        let content_type = header_string(response.headers(), header::CONTENT_TYPE);
        let final_url = response.url().clone();
        let body = response
            .bytes()
            .await
            .map_err(|error| TidyError::http(url.as_str(), error.to_string()))?
            .to_vec();

        if let Some(cache) = &self.cache {
            cache.store(
                &final_url,
                &body,
                content_type.as_deref(),
                etag.as_deref(),
                last_modified.as_deref(),
            )?;
        }

        Ok(FetchResponse {
            url: final_url,
            status,
            body,
            content_type,
            etag,
            last_modified,
            from_cache: false,
        })
    }

    pub async fn is_allowed(&self, url: &Url) -> Result<bool> {
        let Some(rules) = self.robots_for(url).await? else {
            return Ok(true);
        };
        Ok(rules.allowed(url))
    }

    pub async fn robots_for(&self, url: &Url) -> Result<Option<crate::robots::RobotsRules>> {
        let origin = origin_key(url);
        {
            let guard = self.robots_cache.lock().await;
            if let Some(cached) = guard.get(&origin) {
                return Ok(cached.clone());
            }
        }

        let robots_url = texting_robots::get_robots_url(url.as_str()).map_err(|error| {
            TidyError::Message(format!("invalid robots URL for {url}: {error}"))
        })?;
        let robots_url = Url::parse(&robots_url)?;

        let rules = match self.fetch_unchecked(&robots_url).await {
            Ok(response) => Some(crate::robots::parse_robots(&response.body)?),
            Err(TidyError::Http { .. }) => None,
            Err(error) => return Err(error),
        };

        let mut guard = self.robots_cache.lock().await;
        guard.insert(origin, rules.clone());
        Ok(rules)
    }
}

fn header_string(headers: &reqwest::header::HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn origin_key(url: &Url) -> String {
    format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default())
}
