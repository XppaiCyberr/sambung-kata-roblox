<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";

  type CatalogInfo = { shortcut: string; wordCount: number };
  type WordSuggestion = { word: string; length: number };
  type SearchResponse = {
    query: string;
    needle: string;
    totalMatches: number;
    totalWords: number;
    results: WordSuggestion[];
    message: string;
  };

  const SEARCH_LIMIT = 60;
  const OVERLAY_EVENT = "overlay://focus-search";
  const formatter = new Intl.NumberFormat("en-US");

  let query = $state("");
  let loading = $state(false);
  let errorMessage = $state("");
  let feedback = $state("");
  let selectedIndex = $state(0);
  let removedWords = $state<string[]>([]);
  let removalHistory = $state<string[]>([]);
  let response = $state<SearchResponse | null>(null);
  let catalog = $state<CatalogInfo>({ shortcut: "Space", wordCount: 0 });

  let inputElement = $state<HTMLInputElement | null>(null);
  let feedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let requestToken = 0;

  const removedWordSet = $derived(new Set(removedWords));
  const visibleResults = $derived(
    (response?.results ?? []).filter((r) => !removedWordSet.has(r.word)),
  );
  const visibleCount = $derived(visibleResults.length);
  const selectedWord = $derived(visibleResults[selectedIndex] ?? null);

  const infoLine = $derived.by(() => {
    if (feedback) return feedback;
    if (errorMessage) return errorMessage;
    if (response && response.results.length > 0 && visibleCount === 0) {
      return "All candidates removed. Ctrl+Z to restore.";
    }
    if (response && response.totalMatches > 0) {
      return `${formatter.format(response.totalMatches)} matches`;
    }
    return "";
  });

  async function loadCatalogInfo() {
    try {
      catalog = await invoke<CatalogInfo>("catalog_info");
    } catch (error) {
      errorMessage = `Failed to load catalog: ${String(error)}`;
    }
  }

  async function runSearch() {
    const token = ++requestToken;
    loading = true;
    errorMessage = "";
    try {
      const next = await invoke<SearchResponse>("search_words", {
        request: { query, limit: SEARCH_LIMIT },
      });
      if (token !== requestToken) return;
      response = next;
      selectedIndex = 0;
    } catch (error) {
      if (token !== requestToken) return;
      errorMessage = `Search failed: ${String(error)}`;
    } finally {
      if (token === requestToken) loading = false;
    }
  }

  async function focusSearch(selectAll = false) {
    await tick();
    inputElement?.focus();
    if (selectAll) inputElement?.select();
  }

  async function hideOverlay() {
    await invoke("hide_overlay");
  }

  async function quitApp() {
    await invoke("quit_app");
  }

  function flashFeedback(message: string) {
    feedback = message;
    if (feedbackTimer) clearTimeout(feedbackTimer);
    feedbackTimer = setTimeout(() => {
      feedback = "";
    }, 1500);
  }

  function removeWord(word: string) {
    if (removedWordSet.has(word)) return;
    removedWords = [...removedWords, word];
    removalHistory = [...removalHistory, word];
    flashFeedback(`Removed: ${word}`);
  }

  function removeSelected() {
    if (!selectedWord) return;
    removeWord(selectedWord.word);
  }

  function undoLastRemoval() {
    const last = removalHistory[removalHistory.length - 1];
    if (!last) {
      flashFeedback("Nothing to undo.");
      return;
    }
    removalHistory = removalHistory.slice(0, -1);
    removedWords = removedWords.filter((w) => w !== last);
    flashFeedback(`Restored: ${last}`);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key.toLowerCase() === "z") {
      event.preventDefault();
      undoLastRemoval();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      void hideOverlay();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (visibleCount > 0) selectedIndex = (selectedIndex + 1) % visibleCount;
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (visibleCount > 0) selectedIndex = (selectedIndex - 1 + visibleCount) % visibleCount;
      return;
    }
    if (event.key === "PageDown") {
      event.preventDefault();
      if (visibleCount > 0) selectedIndex = Math.min(selectedIndex + 8, visibleCount - 1);
      return;
    }
    if (event.key === "PageUp") {
      event.preventDefault();
      if (visibleCount > 0) selectedIndex = Math.max(selectedIndex - 8, 0);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      selectedIndex = 0;
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      if (visibleCount > 0) selectedIndex = visibleCount - 1;
      return;
    }
    if (event.key === "Enter" || event.key === "Delete") {
      event.preventDefault();
      removeSelected();
    }
  }

  $effect(() => {
    query;
    void runSearch();
  });

  $effect(() => {
    if (visibleCount === 0) {
      selectedIndex = 0;
      return;
    }
    if (selectedIndex >= visibleCount) selectedIndex = visibleCount - 1;
  });

  $effect(() => {
    selectedIndex;
    visibleResults.length;
    void tick().then(() => {
      document.querySelector<HTMLButtonElement>(".word-row.selected")?.scrollIntoView({
        block: "nearest",
      });
    });
  });

  onMount(() => {
    let stopListening: (() => void) | undefined;
    void loadCatalogInfo();
    void focusSearch(true);
    void listen(OVERLAY_EVENT, async () => {
      await focusSearch(true);
    }).then((unlisten) => {
      stopListening = unlisten;
    });
    return () => {
      if (feedbackTimer) clearTimeout(feedbackTimer);
      stopListening?.();
    };
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <title>XppaiCyber | SambungKata VIP</title>
</svelte:head>

<div class="frame">
  <div class="titlebar">
    <span class="titlebar-text">XppaiCyber | SambungKata VIP</span>
    <button type="button" class="close-btn" onclick={quitApp}>&times;</button>
  </div>

  <div class="search-area">
    <input
      bind:this={inputElement}
      bind:value={query}
      autocomplete="off"
      spellcheck="false"
      placeholder="Type prefix..."
      class="search-input"
    />
    <div class="stats-line">
      <span>{formatter.format(catalog.wordCount)} words</span>
      {#if removedWords.length > 0}
        <span>{formatter.format(removedWords.length)} removed</span>
      {/if}
      {#if infoLine}
        <span>{infoLine}</span>
      {/if}
    </div>
  </div>

  <div class="word-list">
    {#if loading && visibleCount === 0}
      <p class="empty">Searching...</p>
    {:else if visibleCount > 0}
      {#each visibleResults as result, index}
        <button
          type="button"
          class="word-row"
          class:selected={selectedIndex === index}
          onclick={() => {
            selectedIndex = index;
            removeWord(result.word);
          }}
        >
          {result.word}
        </button>
      {/each}
    {:else if query}
      <p class="empty">No matches</p>
    {/if}
  </div>

  <div class="hint-bar">
    <span>Esc hide</span>
    <span>Enter remove</span>
    <span>Ctrl+Z undo</span>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background: #171a23;
    color: #eef2ff;
    font-family: "Bahnschrift", "Segoe UI", "Trebuchet MS", sans-serif;
  }
  :global(button),
  :global(input) {
    font: inherit;
  }

  .frame {
    height: 100vh;
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    box-sizing: border-box;
  }

  .titlebar {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    height: 28px;
    padding: 0 8px;
    background: #1d2130;
    user-select: none;
  }
  .titlebar-text {
    font-size: 0.82rem;
    color: #9ea6c0;
    letter-spacing: 0.02em;
  }
  .close-btn {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: #9ea6c0;
    font-size: 1rem;
    cursor: pointer;
    line-height: 1;
  }
  .close-btn:hover {
    background: #c46767;
    color: #fff;
  }

  .search-area {
    padding: 12px 12px 0;
  }
  .search-input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: #232738;
    color: #eef2ff;
    font-size: 1rem;
    outline: none;
    box-sizing: border-box;
  }
  .search-input:focus {
    border-color: rgba(163, 138, 223, 0.45);
  }
  .search-input::placeholder {
    color: rgba(255, 255, 255, 0.25);
  }
  .stats-line {
    display: flex;
    gap: 12px;
    margin: 6px 0 0;
    font-size: 0.75rem;
    color: #9ea6c0;
  }

  .word-list {
    min-height: 0;
    overflow-y: auto;
    padding: 8px 6px;
  }
  .word-list::-webkit-scrollbar {
    width: 4px;
  }
  .word-list::-webkit-scrollbar-track {
    background: transparent;
  }
  .word-list::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
  }

  .word-row {
    display: block;
    width: 100%;
    padding: 7px 12px;
    border: 0;
    border-left: 3px solid transparent;
    border-radius: 0;
    background: transparent;
    color: #eef2ff;
    font-size: 0.9rem;
    text-align: left;
    cursor: pointer;
  }
  .word-row.selected {
    background: rgba(163, 138, 223, 0.12);
    border-left-color: #a38adf;
  }
  .word-row:hover:not(.selected) {
    background: rgba(255, 255, 255, 0.03);
  }

  .empty {
    padding: 20px 12px;
    margin: 0;
    text-align: center;
    font-size: 0.82rem;
    color: #9ea6c0;
  }

  .hint-bar {
    display: flex;
    justify-content: center;
    gap: 16px;
    padding: 8px;
    font-size: 0.7rem;
    color: rgba(158, 166, 192, 0.6);
    border-top: 1px solid rgba(255, 255, 255, 0.04);
  }
</style>
