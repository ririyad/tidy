use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

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

#[derive(Debug, Clone)]
pub struct SourceRow {
    pub id: i64,
    pub url_prefix: String,
    pub title: String,
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
                interval_minutes = excluded.interval_minutes,
                backfill_policy = excluded.backfill_policy,
                enabled = excluded.enabled
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

        let row = self.conn.query_row(
            "SELECT id, url_prefix, title FROM sources WHERE url_prefix = ?1",
            params![source.url_prefix],
            |row| {
                Ok(SourceRow {
                    id: row.get(0)?,
                    url_prefix: row.get(1)?,
                    title: row.get(2)?,
                })
            },
        )?;
        Ok(row)
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
