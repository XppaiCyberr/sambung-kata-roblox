# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Sambung Kata Overlay — a desktop always-on-top helper for Indonesian word games ("sambung kata"). Built with **Tauri 2 + SvelteKit + TypeScript + Rust**. Users type a prefix and the app shows all Indonesian words starting with that prefix, with session-based removal tracking.

## Commands

```bash
npm install                  # install frontend deps
npm run tauri dev            # dev mode (starts Vite + Rust backend)
npm run tauri build          # production build (installer in src-tauri/target/release/bundle/)
npm run check                # svelte-check type checking
npm run check:watch          # type checking in watch mode
```

Rust-only iteration (from `src-tauri/`):
```bash
cargo check                  # type check Rust code
cargo clippy                 # lint Rust code
cargo build                  # build Rust backend only
```

## Architecture

**Two-process Tauri 2 app:** a Rust backend (`src-tauri/`) and a SvelteKit SPA frontend (`src/`).

### Rust Backend (`src-tauri/src/lib.rs`)
- Single file containing all backend logic — no module split.
- **WordIndex**: prefix-search index over two embedded Indonesian wordlists (`include_bytes!` from `src-tauri/resources/`). Uses a bucket map keyed by 1–3 character prefixes for fast lookup. Built once via `OnceLock` on first access.
- **Tauri commands** exposed to frontend: `catalog_info`, `search_words`, `hide_overlay`, `quit_app`.
- **Global shortcut** (`Space`) registered in `setup()` to toggle overlay visibility.
- The window is configured as decorationless, always-on-top, skip-taskbar.

### Frontend (`src/routes/+page.svelte`)
- Single-page app, entire UI in one Svelte 5 component using runes (`$state`, `$derived`, `$effect`).
- SSR disabled (`+layout.ts` exports `ssr = false`) since Tauri has no Node server.
- Calls Rust commands via `invoke()` from `@tauri-apps/api/core`.
- Listens for `overlay://focus-search` event to refocus the search input when the overlay is toggled visible.
- Session state (removed words, undo history) is in-memory only — not persisted.
- All styles are in a `<style>` block at the bottom of `+page.svelte` (no external CSS files).

### Tauri Config (`src-tauri/tauri.conf.json`)
- Capabilities split across `capabilities/default.json` (core + opener) and `capabilities/desktop.json` (global-shortcut).
- Window starts at position (0, 0), 780×460, decorationless.

## Key Conventions

- Rust structs use `#[serde(rename_all = "camelCase")]` so field names arrive as camelCase in the frontend.
- Frontend types (`CatalogInfo`, `SearchResponse`, etc.) are defined inline in `+page.svelte`, matching the Rust response shapes.
- Word normalization strips non-alphabetic chars (except hyphens) and lowercases — applied identically on both search queries and index entries.
