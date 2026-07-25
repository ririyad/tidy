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

CLI vault tools:

```bash
cargo run -p tidy-cli -- init ~/Tidy
```

## Workspace layout

```
crates/tidy-core/   # vault, migrations, (later) fetch/extract/index
crates/tidy-cli/    # headless harness for the engine
src-tauri/          # thin Tauri commands + window
src/                # Svelte UI
docs/SPEC.md        # vault + frontmatter contracts
```

## Status

Milestone **M0** — scaffold and contracts. Fetching, reader, and scheduling
follow in later milestones. See `docs/SPEC.md`.
