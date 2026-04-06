# XppaiCyber | SambungKata VIP

Desktop overlay helper for Indonesian *sambung kata*, built with **Tauri 2 + SvelteKit + TypeScript + Rust**.

## What It Does

- Always-on-top overlay window with minimalist UI
- Prefix search across a merged Indonesian word list (~100k words)
- Remove used words during a session, undo with Ctrl+Z
- Toggle overlay visibility with `Space` global shortcut

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space | Toggle overlay (global) |
| Arrow Up/Down | Navigate results |
| PgUp/PgDn | Jump 8 results |
| Home/End | Jump to first/last |
| Enter/Delete | Remove selected word |
| Ctrl+Z | Undo last removal |
| Escape | Hide overlay |

## Run

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```
