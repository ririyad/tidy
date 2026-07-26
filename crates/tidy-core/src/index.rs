use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct SourceRecord {
    pub url_prefix: String,
    pub title: String,
    pub feed_url: Option<String>,
    pub discovery_mode: String,
    pub interval_minutes: i64,
    pub backfill_policy: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceRow {
    pub id: i64,
    pub url_prefix: String,
    pub title: String,
    pub feed_url: Option<String>,
    pub backfill_policy: String,
    pub interval_minutes: i64,
    pub last_fetch_at: Option<String>,
    pub enabled: bool,
    pub article_count: i64,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchRunRow {
    pub id: i64,
    pub source_id: i64,
    pub source_title: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub discovered: i64,
    pub added: i64,
    pub updated: i64,
    pub skipped: i64,
    pub failed: i64,
}

#[derive(Debug, Clone)]
pub struct ArticleRecord {
    pub source_id: i64,
    pub url: String,
    pub canonical_url: Option<String>,
    pub path: String,
    pub title: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: String,
    pub word_count: i64,
    pub excerpt: String,
    pub body: String,
    pub rendered_html: String,
    pub content_hash: String,
    pub state: String,
    pub starred: bool,
    pub archived: bool,
    pub revision: i64,
    pub quality: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArticleRow {
    pub id: i64,
    pub path: String,
    pub content_hash: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArticleFilter {
    Inbox,
    Unread,
    Starred,
    Archived,
    All,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArticleListItem {
    pub id: i64,
    pub source_id: i64,
    pub source_title: String,
    pub url: String,
    pub path: String,
    pub title: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: String,
    pub word_count: i64,
    pub reading_time: u32,
    pub excerpt: String,
    pub state: String,
    pub starred: bool,
    pub archived: bool,
    pub quality: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArticleDetail {
    pub id: i64,
    pub source_id: i64,
    pub source_title: String,
    pub url: String,
    pub path: String,
    pub title: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: String,
    pub word_count: i64,
    pub reading_time: u32,
    pub excerpt: String,
    pub body: String,
    pub rendered_html: String,
    pub state: String,
    pub starred: bool,
    pub archived: bool,
    pub progress: f64,
    pub quality: String,
    pub revision: i64,
}

pub struct Index {
    conn: Connection,
}

impl Index {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(Self { conn })
    }

    pub fn upsert_source(&self, source: &SourceRecord) -> Result<SourceRow> {
        self.conn.execute(
            r#"
            INSERT INTO sources (
                url_prefix, title, feed_url, discovery_mode,
                interval_minutes, backfill_policy, enabled
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(url_prefix) DO UPDATE SET
                title = excluded.title,
                feed_url = COALESCE(excluded.feed_url, sources.feed_url),
                discovery_mode = excluded.discovery_mode,
                backfill_policy = excluded.backfill_policy
            "#,
            params![
                source.url_prefix,
                source.title,
                source.feed_url,
                source.discovery_mode,
                source.interval_minutes,
                source.backfill_policy,
                source.enabled as i64
            ],
        )?;

        let id: i64 = self.conn.query_row(
            "SELECT id FROM sources WHERE url_prefix = ?1",
            params![source.url_prefix],
            |row| row.get(0),
        )?;
        self.get_source(id)?
            .ok_or_else(|| crate::error::TidyError::Message("source missing after upsert".into()))
    }

    pub fn list_sources(&self) -> Result<Vec<SourceRow>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                s.id, s.url_prefix, s.title, s.feed_url, s.backfill_policy,
                s.interval_minutes, s.last_fetch_at, s.enabled,
                COALESCE((SELECT count(*) FROM articles a WHERE a.source_id = s.id), 0),
                COALESCE((
                    SELECT count(*) FROM articles a
                    WHERE a.source_id = s.id AND a.state = 'unread' AND a.archived = 0
                ), 0)
            FROM sources s
            ORDER BY s.title COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map([], map_source_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_source(&self, id: i64) -> Result<Option<SourceRow>> {
        let row = self
            .conn
            .query_row(
                r#"
                SELECT
                    s.id, s.url_prefix, s.title, s.feed_url, s.backfill_policy,
                    s.interval_minutes, s.last_fetch_at, s.enabled,
                    COALESCE((SELECT count(*) FROM articles a WHERE a.source_id = s.id), 0),
                    COALESCE((
                        SELECT count(*) FROM articles a
                        WHERE a.source_id = s.id AND a.state = 'unread' AND a.archived = 0
                    ), 0)
                FROM sources s
                WHERE s.id = ?1
                "#,
                params![id],
                map_source_row,
            )
            .optional()?;
        Ok(row)
    }

    pub fn update_source_schedule(
        &self,
        source_id: i64,
        interval_minutes: Option<i64>,
        enabled: Option<bool>,
    ) -> Result<Option<SourceRow>> {
        if interval_minutes.is_none() && enabled.is_none() {
            return self.get_source(source_id);
        }
        if let Some(minutes) = interval_minutes {
            let minutes = minutes.max(1);
            self.conn.execute(
                "UPDATE sources SET interval_minutes = ?1 WHERE id = ?2",
                params![minutes, source_id],
            )?;
        }
        if let Some(enabled) = enabled {
            self.conn.execute(
                "UPDATE sources SET enabled = ?1 WHERE id = ?2",
                params![enabled as i64, source_id],
            )?;
        }
        self.get_source(source_id)
    }

    pub fn list_fetch_runs(&self, source_id: Option<i64>, limit: i64) -> Result<Vec<FetchRunRow>> {
        let limit = limit.clamp(1, 200);
        let mut out = Vec::new();
        if let Some(source_id) = source_id {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT
                    r.id, r.source_id, s.title, r.started_at, r.finished_at, r.status,
                    r.discovered, r.added, r.updated, r.skipped, r.failed
                FROM fetch_runs r
                JOIN sources s ON s.id = r.source_id
                WHERE r.source_id = ?1
                ORDER BY r.started_at DESC
                LIMIT ?2
                "#,
            )?;
            let rows = stmt.query_map(params![source_id, limit], map_fetch_run_row)?;
            for row in rows {
                out.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT
                    r.id, r.source_id, s.title, r.started_at, r.finished_at, r.status,
                    r.discovered, r.added, r.updated, r.skipped, r.failed
                FROM fetch_runs r
                JOIN sources s ON s.id = r.source_id
                ORDER BY r.started_at DESC
                LIMIT ?1
                "#,
            )?;
            let rows = stmt.query_map(params![limit], map_fetch_run_row)?;
            for row in rows {
                out.push(row?);
            }
        }
        Ok(out)
    }

    pub fn delete_source(&self, id: i64) -> Result<bool> {
        let changed = self
            .conn
            .execute("DELETE FROM sources WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn set_source_feed_url(&self, source_id: i64, feed_url: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sources SET feed_url = ?1 WHERE id = ?2",
            params![feed_url, source_id],
        )?;
        Ok(())
    }

    pub fn touch_source_fetch(&self, source_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE sources SET last_fetch_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), source_id],
        )?;
        Ok(())
    }

    pub fn start_fetch_run(&self, source_id: i64) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO fetch_runs (source_id, started_at, status)
            VALUES (?1, ?2, 'running')
            "#,
            params![source_id, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn finish_fetch_run(
        &self,
        run_id: i64,
        status: &str,
        discovered: i64,
        added: i64,
        updated: i64,
        skipped: i64,
        failed: i64,
    ) -> Result<()> {
        self.conn.execute(
            r#"
            UPDATE fetch_runs SET
                finished_at = ?1,
                status = ?2,
                discovered = ?3,
                added = ?4,
                updated = ?5,
                skipped = ?6,
                failed = ?7
            WHERE id = ?8
            "#,
            params![
                Utc::now().to_rfc3339(),
                status,
                discovered,
                added,
                updated,
                skipped,
                failed,
                run_id
            ],
        )?;
        Ok(())
    }

    pub fn find_article_by_url(&self, url: &str) -> Result<Option<ArticleRow>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, path, content_hash, revision FROM articles WHERE url = ?1",
                params![url],
                |row| {
                    Ok(ArticleRow {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        content_hash: row.get(2)?,
                        revision: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn list_articles(
        &self,
        filter: ArticleFilter,
        source_id: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<ArticleListItem>> {
        let mut sql = String::from(
            r#"
            SELECT
                a.id, a.source_id, s.title, a.url, a.path, a.title, a.author,
                a.published_at, a.fetched_at, a.word_count, a.excerpt,
                a.state, a.starred, a.archived, a.quality
            FROM articles a
            JOIN sources s ON s.id = a.source_id
            WHERE 1 = 1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        match filter {
            ArticleFilter::Inbox | ArticleFilter::Unread => {
                sql.push_str(" AND a.state = 'unread' AND a.archived = 0");
            }
            ArticleFilter::Starred => {
                sql.push_str(" AND a.starred = 1 AND a.archived = 0");
            }
            ArticleFilter::Archived => {
                sql.push_str(" AND a.archived = 1");
            }
            ArticleFilter::All => {
                sql.push_str(" AND a.archived = 0");
            }
        }

        if let Some(source_id) = source_id {
            sql.push_str(" AND a.source_id = ?");
            params_vec.push(Box::new(source_id));
        }

        sql.push_str(" ORDER BY COALESCE(a.published_at, a.fetched_at) DESC, a.id DESC");

        if let Some(limit) = limit {
            sql.push_str(" LIMIT ?");
            params_vec.push(Box::new(limit));
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let word_count: i64 = row.get(9)?;
            Ok(ArticleListItem {
                id: row.get(0)?,
                source_id: row.get(1)?,
                source_title: row.get(2)?,
                url: row.get(3)?,
                path: row.get(4)?,
                title: row.get(5)?,
                author: row.get(6)?,
                published_at: row.get(7)?,
                fetched_at: row.get(8)?,
                word_count,
                reading_time: reading_time(word_count),
                excerpt: row.get(10)?,
                state: row.get(11)?,
                starred: row.get::<_, i64>(12)? != 0,
                archived: row.get::<_, i64>(13)? != 0,
                quality: row.get(14)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_article(&self, id: i64) -> Result<Option<ArticleDetail>> {
        let row = self
            .conn
            .query_row(
                r#"
                SELECT
                    a.id, a.source_id, s.title, a.url, a.path, a.title, a.author,
                    a.published_at, a.fetched_at, a.word_count, a.excerpt,
                    a.body, a.rendered_html, a.state, a.starred, a.archived,
                    a.progress, a.quality, a.revision
                FROM articles a
                JOIN sources s ON s.id = a.source_id
                WHERE a.id = ?1
                "#,
                params![id],
                |row| {
                    let word_count: i64 = row.get(9)?;
                    Ok(ArticleDetail {
                        id: row.get(0)?,
                        source_id: row.get(1)?,
                        source_title: row.get(2)?,
                        url: row.get(3)?,
                        path: row.get(4)?,
                        title: row.get(5)?,
                        author: row.get(6)?,
                        published_at: row.get(7)?,
                        fetched_at: row.get(8)?,
                        word_count,
                        reading_time: reading_time(word_count),
                        excerpt: row.get(10)?,
                        body: row.get(11)?,
                        rendered_html: row.get(12)?,
                        state: row.get(13)?,
                        starred: row.get::<_, i64>(14)? != 0,
                        archived: row.get::<_, i64>(15)? != 0,
                        progress: row.get(16)?,
                        quality: row.get(17)?,
                        revision: row.get(18)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn update_article_flags(
        &self,
        id: i64,
        state: Option<&str>,
        starred: Option<bool>,
        archived: Option<bool>,
    ) -> Result<bool> {
        let mut sets = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(state) = state {
            sets.push("state = ?");
            params_vec.push(Box::new(state.to_owned()));
        }
        if let Some(starred) = starred {
            sets.push("starred = ?");
            params_vec.push(Box::new(starred as i64));
        }
        if let Some(archived) = archived {
            sets.push("archived = ?");
            params_vec.push(Box::new(archived as i64));
        }
        if sets.is_empty() {
            return Ok(false);
        }

        sets.push("updated_at = ?");
        params_vec.push(Box::new(Utc::now().to_rfc3339()));
        params_vec.push(Box::new(id));

        let sql = format!("UPDATE articles SET {} WHERE id = ?", sets.join(", "));
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let changed = self.conn.execute(&sql, params_refs.as_slice())?;
        Ok(changed > 0)
    }

    pub fn update_article_progress(&self, id: i64, progress: f64) -> Result<()> {
        let progress = progress.clamp(0.0, 1.0);
        self.conn.execute(
            "UPDATE articles SET progress = ?1, updated_at = ?2 WHERE id = ?3",
            params![progress, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn upsert_article(&self, article: &ArticleRecord) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO articles (
                source_id, url, canonical_url, path, title, author,
                published_at, fetched_at, word_count, excerpt, body, rendered_html,
                content_hash, state, starred, archived, revision, quality, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19
            )
            ON CONFLICT(url) DO UPDATE SET
                canonical_url = excluded.canonical_url,
                path = excluded.path,
                title = excluded.title,
                author = excluded.author,
                published_at = excluded.published_at,
                fetched_at = excluded.fetched_at,
                word_count = excluded.word_count,
                excerpt = excluded.excerpt,
                body = excluded.body,
                rendered_html = excluded.rendered_html,
                content_hash = excluded.content_hash,
                state = excluded.state,
                starred = excluded.starred,
                archived = excluded.archived,
                revision = excluded.revision,
                quality = excluded.quality,
                updated_at = excluded.updated_at
            "#,
            params![
                article.source_id,
                article.url,
                article.canonical_url,
                article.path,
                article.title,
                article.author,
                article.published_at,
                article.fetched_at,
                article.word_count,
                article.excerpt,
                article.body,
                article.rendered_html,
                article.content_hash,
                article.state,
                article.starred as i64,
                article.archived as i64,
                article.revision,
                article.quality,
                Utc::now().to_rfc3339(),
            ],
        )?;

        let article_id: i64 = self.conn.query_row(
            "SELECT id FROM articles WHERE url = ?1",
            params![article.url],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "DELETE FROM article_tags WHERE article_id = ?1",
            params![article_id],
        )?;
        for tag in &article.tags {
            self.conn.execute(
                "INSERT OR IGNORE INTO article_tags (article_id, tag) VALUES (?1, ?2)",
                params![article_id, tag],
            )?;
        }

        Ok(article_id)
    }
}

fn reading_time(word_count: i64) -> u32 {
    ((word_count as f64) / 200.0).ceil().max(1.0) as u32
}

fn map_source_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceRow> {
    Ok(SourceRow {
        id: row.get(0)?,
        url_prefix: row.get(1)?,
        title: row.get(2)?,
        feed_url: row.get(3)?,
        backfill_policy: row.get(4)?,
        interval_minutes: row.get(5)?,
        last_fetch_at: row.get(6)?,
        enabled: row.get::<_, i64>(7)? != 0,
        article_count: row.get(8)?,
        unread_count: row.get(9)?,
    })
}

fn map_fetch_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FetchRunRow> {
    Ok(FetchRunRow {
        id: row.get(0)?,
        source_id: row.get(1)?,
        source_title: row.get(2)?,
        started_at: row.get(3)?,
        finished_at: row.get(4)?,
        status: row.get(5)?,
        discovered: row.get(6)?,
        added: row.get(7)?,
        updated: row.get(8)?,
        skipped: row.get(9)?,
        failed: row.get(10)?,
    })
}
