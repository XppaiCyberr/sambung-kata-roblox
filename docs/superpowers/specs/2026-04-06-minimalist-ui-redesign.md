# Minimalist UI Redesign

## Goal

Strip the overlay UI down to its core loop: type prefix, scan matches, remove used words. Remove all panels, tabs, sidebars, and chrome that don't serve this loop.

## What Changes

### Frontend only

The Rust backend (`src-tauri/src/lib.rs`) is unchanged. All 4 Tauri commands (`catalog_info`, `search_words`, `hide_overlay`, `quit_app`) remain. The change is entirely in `src/routes/+page.svelte`.

### Window config

Update `src-tauri/tauri.conf.json`:
- Width: 780 → 420
- Height: 460 → 400
- minWidth: 700 → 320
- minHeight: 380 → 300

## Layout

Single vertical flow, no tabs, no sidebar:

```
┌──────────────────────────────────────┐
│ ░░░░░░░ SambungKata VIP ░░░░  [×]   │  title bar (28px, draggable)
├──────────────────────────────────────┤
│  [prefix input_____________________] │  autofocused, full width
│  23 matches                          │  subtle count (or flash feedback)
├──────────────────────────────────────┤
│  > abaikan                           │  selected row (accent highlight)
│    abang                             │
│    abdi                              │
│    abet                              │
│    abnormal                          │
│    ...scrollable...                  │
├──────────────────────────────────────┤
│  Esc hide · Enter remove · ^Z undo  │  hint bar
└──────────────────────────────────────┘
```

### Sections top to bottom

1. **Title bar** — 28px. App name centered, single `×` button (calls `quit_app`). Entire bar is a drag region. No dotline pattern, no Hide button.
2. **Search area** — full-width input, autofocused. Below it a single line of muted text showing match count ("23 matches") or flash feedback ("Removed: abang" for 1.5s).
3. **Word list** — scrollable, takes remaining vertical space. Plain text rows, no card/box per item. Selected row has a left accent bar + lighter background.
4. **Hint bar** — single line at bottom, muted small text: `Esc hide · Enter remove · Ctrl+Z undo`.

## Removed UI elements

- Tab navigation (Finder/Session/Hotkeys/Config)
- Left sidebar (Session Controls, Live Stats, Recent Removed)
- Inspector panel (selected word details, keyboard flow hints)
- Remove/Undo buttons next to search input
- Chips, badges, module titles, stat cards
- Summary line with full stats breakdown
- Status bar (replaced by hint bar)

## Visual Style

### Colors

Same dark palette, simplified application:

- Frame background: `#171a23` solid, no gradients
- Input: `#232738` background, 1px border `rgba(255,255,255,0.08)`, accent border on focus (`rgba(163,138,223,0.45)`)
- List rows: no individual backgrounds, transparent
- Selected row: `rgba(163,138,223,0.15)` background + 3px left border `#a38adf`
- Text: `#eef2ff` primary, `#9ea6c0` muted
- Close button: `rgba(255,255,255,0.08)` background, red on hover

### Typography

- Font stack: unchanged (Bahnschrift, Segoe UI, Trebuchet MS, sans-serif)
- Input: 1rem
- Word list items: 0.9rem
- Match count + hint bar: 0.75rem, muted color
- No uppercase transforms, no letter-spacing

### Spacing

- Frame padding: 12px
- Gap between search area and list: 8px
- List item padding: 8px vertical, 12px horizontal
- Title bar height: 28px
- Hint bar padding: 8px

### Interactions

- No hover transforms (remove all `translateY(-1px)`)
- Selected row: static accent highlight
- Word removal: instant (no animation)
- Flash feedback: text swap in match count area, 1.5s duration

## Behavior

### Kept identical

- Arrow Up/Down, PgUp/PgDn, Home/End for list navigation
- Enter/Delete to remove selected word
- Ctrl+Z to undo last removal
- Escape to hide overlay
- Space global shortcut to toggle overlay
- `overlay://focus-search` event focuses input on show
- Removed words tracked in-memory (not persisted)
- Search fires on every keystroke via `$effect`
- `requestToken` pattern for stale response handling

### Removed

- Tab switching shortcuts (Alt+1/2/3/4)
- Ctrl+L to focus input (unnecessary — input is always accessible)
- All tab panel rendering (Session, Hotkeys, Config)

### Changed

- Flash feedback displays inline in match count area instead of status bar
- `×` button calls `quit_app` (full exit). Escape hides.
- No Remove/Undo buttons — keyboard only

## Scope

### In scope

- Rewrite `src/routes/+page.svelte` (template, script, styles)
- Update window dimensions in `src-tauri/tauri.conf.json`

### Out of scope

- Rust backend changes
- New files or components
- Tauri capabilities changes
- Wordlist changes
