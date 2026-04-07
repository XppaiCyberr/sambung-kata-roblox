<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";

  type CatalogInfo = { shortcut: string; wordCount: number };
  type WordlistInfo = { name: string; rawLines: number };
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
  let appReady = $state(false);
  let loadProgress = $state(0);
  let loadStatus = $state("Initializing...");
  let query = $state("");
  let loading = $state(false);
  let errorMessage = $state("");
  let feedback = $state("");
  let selectedIndex = $state(0);
  let removedWords = $state<string[]>([]);
  let removalHistory = $state<string[]>([]);
  let response = $state<SearchResponse | null>(null);
  let catalog = $state<CatalogInfo>({ shortcut: "Space", wordCount: 0 });
  let wordlists = $state<WordlistInfo[]>([]);
  let typingSpeed = $state(50); // milliseconds per character
  let showSpeedDialog = $state(false);
  let speedDialogInput = $state(""); // temp input for dialog

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

  function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function loadApp() {
    loadStatus = "Loading word index...";
    loadProgress = 20;
    await sleep(300);

    try {
      catalog = await invoke<CatalogInfo>("catalog_info");
      loadProgress = 50;
      loadStatus = "Fetching wordlist info...";
      await sleep(200);

      wordlists = await invoke<WordlistInfo[]>("wordlist_info");
      loadProgress = 75;
      loadStatus = `${formatter.format(catalog.wordCount)} words loaded`;
      await sleep(300);

      loadProgress = 100;
      loadStatus = "Ready";
      await sleep(400);

      appReady = true;
      await focusSearch(true);
    } catch (error) {
      errorMessage = `Failed to load: ${String(error)}`;
      appReady = true;
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
    if (event.key === "Enter") {
      event.preventDefault();
      if (selectedWord) {
        void handleEnter();
      }
      return;
    }

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
    if (event.key === "Delete") {
      event.preventDefault();
      removeSelected();
    }

    if (event.ctrlKey && event.key.toLowerCase() === "t") {
      event.preventDefault();
      showSpeedDialog = true;
      speedDialogInput = typingSpeed.toString();
      return;
    }
  }

  async function handleEnter() {
    if (!selectedWord) return;

    try {
      // TODO: typingSpeed will be added in Task 4
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

  function saveTypingSpeed(speed: number) {
    if (speed >= 10 && speed <= 500) {
      localStorage.setItem("typing_speed_ms", speed.toString());
      typingSpeed = speed;
    }
  }

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
    // Load typing speed from localStorage
    const savedSpeed = localStorage.getItem("typing_speed_ms");
    if (savedSpeed) {
      const parsed = parseInt(savedSpeed, 10);
      if (!isNaN(parsed) && parsed >= 10 && parsed <= 500) {
        typingSpeed = parsed;
      }
    }

    let stopListening: (() => void) | undefined;
    void loadApp();
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

{#if !appReady}
<div class="splash">
  <pre class="ascii-logo">██╗  ██╗██████╗ ██████╗  █████╗ ██╗ ██████╗██╗   ██╗██████╗ ███████╗██████╗{"\n"}╚██╗██╔╝██╔══██╗██╔══██╗██╔══██╗██║██╔════╝╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗{"\n"} ╚███╔╝ ██████╔╝██████╔╝███████║██║██║      ╚████╔╝ ██████╔╝█████╗  ██████╔╝{"\n"} ██╔██╗ ██╔═══╝ ██╔═══╝ ██╔══██║██║██║       ╚██╔╝  ██╔══██╗██╔══╝  ██╔══██╗{"\n"}██╔╝ ██╗██║     ██║     ██║  ██║██║╚██████╗   ██║   ██████╔╝███████╗██║  ██║{"\n"}╚═╝  ╚═╝╚═╝     ╚═╝     ╚═╝  ╚═╝╚═╝ ╚═════╝   ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝</pre>
  <div class="splash-sub">SambungKata VIP</div>
  <div class="progress-track">
    <div class="progress-fill" style:width="{loadProgress}%"></div>
  </div>
  <div class="splash-status">{loadStatus}</div>
</div>
{:else}
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
    {#if wordlists.length > 0 && !query}
      <div class="wordlist-tags">
        {#each wordlists as wl}
          <span class="wl-tag">{wl.name} <small>({formatter.format(wl.rawLines)})</small></span>
        {/each}
      </div>
    {/if}
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
          {#if response?.needle && result.word.toLowerCase().startsWith(response.needle)}
            <strong class="prefix">{result.word.slice(0, response.needle.length)}</strong>{result.word.slice(response.needle.length)}
          {:else}
            {result.word}
          {/if}
        </button>
      {/each}
    {:else if query}
      <p class="empty">No matches</p>
    {/if}
  </div>

  <div class="hint-bar">
    <span>Esc hide</span>
    <span>Enter remove</span>
    <span>^Z undo</span>
  </div>
</div>
{/if}

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

  .splash {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 20px;
    box-sizing: border-box;
    overflow: hidden;
  }
  .ascii-logo {
    margin: 0;
    font-family: "Consolas", "Courier New", monospace;
    font-size: 0.38rem;
    line-height: 1.2;
    white-space: pre;
    background: linear-gradient(180deg, #a38adf, #7ec8e3);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    text-align: center;
  }
  .splash-sub {
    font-size: 0.8rem;
    color: #9ea6c0;
    letter-spacing: 0.15em;
    text-transform: uppercase;
  }
  .progress-track {
    width: 200px;
    height: 3px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    border-radius: 2px;
    background: linear-gradient(90deg, #a38adf, #7ec8e3);
    transition: width 0.4s ease;
  }
  .splash-status {
    font-size: 0.7rem;
    color: rgba(158, 166, 192, 0.6);
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
    letter-spacing: 0.02em;
    background: linear-gradient(90deg, #a38adf, #7ec8e3, #cbb7ff, #a38adf);
    background-size: 200% 100%;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: shimmer 3s linear infinite;
  }
  @keyframes shimmer {
    0% { background-position: 100% 0; }
    100% { background-position: -100% 0; }
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
  .wordlist-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 6px 0 0;
  }
  .wl-tag {
    padding: 2px 8px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.05);
    font-size: 0.68rem;
    color: #9ea6c0;
  }
  .wl-tag small {
    color: rgba(158, 166, 192, 0.6);
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
  .word-row .prefix {
    color: #a38adf;
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
