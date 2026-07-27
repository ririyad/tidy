use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::error::Result;
use crate::index::{FetchRunRow, Index, SourceRow};

pub const DEFAULT_INTERVAL_MINUTES: i64 = 360;

/// Whether a source is due for a scheduled fetch.
pub fn source_is_due(source: &SourceRow, now: DateTime<Utc>) -> bool {
    if !source.enabled {
        return false;
    }
    let interval = source.interval_minutes.max(1);
    match &source.last_fetch_at {
        None => true,
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(parsed) => {
                let last = parsed.with_timezone(&Utc);
                now >= last + Duration::minutes(interval)
            }
            Err(_) => true,
        },
    }
}

/// Enabled sources whose interval has elapsed (or never fetched).
pub fn list_due_sources(index: &Index) -> Result<Vec<SourceRow>> {
    let now = Utc::now();
    let sources = index.list_sources()?;
    Ok(sources
        .into_iter()
        .filter(|source| source_is_due(source, now))
        .collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduleStatus {
    pub source_id: i64,
    pub title: String,
    pub interval_minutes: i64,
    pub last_fetch_at: Option<String>,
    pub enabled: bool,
    pub due: bool,
    pub next_fetch_at: Option<String>,
}

pub fn schedule_status(index: &Index) -> Result<Vec<ScheduleStatus>> {
    let now = Utc::now();
    let sources = index.list_sources()?;
    Ok(sources
        .into_iter()
        .map(|source| {
            let due = source_is_due(&source, now);
            let next_fetch_at = next_fetch_at(&source);
            ScheduleStatus {
                source_id: source.id,
                title: source.title,
                interval_minutes: source.interval_minutes,
                last_fetch_at: source.last_fetch_at,
                enabled: source.enabled,
                due,
                next_fetch_at,
            }
        })
        .collect())
}

fn next_fetch_at(source: &SourceRow) -> Option<String> {
    if !source.enabled {
        return None;
    }
    let interval = source.interval_minutes.max(1);
    let Some(raw) = &source.last_fetch_at else {
        return Some(Utc::now().to_rfc3339());
    };
    let parsed = DateTime::parse_from_rfc3339(raw).ok()?;
    let next = parsed.with_timezone(&Utc) + Duration::minutes(interval);
    Some(next.to_rfc3339())
}

/// Recent fetch runs, optionally filtered by source.
pub fn list_run_history(
    index: &Index,
    source_id: Option<i64>,
    limit: i64,
) -> Result<Vec<FetchRunRow>> {
    index.list_fetch_runs(source_id, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(last: Option<&str>, interval: i64, enabled: bool) -> SourceRow {
        SourceRow {
            id: 1,
            url_prefix: "https://example.com/".into(),
            title: "Example".into(),
            feed_url: None,
            backfill_policy: "recent".into(),
            interval_minutes: interval,
            last_fetch_at: last.map(str::to_owned),
            enabled,
            article_count: 0,
            unread_count: 0,
            overrides: Default::default(),
        }
    }

    #[test]
    fn never_fetched_is_due() {
        let now = Utc::now();
        assert!(source_is_due(&sample(None, 60, true), now));
    }

    #[test]
    fn disabled_is_never_due() {
        let now = Utc::now();
        assert!(!source_is_due(&sample(None, 60, false), now));
    }

    #[test]
    fn recent_fetch_not_due() {
        let now = Utc::now();
        let last = (now - Duration::minutes(10)).to_rfc3339();
        assert!(!source_is_due(&sample(Some(&last), 60, true), now));
    }

    #[test]
    fn elapsed_interval_is_due() {
        let now = Utc::now();
        let last = (now - Duration::minutes(90)).to_rfc3339();
        assert!(source_is_due(&sample(Some(&last), 60, true), now));
    }
}
