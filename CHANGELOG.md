# Changelog

All notable changes to Tidy are documented here.

## [0.5.1] — 2026-07-27

### Fixed

- macOS beach-ball hang when choosing a vault (async folder dialog + deferred catch-up)

## [0.5.0] — 2026-07-27

### Added

- **M7 Polish** — first-run onboarding, last-vault reopen, shortcut help
- Per-source extraction/crawl overrides (CSS selectors, max pages)
- Review filter for `needs_review` articles
- Vault backup and markdown reindex (app + CLI)

## [0.4.0] — 2026-07-27

### Added

- **M6 Highlights** — TextQuote-anchored highlights with optional notes
- Durable frontmatter `highlights` mirrored into SQLite
- Reader selection popup + highlight list; CLI `tidy highlights`

## [0.3.0] — 2026-07-27

### Added

- **M5 Search** — FTS5 full-text search, tag sidebar filters, saved smart views
- CLI `tidy search` for headless queries against the vault index

## [0.2.1] — 2026-07-27

### Changed

- Release workflow aligned with CourseLib-style publish (auto-generated notes, three-platform matrix)
- Windows x64 NSIS installer added to GitHub Releases

## [0.2.0] — 2026-07-26

### Added

- **M4 Scheduler** — per-source refresh intervals, pause/resume, launch catch-up, run history
- CLI `tidy schedule` for due status and fetch runs
- **macOS Intel** (`x86_64`) release packages alongside Apple Silicon

### Platforms

- `Tidy_*_aarch64.dmg` — Apple Silicon
- `Tidy_*_x64.dmg` — Intel

Unsigned installs still need `xattr -cr /Applications/Tidy.app` after a browser download.

## [0.1.1] — 2026-07-26

### Fixed

- Run `svelte-kit sync` before typecheck so fresh CI checkouts (without `.svelte-kit/`) pass

## [0.1.0] — 2026-07-26

First tagged release. Milestones **M0–M3** are in place: vault + fetch engine + desktop reader.

### Added

- Tauri 2 + SvelteKit desktop shell with vault selection
- Markdown vault (Obsidian-friendly frontmatter) and rebuildable SQLite index
- Discovery pipeline: RSS/Atom/JSON feeds → sitemaps → polite prefix crawl
- Extraction pipeline: readability HTML → markdown, local images, change detection
- CLI: `tidy init`, `tidy discover`, `tidy fetch`
- Reader UI: sources sidebar, day-grouped Information Feed, typography/themes
- Article state: unread/read, starred, archived (SQLite + frontmatter)
- Live fetch progress events and keyboard navigation
- CI and release packaging for **macOS Apple Silicon** (`aarch64-apple-darwin`)

[0.2.1]: https://github.com/ririyad/tidy/releases/tag/v0.2.1
[0.2.0]: https://github.com/ririyad/tidy/releases/tag/v0.2.0
[0.1.1]: https://github.com/ririyad/tidy/releases/tag/v0.1.1
[0.1.0]: https://github.com/ririyad/tidy/releases/tag/v0.1.0
