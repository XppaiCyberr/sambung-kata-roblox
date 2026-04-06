<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";

  type CatalogInfo = {
    shortcut: string;
    wordCount: number;
  };

  type WordSuggestion = {
    word: string;
    length: number;
  };

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
  const formatter = new Intl.NumberFormat("id-ID");
  const SEARCH_PLACEHOLDER = "Ketik awalan kata";
  const SEARCH_HELP = "Tampilkan semua kata yang diawali huruf yang sama.";

  let query = $state("");
  let loading = $state(false);
  let errorMessage = $state("");
  let feedback = $state("");
  let selectedIndex = $state(0);
  let removedWords = $state<string[]>([]);
  let removalHistory = $state<string[]>([]);
  let response = $state<SearchResponse | null>(null);
  let catalog = $state<CatalogInfo>({
    shortcut: "Space",
    wordCount: 0,
  });

  let inputElement: HTMLInputElement | null = null;
  let feedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let requestToken = 0;

  const removedWordSet = $derived(new Set(removedWords));
  const visibleResults = $derived(
    (response?.results ?? []).filter((result) => !removedWordSet.has(result.word)),
  );
  const removedCount = $derived(removedWords.length);

  const statusLine = $derived.by(() => {
    if (feedback) {
      return feedback;
    }

    if (errorMessage) {
      return errorMessage;
    }

    if (response && response.results.length > 0 && visibleResults.length === 0) {
      return "Semua kandidat di daftar ini sudah dikeluarkan. Tekan Ctrl+Z untuk undo.";
    }

    return response?.message ?? SEARCH_HELP;
  });

  const summaryLine = $derived.by(() => {
    const loaded = formatter.format(response?.totalWords ?? catalog.wordCount);
    const matches = formatter.format(response?.totalMatches ?? 0);
    const needle = response?.needle ? ` | kunci ${response.needle.toUpperCase()}` : "";
    const removed = removedCount > 0 ? ` | ${formatter.format(removedCount)} keluar` : "";
    return `${loaded} kata | ${matches} hasil${needle}${removed}`;
  });

  async function loadCatalogInfo() {
    try {
      catalog = await invoke<CatalogInfo>("catalog_info");
    } catch (error) {
      errorMessage = `Gagal memuat katalog: ${String(error)}`;
    }
  }

  async function runSearch() {
    const token = ++requestToken;
    loading = true;
    errorMessage = "";

    try {
      const nextResponse = await invoke<SearchResponse>("search_words", {
        request: {
          query,
          limit: SEARCH_LIMIT,
        },
      });

      if (token !== requestToken) {
        return;
      }

      response = nextResponse;
      selectedIndex = 0;
    } catch (error) {
      if (token !== requestToken) {
        return;
      }

      errorMessage = `Pencarian gagal: ${String(error)}`;
    } finally {
      if (token === requestToken) {
        loading = false;
      }
    }
  }

  async function focusSearch(selectAll = false) {
    await tick();
    inputElement?.focus();
    if (selectAll) {
      inputElement?.select();
    }
  }

  async function hideOverlay() {
    await invoke("hide_overlay");
  }

  async function quitApp() {
    await invoke("quit_app");
  }

  function flashFeedback(message: string) {
    feedback = message;

    if (feedbackTimer) {
      clearTimeout(feedbackTimer);
    }

    feedbackTimer = setTimeout(() => {
      feedback = "";
    }, 1800);
  }

  function removeWord(word: string) {
    if (removedWordSet.has(word)) {
      return;
    }

    removedWords = [...removedWords, word];
    removalHistory = [...removalHistory, word];
    flashFeedback(`Dikeluarkan: ${word}`);
  }

  function undoLastRemoval() {
    const lastWord = removalHistory[removalHistory.length - 1];
    if (!lastWord) {
      flashFeedback("Belum ada kata yang dikeluarkan.");
      return;
    }

    removalHistory = removalHistory.slice(0, -1);
    removedWords = removedWords.filter((word) => word !== lastWord);
    flashFeedback(`Dikembalikan: ${lastWord}`);
  }

  function removeSelected() {
    const target = visibleResults[selectedIndex] ?? visibleResults[0];
    if (!target) {
      return;
    }

    removeWord(target.word);
  }

  async function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key.toLowerCase() === "z") {
      event.preventDefault();
      undoLastRemoval();
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (visibleResults.length > 0) {
        selectedIndex = (selectedIndex + 1) % visibleResults.length;
      }
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (visibleResults.length > 0) {
        selectedIndex =
          (selectedIndex - 1 + visibleResults.length) % visibleResults.length;
      }
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      selectedIndex = 0;
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      if (visibleResults.length > 0) {
        selectedIndex = visibleResults.length - 1;
      }
      return;
    }

    if (event.key === "Enter" || event.key === "Delete") {
      event.preventDefault();
      removeSelected();
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      await hideOverlay();
    }
  }

  $effect(() => {
    query;
    void runSearch();
  });

  $effect(() => {
    const count = visibleResults.length;

    if (count === 0) {
      selectedIndex = 0;
      return;
    }

    if (selectedIndex >= count) {
      selectedIndex = count - 1;
    }
  });

  $effect(() => {
    selectedIndex;
    visibleResults.length;

    void tick().then(() => {
      document
        .querySelector<HTMLButtonElement>(".results-list button.selected")
        ?.scrollIntoView({ block: "nearest" });
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
      if (feedbackTimer) {
        clearTimeout(feedbackTimer);
      }

      stopListening?.();
    };
  });

  function formatCount(value: number) {
    return formatter.format(value);
  }

  function selectResult(index: number) {
    selectedIndex = index;
  }
</script>

<svelte:head>
  <title>Sambung Kata Overlay</title>
</svelte:head>

<div class="frame">
  <section class="shell">
    <div class="titlebar">
      <div
        class="drag-handle"
        data-tauri-drag-region
        title="Drag to move"
      >
        <span class="drag-mark" aria-hidden="true" data-tauri-drag-region>::</span>
        <div class="title-copy" data-tauri-drag-region>
          <strong data-tauri-drag-region>Sambung Kata</strong>
          <small data-tauri-drag-region>{catalog.shortcut} untuk buka / tutup</small>
        </div>
      </div>

      <div class="window-actions">
        <button type="button" class="mini ghost" onclick={hideOverlay}>Hide</button>
        <button type="button" class="mini danger" onclick={quitApp}>X</button>
      </div>
    </div>

    <section class="search-card">
      <div class="search-head">
        <div>
          <p class="mode-caption">Cari Awalan</p>
          <p class="meta-line">{summaryLine}</p>
        </div>
      </div>

      <div class="search-row">
        <input
          bind:this={inputElement}
          bind:value={query}
          autocomplete="off"
          spellcheck="false"
          placeholder={SEARCH_PLACEHOLDER}
          onkeydown={handleKeydown}
        />
        <button type="button" class="undo-button" onclick={undoLastRemoval}>
          Undo
        </button>
      </div>

      <div class="stat-row">
        <span class="chip">{formatCount(catalog.wordCount)} kata</span>
        <span class="chip">{formatCount(response?.totalMatches ?? 0)} cocok</span>
        <span class="chip">{formatCount(removedCount)} keluar</span>
        <span class="chip">
          {response?.needle ? `kunci ${response.needle.toUpperCase()}` : "belum ada kunci"}
        </span>
      </div>
    </section>

    <p class="status-strip">{statusLine}</p>

    <section class="results-panel">
      <div class="results-head">
        <span>Hasil</span>
        <span>Enter hapus | Ctrl+Z undo | Esc sembunyi</span>
      </div>

      <div class="results-list">
        {#if loading}
          <p class="empty-state">Mencari kata...</p>
        {:else if visibleResults.length > 0}
          {#each visibleResults as result, index}
            <button
              type="button"
              class:selected={selectedIndex === index}
              onclick={() => {
                selectResult(index);
                removeWord(result.word);
              }}
            >
              <span class="result-main">{result.word}</span>
              <span class="result-side">{result.length}h</span>
            </button>
          {/each}
        {:else}
          <p class="empty-state">{statusLine}</p>
        {/if}
      </div>
    </section>
  </section>
</div>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background:
      radial-gradient(circle at top, rgba(214, 164, 78, 0.18), transparent 24%),
      linear-gradient(180deg, #10181b 0%, #091013 100%);
    color: #eef3ef;
    font-family: "Bahnschrift", "Segoe UI Variable Text", "Trebuchet MS", sans-serif;
  }

  :global(button),
  :global(input) {
    font: inherit;
  }

  .frame {
    min-height: 100vh;
    padding: 8px;
    box-sizing: border-box;
  }

  .shell {
    height: calc(100vh - 16px);
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr);
    gap: 8px;
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 18px;
    background: rgba(7, 12, 14, 0.92);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 14px 30px rgba(0, 0, 0, 0.34);
    box-sizing: border-box;
  }

  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    user-select: none;
  }

  .drag-handle {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0;
    color: inherit;
    cursor: grab;
    text-align: left;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .drag-mark {
    color: #f2b453;
    font-size: 1rem;
    letter-spacing: 0.1em;
  }

  .title-copy {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .title-copy strong,
  .title-copy small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title-copy strong {
    font-size: 0.96rem;
    letter-spacing: 0.04em;
  }

  .title-copy small {
    color: rgba(238, 243, 239, 0.56);
    font-size: 0.72rem;
  }

  .window-actions {
    display: flex;
    gap: 6px;
  }

  .mini,
  .undo-button,
  .results-list button {
    border: 0;
    border-radius: 10px;
  }

  .mini,
  .undo-button,
  .results-list button {
    cursor: pointer;
    transition:
      background 120ms ease,
      transform 120ms ease;
  }

  .mini:hover,
  .undo-button:hover,
  .results-list button:hover {
    transform: translateY(-1px);
  }

  .mini {
    min-width: 42px;
    padding: 7px 10px;
    color: #eef3ef;
    font-size: 0.78rem;
  }

  .ghost {
    background: rgba(255, 255, 255, 0.08);
  }

  .danger {
    background: linear-gradient(135deg, #9c4637, #d26745);
  }

  .search-card {
    display: grid;
    gap: 8px;
    padding: 10px;
    border-radius: 14px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.02)),
      linear-gradient(135deg, rgba(239, 178, 76, 0.1), rgba(42, 126, 122, 0.12));
  }

  .search-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: flex-start;
  }

  .mode-caption,
  .meta-line,
  .status-strip,
  .results-head,
  .empty-state {
    margin: 0;
  }

  .mode-caption {
    font-size: 0.72rem;
    color: #f2b453;
    text-transform: uppercase;
    letter-spacing: 0.16em;
  }

  .meta-line {
    margin-top: 4px;
    color: rgba(238, 243, 239, 0.64);
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .search-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  .search-row input {
    width: 100%;
    min-width: 0;
    padding: 12px 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    background: rgba(0, 0, 0, 0.28);
    color: #fdfcf8;
    font-size: 1.05rem;
    outline: none;
    box-sizing: border-box;
  }

  .search-row input:focus {
    border-color: rgba(242, 180, 83, 0.45);
    box-shadow: 0 0 0 1px rgba(242, 180, 83, 0.2);
  }

  .search-row input::placeholder {
    color: rgba(238, 243, 239, 0.34);
  }

  .undo-button {
    padding: 0 14px;
    background: linear-gradient(135deg, #e2a63e, #cf7e3a);
    color: #10181b;
    font-size: 0.82rem;
    font-weight: 700;
  }

  .stat-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    padding: 5px 9px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.07);
    color: rgba(238, 243, 239, 0.76);
    font-size: 0.72rem;
    line-height: 1;
  }

  .status-strip {
    padding: 8px 10px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    color: rgba(238, 243, 239, 0.74);
    font-size: 0.78rem;
    line-height: 1.35;
  }

  .results-panel {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 6px;
    padding: 8px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.03);
  }

  .results-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 2px 0;
    color: rgba(238, 243, 239, 0.52);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .results-list {
    min-height: 0;
    display: grid;
    align-content: start;
    gap: 6px;
    overflow: auto;
    padding-right: 2px;
  }

  .results-list button {
    width: 100%;
    padding: 10px 12px;
    background: rgba(0, 0, 0, 0.26);
    color: #f3f7f3;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    text-align: left;
  }

  .results-list button.selected {
    background: linear-gradient(135deg, rgba(242, 180, 83, 0.18), rgba(42, 126, 122, 0.22));
    box-shadow: inset 0 0 0 1px rgba(242, 180, 83, 0.26);
  }

  .result-main {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.96rem;
    font-weight: 700;
  }

  .result-side {
    color: rgba(238, 243, 239, 0.56);
    font-size: 0.74rem;
    white-space: nowrap;
  }

  .empty-state {
    padding: 14px 6px;
    color: rgba(238, 243, 239, 0.58);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  @media (max-width: 420px) {
    .search-head,
    .results-head {
      flex-direction: column;
    }

    .window-actions {
      flex-shrink: 0;
    }
  }
</style>
