use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Window};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent, ShortcutState};

const EMBEDDED_WORDLISTS: [&[u8]; 2] = [
    include_bytes!("../resources/indonesian-wordlist.txt"),
    include_bytes!("../resources/kbbi3-2001-sort-alpha.lst"),
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
}

impl WordIndex {
    fn load() -> Self {
        let mut entries = Vec::new();
        let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
        let mut seen = HashSet::new();

        for bytes in EMBEDDED_WORDLISTS {
            let content = String::from_utf8_lossy(bytes);

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

        Self { entries, buckets }
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
                message: "Ketik awalan kata untuk mulai mencari.".to_string(),
            };
        }

        let (total_matches, results) = self.search_prefix(&needle, limit);

        let message = if total_matches == 0 {
            format!("Tidak ada kata yang diawali \"{}\".", needle)
        } else if total_matches > results.len() {
            format!(
                "Menampilkan {} dari {} hasil untuk awalan \"{}\".",
                results.len(),
                total_matches,
                needle
            )
        } else {
            format!(
                "Ditemukan {} hasil untuk awalan \"{}\".",
                total_matches, needle
            )
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
            search_words,
            hide_overlay,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
