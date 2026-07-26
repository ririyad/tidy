# Tidy

Local-first desktop reader and fetch engine. Point Tidy at a blog URL prefix,
and it gathers every post underneath into an Obsidian-compatible Markdown vault
with a calm reading interface.

## Stack

- **Desktop shell:** Tauri 2
- **UI:** SvelteKit (static SPA) + Svelte 5 + Tailwind
- **Engine:** Rust workspace (`tidy-core`, `tidy-cli`, `src-tauri`)
- **Storage:** Markdown + YAML frontmatter (source of truth) + SQLite index

## Development

```bash
npm install
npm run tauri dev
```

CLI:

```bash
# Create a vault
cargo run -p tidy-cli -- init ~/Tidy

# Enumerate posts under a prefix
cargo run -p tidy-cli -- discover https://example.com/blog --limit 20

# Fetch, extract, and write Obsidian-friendly markdown
cargo run -p tidy-cli -- fetch https://example.com/blog --vault ~/Tidy --limit 5
```

## Workspace layout

```
crates/tidy-core/   # discovery, extraction, vault writer, SQLite index
crates/tidy-cli/    # headless harness for the engine
src-tauri/          # thin Tauri commands + window
src/                # Svelte UI
docs/SPEC.md        # vault + frontmatter contracts
```

## Milestones

| Milestone | Status | What landed |
| --- | --- | --- |
| **M0** Scaffold | Done | Tauri + SvelteKit shell, vault init, SQLite schema, contracts |
| **M1** Discovery | Done | Polite HTTP client, robots/feeds/sitemaps, crawl fallback, `tidy discover` |
| **M2** Extraction | Done | Readability → markdown, images, atomic vault writes, change detection, `tidy fetch` |
| **M3** Reader UI | Next | Information Feed timeline, reader chrome, keyboard nav, read/star/archive |
| **M4** Scheduler | Planned | Per-source intervals, launch catch-up, run history |
| **M5** Search | Planned | FTS5 search, tags, smart views |
| **M6** Highlights | Planned | Anchored highlights + notes |
| **M7** Polish | Planned | Onboarding, overrides, packaging |

### Current: M2

`tidy fetch` discovers posts under a URL prefix, extracts readable content with
`dom_smoothie`, converts to Markdown (`htmd`), downloads images into
`attachments/`, writes Obsidian-compatible files with YAML frontmatter, and
upserts the SQLite index. Re-running is idempotent via `content_hash`.

See `docs/SPEC.md` for the vault and frontmatter contract.
