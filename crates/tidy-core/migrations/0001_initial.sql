PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS sources (
    id INTEGER PRIMARY KEY,
    url_prefix TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    feed_url TEXT,
    discovery_mode TEXT NOT NULL DEFAULT 'auto',
    interval_minutes INTEGER NOT NULL DEFAULT 360,
    backfill_policy TEXT NOT NULL DEFAULT 'ask',
    etag TEXT,
    last_modified TEXT,
    last_fetch_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    url TEXT NOT NULL UNIQUE,
    canonical_url TEXT,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    author TEXT,
    published_at TEXT,
    fetched_at TEXT NOT NULL,
    word_count INTEGER NOT NULL DEFAULT 0,
    excerpt TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    rendered_html TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'unread' CHECK (state IN ('unread', 'read')),
    starred INTEGER NOT NULL DEFAULT 0 CHECK (starred IN (0, 1)),
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    progress REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1),
    revision INTEGER NOT NULL DEFAULT 1,
    quality TEXT NOT NULL DEFAULT 'ok' CHECK (quality IN ('ok', 'needs_review')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS articles_source_published
    ON articles(source_id, published_at DESC);
CREATE INDEX IF NOT EXISTS articles_state_published
    ON articles(state, archived, published_at DESC);

CREATE TABLE IF NOT EXISTS article_tags (
    article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (article_id, tag)
);

CREATE INDEX IF NOT EXISTS article_tags_tag ON article_tags(tag);

CREATE VIRTUAL TABLE IF NOT EXISTS articles_fts USING fts5(
    title,
    excerpt,
    body,
    content='articles',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS articles_fts_insert AFTER INSERT ON articles BEGIN
    INSERT INTO articles_fts(rowid, title, excerpt, body)
    VALUES (new.id, new.title, new.excerpt, new.body);
END;

CREATE TRIGGER IF NOT EXISTS articles_fts_delete AFTER DELETE ON articles BEGIN
    INSERT INTO articles_fts(articles_fts, rowid, title, excerpt, body)
    VALUES ('delete', old.id, old.title, old.excerpt, old.body);
END;

CREATE TRIGGER IF NOT EXISTS articles_fts_update AFTER UPDATE ON articles BEGIN
    INSERT INTO articles_fts(articles_fts, rowid, title, excerpt, body)
    VALUES ('delete', old.id, old.title, old.excerpt, old.body);
    INSERT INTO articles_fts(rowid, title, excerpt, body)
    VALUES (new.id, new.title, new.excerpt, new.body);
END;

CREATE TABLE IF NOT EXISTS crawl_queue (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    depth INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    UNIQUE (source_id, url)
);

CREATE TABLE IF NOT EXISTS fetch_runs (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    discovered INTEGER NOT NULL DEFAULT 0,
    added INTEGER NOT NULL DEFAULT 0,
    updated INTEGER NOT NULL DEFAULT 0,
    skipped INTEGER NOT NULL DEFAULT 0,
    failed INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS highlights (
    id TEXT PRIMARY KEY,
    article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    note TEXT,
    prefix TEXT NOT NULL DEFAULT '',
    suffix TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS highlights_article ON highlights(article_id);

CREATE TABLE IF NOT EXISTS smart_views (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    query_json TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

PRAGMA user_version = 1;
