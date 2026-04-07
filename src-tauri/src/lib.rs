use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Window};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent, ShortcutState};

const EMBEDDED_WORDLISTS: [(&str, &[u8]); 4] = [
    (
        "indonesian-wordlist.txt",
        include_bytes!("../resources/indonesian-wordlist.txt"),
    ),
    (
        "kbbi3-2001-sort-alpha.lst",
        include_bytes!("../resources/kbbi3-2001-sort-alpha.lst"),
    ),
    (
        "ivanlanin2011-sort-alpha.lst",
        include_bytes!("../resources/ivanlanin2011-sort-alpha.lst"),
    ),
    (
        "myspell2006-sort-alpha.lst",
        include_bytes!("../resources/myspell2006-sort-alpha.lst"),
    ),
];
const OVERLAY_EVENT: &str = "overlay://focus-search";
const TOGGLE_SHORTCUT: &str = "Space";
const DEFAULT_LIMIT: usize = 60;
const MAX_LIMIT: usize = 100;

static WORD_INDEX: OnceLock<WordIndex> = OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    query: String,
    limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogInfo {
    shortcut: String,
    word_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WordlistInfo {
    name: String,
    raw_lines: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WordSuggestion {
    word: String,
    length: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    query: String,
    needle: String,
    total_matches: usize,
    total_words: usize,
    results: Vec<WordSuggestion>,
    message: String,
}

#[derive(Debug)]
struct WordEntry {
    display: String,
    normalized: String,
    length: usize,
}

#[derive(Debug, Default)]
struct WordIndex {
    entries: Vec<WordEntry>,
    buckets: HashMap<String, Vec<usize>>,
    wordlists: Vec<WordlistInfo>,
}

impl WordIndex {
    fn load() -> Self {
        let mut entries = Vec::new();
        let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
        let mut seen = HashSet::new();
        let mut wordlists = Vec::new();

        for (name, bytes) in EMBEDDED_WORDLISTS {
            let content = String::from_utf8_lossy(bytes);
            let raw_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
            wordlists.push(WordlistInfo {
                name: name.to_string(),
                raw_lines,
            });

            for raw_line in content.lines() {
                let display = raw_line.trim();
                if display.is_empty() {
                    continue;
                }

                let normalized = normalize_token(display);
                if normalized.is_empty() || !seen.insert(normalized.clone()) {
                    continue;
                }

                let entry = WordEntry {
                    display: display.to_string(),
                    length: normalized.chars().count(),
                    normalized,
                };

                let next_index = entries.len();
                for prefix_len in 1..=3 {
                    if let Some(key) = first_chars(&entry.normalized, prefix_len) {
                        buckets.entry(key).or_default().push(next_index);
                    }
                }

                entries.push(entry);
            }
        }

        Self { entries, buckets, wordlists }
    }

    fn total_words(&self) -> usize {
        self.entries.len()
    }

    fn search(&self, request: SearchRequest) -> SearchResponse {
        let query = normalize_query(&request.query);
        let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let needle = query.clone();

        if query.is_empty() {
            return SearchResponse {
                query,
                needle,
                total_matches: 0,
                total_words: self.total_words(),
                results: Vec::new(),
                message: "Type a word prefix to start searching.".to_string(),
            };
        }

        let (total_matches, results) = self.search_prefix(&needle, limit);

        let message = if total_matches == 0 {
            format!("No words start with \"{}\".", needle)
        } else if total_matches > results.len() {
            format!(
                "Showing {} of {} results for prefix \"{}\".",
                results.len(),
                total_matches,
                needle
            )
        } else {
            format!("Found {} results for prefix \"{}\".", total_matches, needle)
        };

        SearchResponse {
            query,
            needle,
            total_matches,
            total_words: self.total_words(),
            results,
            message,
        }
    }

    fn search_prefix(&self, needle: &str, limit: usize) -> (usize, Vec<WordSuggestion>) {
        let Some(bucket_key) = first_chars(needle, needle.chars().count().min(3)) else {
            return (0, Vec::new());
        };

        let Some(indexes) = self.buckets.get(&bucket_key) else {
            return (0, Vec::new());
        };

        self.collect_matches(indexes, limit, |entry| entry.normalized.starts_with(needle))
    }

    fn collect_matches<F>(
        &self,
        indexes: &[usize],
        limit: usize,
        predicate: F,
    ) -> (usize, Vec<WordSuggestion>)
    where
        F: Fn(&WordEntry) -> bool,
    {
        let mut total_matches = 0;
        let mut results = Vec::with_capacity(limit.min(indexes.len()));

        for &index in indexes {
            let entry = &self.entries[index];
            if !predicate(entry) {
                continue;
            }

            total_matches += 1;

            if results.len() < limit {
                results.push(WordSuggestion {
                    word: entry.display.clone(),
                    length: entry.length,
                });
            }
        }

        // Sort by length (shortest to longest)
        results.sort_by_key(|s| s.length);

        (total_matches, results)
    }
}

fn word_index() -> &'static WordIndex {
    WORD_INDEX.get_or_init(WordIndex::load)
}

fn normalize_query(value: &str) -> String {
    normalize_token(value)
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphabetic() || *character == '-')
        .collect()
}

fn first_chars(value: &str, count: usize) -> Option<String> {
    let collected: String = value.chars().take(count).collect();
    if collected.chars().count() == count {
        Some(collected)
    } else {
        None
    }
}

fn focus_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        let _ = window.set_focus();
        let _ = app.emit(OVERLAY_EVENT, ());
    }
}

fn toggle_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let should_show = window.is_visible().map(|visible| !visible).unwrap_or(true);
        if should_show {
            focus_overlay(app);
        } else {
            let _ = window.hide();
        }
    }
}

fn handle_shortcut(app: &AppHandle, event: ShortcutEvent) {
    if event.state == ShortcutState::Released {
        toggle_overlay(app);
    }
}

#[tauri::command]
fn wordlist_info() -> Vec<WordlistInfo> {
    word_index().wordlists.clone()
}

#[tauri::command]
fn catalog_info() -> CatalogInfo {
    CatalogInfo {
        shortcut: TOGGLE_SHORTCUT.to_string(),
        word_count: word_index().total_words(),
    }
}

#[tauri::command]
fn search_words(request: SearchRequest) -> SearchResponse {
    word_index().search(request)
}

#[tauri::command]
fn hide_overlay(window: Window) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn calculate_text_to_type(word: &str, query: &str) -> String {
    let normalized_word = normalize_token(word);
    let normalized_query = normalize_token(query);

    if normalized_word.starts_with(&normalized_query) {
        normalized_word[normalized_query.len()..].to_string()
    } else {
        normalized_word
    }
}

fn char_to_key(ch: char) -> enigo::Key {
    use enigo::Key;
    match ch.to_lowercase().next().unwrap_or(ch) {
        'a' => Key::A,
        'b' => Key::B,
        'c' => Key::C,
        'd' => Key::D,
        'e' => Key::E,
        'f' => Key::F,
        'g' => Key::G,
        'h' => Key::H,
        'i' => Key::I,
        'j' => Key::J,
        'k' => Key::K,
        'l' => Key::L,
        'm' => Key::M,
        'n' => Key::N,
        'o' => Key::O,
        'p' => Key::P,
        'q' => Key::Q,
        'r' => Key::R,
        's' => Key::S,
        't' => Key::T,
        'u' => Key::U,
        'v' => Key::V,
        'w' => Key::W,
        'x' => Key::X,
        'y' => Key::Y,
        'z' => Key::Z,
        ' ' => Key::Space,
        _ => Key::Space, // Default for unsupported characters
    }
}

#[tauri::command]
async fn type_and_hide(
    window: tauri::Window,
    word: String,
    query: String,
    speed: u64,
    randomize: Option<bool>,
) -> Result<(), String> {
    let text_to_type = calculate_text_to_type(&word, &query);

    // Hide window immediately
    window.hide().map_err(|e| e.to_string())?;

    // Wait for window to fully hide and focus to transfer to the game
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Simulate keystrokes with delay (using key presses for game compatibility)
    use enigo::{Enigo, Settings, Keyboard, Direction};
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to initialize keyboard controller: {:?}", e))?;

    let should_randomize = randomize.unwrap_or(false);

    for ch in text_to_type.chars() {
        let key = char_to_key(ch);

        // Press key
        enigo
            .key(key, Direction::Press)
            .map_err(|e| format!("Failed to press key: {:?}", e))?;

        // Release key
        enigo
            .key(key, Direction::Release)
            .map_err(|e| format!("Failed to release key: {:?}", e))?;

        // Calculate actual delay with randomization if enabled
        let actual_delay = if should_randomize {
            // ±40% variance for human-like typing
            use rand::Rng;
            let variance = rand::thread_rng().gen_range(-40..=40) as f64 / 100.0;
            let delay = (speed as f64 * (1.0 + variance)) as u64;
            delay.max(10) // Minimum 10ms
        } else {
            speed
        };

        tokio::time::sleep(std::time::Duration::from_millis(actual_delay)).await;
    }

    // Press Enter to submit the word
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    enigo
        .key(enigo::Key::Return, Direction::Press)
        .map_err(|e| format!("Failed to press Enter: {:?}", e))?;
    enigo
        .key(enigo::Key::Return, Direction::Release)
        .map_err(|e| format!("Failed to release Enter: {:?}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _ = word_index();

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
                let _ = window.set_skip_taskbar(true);
            }

            if let Err(error) =
                app.global_shortcut()
                    .on_shortcut(TOGGLE_SHORTCUT, |app, _shortcut, event| {
                        handle_shortcut(app, event);
                    })
            {
                eprintln!("failed to register global shortcut: {error}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            catalog_info,
            wordlist_info,
            search_words,
            hide_overlay,
            quit_app,
            type_and_hide
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
