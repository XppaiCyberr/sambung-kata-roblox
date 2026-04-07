# Autocomplete Keystroke Feature Design

**Date:** 2026-04-07  
**Feature:** Keystroke Automation on Word Selection  
**Status:** Design Approved

## Overview

When a user selects a word from search results and presses Enter, the overlay hides and automatically types the remaining characters (or full word if unrelated to the search prefix). The typing speed is configurable via `Ctrl+T` dialog, defaulting to 50ms per character. The overlay remains hidden until the user toggles it again with the Space shortcut.

## User Flow

1. User opens overlay (Space)
2. User types search prefix (e.g., "ba")
3. Results appear including "bahu"
4. User navigates to "bahu" with arrow keys
5. User presses Enter
6. Overlay immediately hides
7. System types "hu" (remainder after "ba") with 50ms delays between keystrokes
8. User continues their game
9. User presses Space to reopen overlay and search again

Alternative: If selected word doesn't start with search prefix (e.g., search "ba" but somehow selected "makan"), the system types the full word "makan".

## Technical Design

### Frontend (`src/routes/+page.svelte`)

#### New State
- `typingSpeed: number` — current typing speed in milliseconds (default 50, persisted to localStorage under key `typing_speed_ms`)
- `showSpeedDialog: boolean` — whether speed config modal is visible

#### New Event Handlers
- Extend `handleKeyDown()` to detect Enter key:
  - If a word is selected (`selectedWord !== null`), invoke Tauri command `type_and_hide` with:
    - `word`: the selected word's display text
    - `query`: the current search query (normalized)
    - `speed`: the current `typingSpeed` value
  - Prevent default behavior
  - After invocation, set `query = ""` and `response = null` to reset UI state

- Add `Ctrl+T` handler to toggle `showSpeedDialog`

#### New UI Component
- Modal dialog (appears over search results):
  - Displays current speed value
  - Input field accepting integers 10–500ms
  - Buttons: "Apply" and "Cancel"
  - On "Apply": update `typingSpeed`, save to localStorage, close dialog
  - On "Cancel": close dialog without saving
  - ESC key closes dialog

#### localStorage Persistence
- On app initialization (`onMount`), attempt to read `typing_speed_ms` from localStorage
- If present and valid (10–500), use that value; otherwise use default 50ms
- On speed dialog apply, write new value to localStorage

### Backend (`src-tauri/src/lib.rs`)

#### New Dependencies
- Add to `Cargo.toml`: `enigo = "0.1"` (keyboard simulation crate)

#### New Tauri Command: `type_and_hide`
```rust
#[tauri::command]
async fn type_and_hide(
    window: Window,
    word: String,
    query: String,
    speed: u64,
) -> Result<(), String>
```

**Parameters:**
- `word`: the selected word to type (e.g., "bahu")
- `query`: the normalized search query (e.g., "ba")
- `speed`: typing speed in milliseconds per character (10–500)

**Logic:**
1. Calculate text to type:
   - Normalize both `word` and `query` using existing normalization function
   - If normalized `word` starts with normalized `query`, extract the remainder
   - Otherwise, use the full word
2. Hide the window: `window.hide().map_err(|e| e.to_string())?`
3. Use `enigo` to simulate keystrokes:
   - For each character in the text:
     - Type the character
     - Sleep for `speed` milliseconds
4. Return `Ok(())`

**Error Handling:**
- If window hide fails, return descriptive error (user sees in console)
- If keyboard automation fails (rare on Windows), return error message
- No recovery needed — overlay can be reopened with Space

#### Word Normalization
- Use the existing normalization logic already present in the codebase (strips non-alphabetic chars except hyphens, lowercases)
- Ensure `type_and_hide` uses identical normalization to search query normalization

#### Integration
- Register command in `setup()` function
- No changes to existing `search_words`, `catalog_info`, or other commands

## Configuration

- **Default typing speed:** 50ms per character
- **Speed range:** 10–500ms (enforced in frontend dialog and backend validation)
- **Persistence:** localStorage key `typing_speed_ms` (frontend only)
- **Keyboard shortcut:** `Ctrl+T` to open speed config dialog
- **Action shortcut:** Enter key on selected word triggers keystroke + hide

## Success Criteria

- [ ] User can press Enter on a selected word and it types the remainder
- [ ] Overlay hides immediately after typing begins
- [ ] Typing speed matches configuration (±10ms tolerance acceptable)
- [ ] `Ctrl+T` opens configurable speed dialog
- [ ] Speed persists across app restarts
- [ ] Overlay reopens with Space after typing completes
- [ ] Full word is typed if selected word doesn't match search prefix

## Dependencies

- `enigo` crate (Windows keyboard simulation)
- Existing `@tauri-apps/api/core` invoke
- Existing word normalization logic

## Future Considerations

- Per-game speed presets if needed
- Mouse click to select + auto-type (extend beyond keyboard)
- Typing delay randomization (simulate human variation)
