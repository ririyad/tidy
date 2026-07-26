# Tidy

Local-first desktop reader and fetch engine. Point Tidy at a blog URL prefix,
and it gathers every post underneath into an Obsidian-compatible Markdown vault
with a calm reading interface.

## Stack

- **Desktop shell:** Tauri 2
- **UI:** SvelteKit (static SPA) + Svelte 5 + Tailwind
- **Engine:** Rust workspace (`tidy-core`, `tidy-cli`, `src-tauri`)
- **Storage:** Markdown + YAML frontmatter (source of truth) + SQLite index

## Releases

Distribution is **unsigned** macOS Apple Silicon `.dmg` / `.app` artifacts on
[GitHub Releases](https://github.com/ririyad/tidy/releases) (built by the `v*` tag workflow).
See [CHANGELOG.md](CHANGELOG.md).

CI on `main` / PRs runs fmt, clippy, tests, frontend check, and an aarch64 Tauri build
on `macos-latest`.

### First open after downloading the DMG

Chrome/Safari mark GitHub downloads with a quarantine flag. For an unsigned app, macOS
then shows *“Tidy is damaged and can’t be opened”* — that is Gatekeeper, not a bad file.

After dragging **Tidy** into **Applications**:

```bash
xattr -cr /Applications/Tidy.app && open /Applications/Tidy.app
```

That is the supported install path for unsigned GitHub releases. (Notarized builds would
skip this step; we intentionally ship unsigned.)

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

# Show schedule / due status and recent fetch runs
cargo run -p tidy-cli -- schedule --vault ~/Tidy
cargo run -p tidy-cli -- schedule --vault ~/Tidy --runs
```

In the app: choose a vault → **Add** a source (recent or full backfill, refresh interval)
→ browse the Information Feed → read with typography controls. Due sources catch up on
launch. Keyboard: `j`/`k` move, `o` open, `u` read/unread, `s` star, `e` archive, `r`
refresh, `g f` inbox, `/` add source.

## Workspace layout

```
crates/tidy-core/   # discovery, extraction, vault writer, SQLite index, scheduler, state APIs
crates/tidy-cli/    # headless harness for the engine
src-tauri/          # Tauri commands, vault session, fetch progress events
src/                # Svelte reader UI (sidebar, feed, reader)
docs/SPEC.md        # vault + frontmatter + scheduler contracts
```

## Milestones

| Milestone | Status | What landed |
| --- | --- | --- |
| **M0** Scaffold | Done | Tauri + SvelteKit shell, vault init, SQLite schema, contracts |
| **M1** Discovery | Done | Polite HTTP client, robots/feeds/sitemaps, crawl fallback, `tidy discover` |
| **M2** Extraction | Done | Readability → markdown, images, atomic vault writes, change detection, `tidy fetch` |
| **M3** Reader UI | Done | Source CRUD, live refresh progress, day-grouped feed, reader themes, keyboard nav |
| **M4** Scheduler | Done | Per-source intervals, launch catch-up, run history |
| **M5** Search | Next | FTS5 search, tags, smart views |
| **M6** Highlights | Planned | Anchored highlights + notes |
| **M7** Polish | Planned | Onboarding, overrides, packaging |

### Current: M4

Enabled sources refresh on their interval. Opening a vault catches up anything due;
while the app stays open a one-minute tick does the same. Each source shows interval,
last fetch, pause/resume, and recent run history. CLI: `tidy schedule --vault PATH`.

See `docs/SPEC.md` for the vault and frontmatter contract.
