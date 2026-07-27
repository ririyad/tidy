# Tidy vault and article contracts

This document is the source of truth for vault layout, markdown frontmatter,
and the rebuildable SQLite index. Implementation lives in `tidy-core`.

## Vault layout

```
<vault-root>/
  .tidy/
    index.db          # SQLite index (derived; rebuildable)
    config.toml       # app + reader settings
    sources.toml      # source definitions (human-editable)
    cache/            # HTTP / robots cache
    logs/
  Sources/
    <source-slug>/
      YYYY-MM-DD-<slug>.md
  attachments/
    <source-slug>/<article-stem>/<asset>
```

### Filenames

- Pattern: `YYYY-MM-DD-<slug>.md`
- Strip characters unsafe for Obsidian / cross-platform filesystems:
  `[] # ^ | : / \ *` and Windows reserved names
- Cap length; on collision append a short URL hash

## Frontmatter contract

```yaml
---
title: "The Post Title"
url: https://example.com/articles/the-post-title
source: stratechery-com-articles
author: Ben Thompson
published: 2026-03-14T09:00:00Z
fetched: 2026-07-25T18:22:03Z
tags: [source/stratechery, topic/business]
word_count: 1840
reading_time: 9
lang: en
excerpt: "First 200 characters..."
state: unread
starred: false
archived: false
content_hash: sha256:9f2c...
revision: 1
extraction: { engine: dom_smoothie, quality: ok }
---
```

### Field rules

| Field | Durable? | Notes |
| --- | --- | --- |
| `state` | yes | `unread` \| `read` |
| `starred` | yes | boolean |
| `archived` | yes | boolean |
| `tags` | yes | auto from source + feed metadata in v1 |
| `highlights` | yes | TextQuote anchors + optional note; mirrored into SQLite |
| `content_hash` | yes | body change detection |
| `revision` | yes | bumped when body changes on re-fetch |
| scroll progress | no | SQLite only — never rewrite the file for scroll |

Markdown body follows the YAML document. Images use relative links into
`attachments/`.

Highlight frontmatter entries:

```yaml
highlights:
  - id: hl1a2b3c
    text: "exact quote from the article"
    note: "optional reader note"
    prefix: "…context before…"
    suffix: "…context after…"
    created_at: 2026-07-27T15:30:00Z
```

Anchors use TextQuote-style `text` + `prefix`/`suffix` so they survive
modest body reflows after re-fetch.

## SQLite schema (v1)

Defined in `crates/tidy-core/migrations/0001_initial.sql`.

Tables:

- `sources` — registered URL prefixes and fetch settings
- `articles` — indexed article metadata + cached `rendered_html`
- `article_tags` — tag join table
- `articles_fts` — FTS5 over title, excerpt, body
- `crawl_queue` — pending discovery URLs
- `fetch_runs` — per-source run history
- `highlights` — query mirror of frontmatter highlights
- `smart_views` — saved rule-based views

Smart view `query_json` shape:

```json
{
  "filter": "inbox",
  "tag": "source/example",
  "query": "optional fts terms",
  "source_id": null
}
```

Search uses the `articles_fts` FTS5 table (`title`, `excerpt`, `body`). User queries are
tokenized and matched with OR semantics. Combine search with inbox/starred filters and tags.

`PRAGMA user_version = 2` tracks the applied schema revision (migration `0002_overrides`
adds `sources.overrides_json`).

## Scheduler (M4)

- Each source has `interval_minutes` (default 360) and `enabled`
- A source is **due** when enabled and either never fetched or
  `now >= last_fetch_at + interval_minutes`
- Opening the app (and a soft in-app tick) refreshes due sources
- `fetch_runs` stores per-run counts for history UI / `tidy schedule --runs`

Manual **Refresh** always fetches the selected source (or all enabled sources).

## Search (M5)

- Feed header search box queries `articles_fts` (title, excerpt, body)
- Tags from fetch appear in the sidebar; click to filter
- **Smart views** persist filter + tag + source + search as JSON in `smart_views`
- CLI: `tidy search --vault PATH [--filter inbox] [--tag TAG] [QUERY...]`

## Highlights (M6)

- Select text in the reader to save a TextQuote-anchored highlight (+ optional note)
- Highlights are durable in frontmatter and mirrored into the `highlights` table
- Re-fetch preserves highlights from existing frontmatter
- CLI: `tidy highlights --vault PATH [--article ID]`

## Polish (M7)

- First-run welcome when a vault is created; last vault reopens on launch
- Per-source `overrides_json`: `content_selector`, `title_selector`,
  `pagination_link_selector`, `max_pages`
- Review filter lists `quality = needs_review`
- `tidy backup` / `tidy reindex` (and matching app actions) for recovery
- Shortcut help via `?`

## Initialization

Creating a vault:

1. Ensure `.tidy/`, `Sources/`, `attachments/`, `.tidy/cache/`, `.tidy/logs/`
2. Write default `config.toml` and `sources.toml` if missing
3. Open `.tidy/index.db` and apply migrations

CLI: `tidy init [PATH]`  
App: folder picker → `Vault::initialize`
