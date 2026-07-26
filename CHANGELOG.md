# Changelog

All notable changes to Tidy are documented here.

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

### Not yet

- Per-source scheduler and launch catch-up (**M4**)
- Full-text search, tags, smart views (**M5**)
- Highlights and notes (**M6**)
- Onboarding polish, Intel Mac / Windows / Linux packages, signing (**M7+**)

[0.1.0]: https://github.com/ririyad/tidy/releases/tag/v0.1.0
