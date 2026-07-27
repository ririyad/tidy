-- Per-source extraction / crawl overrides (JSON object).
ALTER TABLE sources ADD COLUMN overrides_json TEXT NOT NULL DEFAULT '{}';

PRAGMA user_version = 2;
