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

Desktop builds for **macOS (Apple Silicon + Intel)** and **Windows (x64)** are published on
[GitHub Releases](https://github.com/ririyad/tidy/releases). Each release attaches `.dmg` /
`.app.tar.gz` (macOS) and an NSIS `.exe` (Windows).

| Platform | Asset |
| --- | --- |
| Apple Silicon | `Tidy_*_aarch64.dmg` |
| Intel Mac | `Tidy_*_x64.dmg` |
| Windows | `Tidy_*_x64-setup.exe` |

Builds are unsigned. macOS browser downloads may show *“Tidy is damaged”* (Gatekeeper
quarantine). Install without quarantine:

```bash
curl -fsSL https://raw.githubusercontent.com/ririyad/tidy/main/scripts/install-macos.sh | bash
```

Or after a manual DMG install: `xattr -cr /Applications/Tidy.app && open /Applications/Tidy.app`

Push a `v*` tag to trigger the [Release](https://github.com/ririyad/tidy/actions/workflows/release.yml)
workflow (same layout as [CourseLib releases](https://github.com/ririyad/courselib/releases)).

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
