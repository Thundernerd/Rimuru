<script lang="ts">
  import Icon from "./Icon.svelte";
  import { canGoBack, canGoForward } from "./nav.svelte";

  /**
   * The bar above the content: back/forward, and the forum search
   * (⌘K / Ctrl K). Its empty space drags the window.
   */
  let {
    onback,
    onforward,
    onsearch,
    mac,
    input = $bindable(),
  }: {
    onback: () => void;
    onforward: () => void;
    onsearch: (query: string, allForums: boolean) => void;
    mac: boolean;
    input?: HTMLInputElement;
  } = $props();

  let text = $state("");
  let allForums = $state(false);

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (text.trim()) onsearch(text.trim(), allForums);
  }
</script>

<header data-tauri-drag-region>
  <div class="history">
    <button type="button" class="subtle" onclick={onback} disabled={!canGoBack()} aria-label="Back" title="Back">
      <Icon name="back" />
    </button>
    <button
      type="button"
      class="subtle"
      onclick={onforward}
      disabled={!canGoForward()}
      aria-label="Forward"
      title="Forward"
    >
      <Icon name="forward" />
    </button>
  </div>

  <form class="search" onsubmit={submit}>
    <Icon name="search" size={16} />
    <input
      bind:this={input}
      bind:value={text}
      type="search"
      placeholder={allForums ? "Search all forums" : "Search games"}
      aria-label="Search games"
    />
    <button
      type="button"
      class="scope"
      class:on={allForums}
      onclick={() => (allForums = !allForums)}
      title="Search topic titles in all forums, not just the main game forum"
    >
      All forums
    </button>
    <kbd>{mac ? "⌘" : "Ctrl"} K</kbd>
  </form>

  <div class="spacer" data-tauri-drag-region></div>
</header>

<style>
  header {
    display: grid;
    grid-template-columns: 1fr minmax(280px, 560px) 1fr;
    align-items: center;
    gap: 16px;
    height: 56px;
    padding: 0 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    user-select: none;
  }
  .history {
    display: flex;
    gap: 2px;
  }
  .history button {
    display: grid;
    place-items: center;
    padding: 6px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 6px 0 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
  }
  .search:focus-within {
    border-color: rgb(63 200 240 / 0.6);
  }
  /* WebKit draws search fields natively and clips text to the content box,
     so with no padding the descenders of p/y/g/j were cut off. Plain field,
     full height of the bar, and a line height with room for them. */
  input {
    flex: 1;
    min-width: 0;
    align-self: stretch;
    height: 100%;
    padding: 0 2px;
    border: none;
    background: none;
    outline: none;
    line-height: 20px;
    -webkit-appearance: none;
    appearance: none;
  }
  input:focus-visible {
    outline: none;
  }
  .scope {
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    color: var(--muted);
    background: none;
    white-space: nowrap;
  }
  .scope.on {
    color: var(--accent);
    border-color: rgb(63 200 240 / 0.5);
    background: var(--accent-soft);
  }
  kbd {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 5px;
    font-family: var(--mono);
    font-size: 11px;
    white-space: nowrap;
  }
  .spacer {
    height: 100%;
  }
</style>
