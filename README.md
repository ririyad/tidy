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

# Enumerate posts under a URL prefix
cargo run -p tidy-cli -- discover https://example.com/blog --limit 20

# Fetch, extract, and write Obsidian-friendly markdown
cargo run -p tidy-cli -- fetch https://example.com/blog --vault ~/Tidy --limit 5
```

In the app: choose a vault → **Add** a source (recent or full backfill) → browse the
Information Feed → read with typography controls. Keyboard: `j`/`k` move, `o`
open, `u` read/unread, `s` star, `e` archive, `r` refresh, `g f` inbox, `/` add source.

## Workspace layout

```
crates/tidy-core/   # discovery, extraction, vault writer, SQLite index, state APIs
crates/tidy-cli/    # headless harness for the engine
src-tauri/          # Tauri commands, vault session, fetch progress events
src/                # Svelte reader UI (sidebar, feed, reader)
docs/SPEC.md        # vault + frontmatter contracts
```

## Milestones

| Milestone | Status | What landed |
| --- | --- | --- |
| **M0** Scaffold | Done | Tauri + SvelteKit shell, vault init, SQLite schema, contracts |
| **M1** Discovery | Done | Polite HTTP client, robots/feeds/sitemaps, crawl fallback, `tidy discover` |
| **M2** Extraction | Done | Readability → markdown, images, atomic vault writes, change detection, `tidy fetch` |
| **M3** Reader UI | Done | Source CRUD, live refresh progress, day-grouped feed, reader themes, keyboard nav |
| **M4** Scheduler | Next | Per-source intervals, launch catch-up, run history |
| **M5** Search | Planned | FTS5 search, tags, smart views |
| **M6** Highlights | Planned | Anchored highlights + notes |
| **M7** Polish | Planned | Onboarding, overrides, packaging |

### Current: M3

The desktop UI is a usable reader: add sources with backfill choice, refresh with
progress events, browse a day-grouped Information Feed, and read articles with
serif/sans, size, measure, line-height, and paper/ink/sepia themes. Read, star,
and archive flush to both SQLite and markdown frontmatter.

See `docs/SPEC.md` for the vault and frontmatter contract.
