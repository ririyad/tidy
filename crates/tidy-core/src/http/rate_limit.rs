use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use url::Url;

#[derive(Clone)]
pub struct HostRateLimiter {
    per_host_interval: Duration,
    per_host: Arc<Mutex<HashMap<String, HostState>>>,
    global: Arc<Semaphore>,
    per_host_concurrency: usize,
}

struct HostState {
    semaphore: Arc<Semaphore>,
    next_allowed: Instant,
}

pub struct RatePermit {
    _host: OwnedSemaphorePermit,
    _global: OwnedSemaphorePermit,
}

impl HostRateLimiter {
    pub fn new(
        per_host_interval: Duration,
        per_host_concurrency: usize,
        global_concurrency: usize,
    ) -> Self {
        Self {
            per_host_interval,
            per_host: Arc::new(Mutex::new(HashMap::new())),
            global: Arc::new(Semaphore::new(global_concurrency.max(1))),
            per_host_concurrency: per_host_concurrency.max(1),
        }
    }

    pub async fn acquire(&self, url: &Url) -> RatePermit {
        let host = url.host_str().unwrap_or("unknown").to_owned();

        let host_semaphore = {
            let mut map = self.per_host.lock().await;
            let state = map.entry(host.clone()).or_insert_with(|| HostState {
                semaphore: Arc::new(Semaphore::new(self.per_host_concurrency)),
                next_allowed: Instant::now(),
            });
            state.semaphore.clone()
        };

        let host_permit = host_semaphore
            .acquire_owned()
            .await
            .expect("host semaphore closed");
        let global_permit = self
            .global
            .clone()
            .acquire_owned()
            .await
            .expect("global semaphore closed");

        {
            let mut map = self.per_host.lock().await;
            if let Some(state) = map.get_mut(&host) {
                let now = Instant::now();
                if state.next_allowed > now {
                    let wait = state.next_allowed - now;
                    drop(map);
                    tokio::time::sleep(wait).await;
                    let mut map = self.per_host.lock().await;
                    if let Some(state) = map.get_mut(&host) {
                        state.next_allowed = Instant::now() + self.per_host_interval;
                    }
                } else {
                    state.next_allowed = now + self.per_host_interval;
                }
            }
        }

        RatePermit {
            _host: host_permit,
            _global: global_permit,
        }
    }
}
