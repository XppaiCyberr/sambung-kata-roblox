# Keystroke Autocomplete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When user presses Enter on a selected word, hide overlay and automatically type the remainder of the word (or full word if unrelated to search prefix) with configurable keystroke speed.

**Architecture:** Frontend captures Enter key and invokes new Tauri backend command. Backend uses `enigo` crate to simulate keystrokes with configured delays. Speed setting is stored in browser localStorage and persists across restarts. Modal dialog (spawned by Ctrl+T) allows real-time speed adjustment.

**Tech Stack:** Tauri 2, enigo 0.1 (Windows keyboard simulation), Svelte 5 runes, TypeScript, Rust

---

### Task 1: Add enigo Dependency to Cargo.toml

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add enigo to dependencies**

Open `src-tauri/Cargo.toml` and add `enigo = "0.1"` to the `[dependencies]` section (after `serde_json = "1"`):

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
enigo = "0.1"
```

- [ ] **Step 2: Verify dependency resolves**

Run from `src-tauri/`:
```bash
cargo check
```

Expected: Builds successfully, no errors.

- [ ] **Step 3: Commit**

```bash
cd src-tauri
git add Cargo.toml Cargo.lock
git commit -m "deps: add enigo for keyboard simulation"
```

---

### Task 2: Implement type_and_hide Command in Backend

**Files:**
- Modify: `src-tauri/src/lib.rs` (add new command + helper function)

- [ ] **Step 1: Add helper function for text remainder calculation**

Find the `WordIndex` impl block in `src-tauri/src/lib.rs` (around line 88). Add a helper function before the `pub async fn run_app()` function. Insert after the existing struct definitions, before the setup function:

```rust
fn normalize_text(text: &str) -> String {
    text
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase()
}

fn calculate_text_to_type(word: &str, query: &str) -> String {
    let normalized_word = normalize_text(word);
    let normalized_query = normalize_text(query);
    
    if normalized_word.starts_with(&normalized_query) {
        normalized_word[normalized_query.len()..].to_string()
    } else {
        normalized_word
    }
}
```

- [ ] **Step 2: Add type_and_hide Tauri command**

Find the `#[tauri::command]` section in lib.rs (search for `pub async fn search_words`). Add the new command after all existing commands:

```rust
#[tauri::command]
async fn type_and_hide(
    window: tauri::Window,
    word: String,
    query: String,
    speed: u64,
) -> Result<(), String> {
    let text_to_type = calculate_text_to_type(&word, &query);
    
    // Hide window immediately
    window.hide().map_err(|e| e.to_string())?;
    
    // Simulate keystrokes with delay
    use enigo::Enigo;
    let mut enigo = Enigo::new().map_err(|e| format!("Failed to init keyboard: {:?}", e))?;
    
    for ch in text_to_type.chars() {
        enigo
            .text(&ch.to_string())
            .map_err(|e| format!("Failed to type character: {:?}", e))?;
        std::thread::sleep(std::time::Duration::from_millis(speed));
    }
    
    Ok(())
}
```

- [ ] **Step 3: Register command in setup function**

Find the `pub async fn run_app()` function (search for it). Inside, find the builder chain that adds commands (search for `.invoke_handler`). Add `type_and_hide` to the command list. It should look like:

```rust
.invoke_handler(tauri::generate_handler![
    catalog_info,
    search_words,
    hide_overlay,
    quit_app,
    wordlist_info,
    type_and_hide,
])
```

(Add `type_and_hide` to the list if not already there.)

- [ ] **Step 4: Verify compilation**

Run from `src-tauri/`:
```bash
cargo check
```

Expected: No compilation errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add type_and_hide command for keystroke automation"
```

---

### Task 3: Add Enter Key Handler in Frontend

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Find and extend handleKeyDown function**

Open `src/routes/+page.svelte`. Find the `function handleKeyDown(event: KeyboardEvent)` function (search for "handleKeyDown"). Add Enter key handling at the beginning of the function:

```typescript
function handleKeyDown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    if (selectedWord) {
      handleEnter();
    }
    return;
  }
  
  // ... rest of existing handlers (ArrowUp, ArrowDown, etc.)
}
```

- [ ] **Step 2: Add handleEnter function**

Add this new function right after `handleKeyDown`:

```typescript
async function handleEnter() {
  if (!selectedWord) return;
  
  try {
    await invoke("type_and_hide", {
      word: selectedWord.word,
      query: query,
      speed: typingSpeed,
    });
    
    // Reset UI after typing
    query = "";
    response = null;
    selectedIndex = 0;
  } catch (error) {
    errorMessage = `Failed to type: ${String(error)}`;
  }
}
```

- [ ] **Step 3: Verify no syntax errors**

Run:
```bash
npm run check
```

Expected: No type errors.

- [ ] **Step 4: Test manually in dev mode**

Run:
```bash
npm run tauri dev
```

- Type a search query (e.g., "ba")
- Arrow to select a word
- Press Enter
- Verify overlay hides and nothing is typed yet (we haven't added typing speed config)

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: add Enter key handler for keystroke trigger"
```

---

### Task 4: Add typingSpeed State and localStorage Persistence

**Files:**
- Modify: `src/routes/+page.svelte` (state initialization + effects)

- [ ] **Step 1: Add typingSpeed state variable**

Find the state declarations at the top of the `<script>` block (search for `let query = $state`). Add:

```typescript
let typingSpeed = $state(50); // milliseconds per character
```

- [ ] **Step 2: Add localStorage load on mount**

Find the `onMount` hook (search for `onMount`). Add this code inside the `onMount` callback at the very beginning:

```typescript
// Load typing speed from localStorage
const savedSpeed = localStorage.getItem("typing_speed_ms");
if (savedSpeed) {
  const parsed = parseInt(savedSpeed, 10);
  if (!isNaN(parsed) && parsed >= 10 && parsed <= 500) {
    typingSpeed = parsed;
  }
}
```

The full `onMount` should look like:

```typescript
onMount(async () => {
  // Load typing speed from localStorage
  const savedSpeed = localStorage.getItem("typing_speed_ms");
  if (savedSpeed) {
    const parsed = parseInt(savedSpeed, 10);
    if (!isNaN(parsed) && parsed >= 10 && parsed <= 500) {
      typingSpeed = parsed;
    }
  }
  
  await loadApp();
});
```

- [ ] **Step 3: Create function to save speed to localStorage**

Add this helper function right after the `handleEnter` function:

```typescript
function saveTypingSpeed(speed: number) {
  if (speed >= 10 && speed <= 500) {
    localStorage.setItem("typing_speed_ms", speed.toString());
    typingSpeed = speed;
  }
}
```

- [ ] **Step 4: Run type check**

```bash
npm run check
```

Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: add typing speed state with localStorage persistence"
```

---

### Task 5: Add Speed Config Dialog State and Handler

**Files:**
- Modify: `src/routes/+page.svelte` (state + handler)

- [ ] **Step 1: Add showSpeedDialog state**

In the state declarations section (where you added `typingSpeed`), add:

```typescript
let showSpeedDialog = $state(false);
let speedDialogInput = $state(""); // temp input for dialog
```

- [ ] **Step 2: Add Ctrl+T handler to handleKeyDown**

Find the `handleKeyDown` function. At the end of the function (before the closing brace), add:

```typescript
  if (event.ctrlKey && event.key.toLowerCase() === "t") {
    event.preventDefault();
    showSpeedDialog = true;
    speedDialogInput = typingSpeed.toString();
    return;
  }
```

The function should now start like:

```typescript
function handleKeyDown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    if (selectedWord) {
      handleEnter();
    }
    return;
  }
  
  if (event.ctrlKey && event.key.toLowerCase() === "t") {
    event.preventDefault();
    showSpeedDialog = true;
    speedDialogInput = typingSpeed.toString();
    return;
  }
  
  // ... rest of handlers
}
```

- [ ] **Step 3: Add speed dialog apply/cancel handlers**

Add these functions right after `saveTypingSpeed`:

```typescript
function applySpeedDialog() {
  const parsed = parseInt(speedDialogInput, 10);
  if (!isNaN(parsed) && parsed >= 10 && parsed <= 500) {
    saveTypingSpeed(parsed);
    showSpeedDialog = false;
    feedback = `Typing speed set to ${parsed}ms`;
    feedbackTimer = setTimeout(() => (feedback = ""), 2000);
  } else {
    errorMessage = "Speed must be between 10 and 500 ms";
  }
}

function cancelSpeedDialog() {
  showSpeedDialog = false;
  speedDialogInput = "";
}
```

- [ ] **Step 4: Run type check**

```bash
npm run check
```

Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: add Ctrl+T handler and speed dialog state management"
```

---

### Task 6: Add Speed Dialog UI to Template

**Files:**
- Modify: `src/routes/+page.svelte` (template section)

- [ ] **Step 1: Find the main content div**

In the `<div>` section (the HTML template part), find where the search input and results are rendered. Look for the structure with `{#if appReady}`. You'll add the dialog modal right after the closing `</div>` of the main content, still inside the `appReady` block.

Find this line (the main content wrapper):
```svelte
  <div class="container">
    <!-- search input, results, etc -->
  </div>
```

After the closing `</div>`, add the modal:

```svelte
  {#if showSpeedDialog}
    <div class="modal-overlay" on:keydown={(e) => e.key === "Escape" && cancelSpeedDialog()}>
      <div class="modal-content">
        <h3>Typing Speed</h3>
        <p>Speed (10-500 ms per character):</p>
        <input
          type="number"
          bind:value={speedDialogInput}
          min="10"
          max="500"
          onkeydown={(e) => e.key === "Enter" && applySpeedDialog()}
        />
        <div class="modal-buttons">
          <button onclick={() => applySpeedDialog()}>Apply</button>
          <button onclick={() => cancelSpeedDialog()}>Cancel</button>
        </div>
      </div>
    </div>
  {/if}
```

- [ ] **Step 2: Add modal styles to the style block**

At the bottom of the file, find the `<style>` block. Add these styles before the closing `</style>`:

```css
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: white;
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  min-width: 300px;
  color: #333;
}

.modal-content h3 {
  margin-top: 0;
  font-size: 18px;
}

.modal-content p {
  margin: 10px 0 5px 0;
  font-size: 14px;
}

.modal-content input {
  width: 100%;
  padding: 8px;
  margin-bottom: 15px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 14px;
  box-sizing: border-box;
}

.modal-buttons {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.modal-buttons button {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.modal-buttons button:first-child {
  background: #4CAF50;
  color: white;
}

.modal-buttons button:first-child:hover {
  background: #45a049;
}

.modal-buttons button:last-child {
  background: #f44336;
  color: white;
}

.modal-buttons button:last-child:hover {
  background: #da190b;
}
```

- [ ] **Step 3: Run type check**

```bash
npm run check
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: add speed config dialog UI and styling"
```

---

### Task 7: Integration Test in Dev Mode

**Files:**
- No files modified (manual testing only)

- [ ] **Step 1: Start dev server**

```bash
npm run tauri dev
```

Expected: Overlay appears, app loads, ready screen shows.

- [ ] **Step 2: Test basic search and keystroke**

1. Type "ba" in search
2. Arrow down to select "bahu" (or any word starting with "ba")
3. Press Enter
4. Verify overlay hides
5. Verify characters appear in text editor or game (system-wide keystroke simulation)
6. Press Space to reopen overlay

Expected: All steps work as described.

- [ ] **Step 3: Test speed config dialog**

1. While overlay is visible, press Ctrl+T
2. Verify modal dialog appears showing current speed (should be 50)
3. Change value to 100
4. Click Apply
5. Verify dialog closes and feedback message shows "Typing speed set to 100ms"
6. Verify speed persists: close and reopen overlay (Space), then Ctrl+T again — should show 100

Expected: Dialog works, value persists.

- [ ] **Step 4: Test invalid speed input**

1. Press Ctrl+T to open dialog
2. Try entering 5 (below minimum)
3. Click Apply
4. Verify error message: "Speed must be between 10 and 500 ms"
5. Try entering 600 (above maximum)
6. Click Apply
7. Verify same error

Expected: Validation works.

- [ ] **Step 5: Test keystroke with different speeds**

1. Set speed to 200ms via Ctrl+T
2. Search and select a word, press Enter
3. Observe slower typing
4. Set speed to 10ms via Ctrl+T
5. Search and select another word, press Enter
6. Observe very fast typing

Expected: Speed adjustment is obvious.

- [ ] **Step 6: Test unrelated word selection**

1. Search for "ba"
2. Manually navigate to a word that doesn't start with "ba" (if possible) or use arrow keys to go past results
3. Press Enter on that word
4. Verify full word is typed (not just remainder)

Expected: Full word typed.

- [ ] **Step 7: Test localStorage persistence across restarts**

1. Set typing speed to 150ms via Ctrl+T
2. Close the overlay (Escape or Space)
3. Fully close the app
4. Restart the app
5. Press Ctrl+T
6. Verify speed shows 150

Expected: Speed is remembered.

- [ ] **Step 8: No test failures or console errors**

Check browser console (F12 in dev mode) for any errors. Fix if found, re-run tests.

---

### Task 8: Final Integration Commit

**Files:**
- No additional files (all changes already committed)

- [ ] **Step 1: Verify all changes are committed**

```bash
git status
```

Expected: "On branch main, nothing to commit, working tree clean"

- [ ] **Step 2: View commit log to confirm feature commits**

```bash
git log --oneline -6
```

Expected: Recent commits include:
- "feat: add speed config dialog UI and styling"
- "feat: add Ctrl+T handler and speed dialog state management"
- "feat: add typing speed state with localStorage persistence"
- "feat: add Enter key handler for keystroke trigger"
- "feat: add type_and_hide command for keystroke automation"
- "deps: add enigo for keyboard simulation"
