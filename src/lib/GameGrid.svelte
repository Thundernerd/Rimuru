<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage, MAIN_FORUM_ID } from "./api";
  import GameCard from "./GameCard.svelte";
  import { setStatus } from "./status.svelte";
  import type { TopicSummary } from "./types";

  let { onselect }: { onselect: (topic: TopicSummary) => void } = $props();

  let topics = $state<TopicSummary[]>([]);
  /** `start=` of the next forum page to load; null once the last is loaded. */
  let nextStart = $state<number | null>(0);
  /** `start=` of the most recently loaded page, for saving snapshots. */
  let lastStart = $state(0);
  let pagesLoaded = $state(0);
  let totalPages = $state<number | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let filter = $state("");
  let sentinel = $state<HTMLElement>();

  const shown = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    return q ? topics.filter((t) => t.title.toLowerCase().includes(q)) : topics;
  });

  /** Loads the next forum page, one at a time. */
  async function loadMore() {
    if (loading || nextStart === null) return;
    loading = true;
    error = null;
    const start = nextStart;
    try {
      const page = await api.forumPage(MAIN_FORUM_ID, start);
      // New posts move topics between pages, so a later page can repeat some.
      const seen = new Set(topics.map((t) => t.id));
      topics.push(...page.topics.filter((t) => !seen.has(t.id)));
      nextStart = page.pagination.next_start;
      lastStart = start;
      pagesLoaded += 1;
      totalPages = page.pagination.total_pages;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  async function saveSnapshot() {
    try {
      setStatus(`Saved ${await api.saveSnapshot(`viewforum.php?f=${MAIN_FORUM_ID}&start=${lastStart}`)}`);
    } catch (e) {
      setStatus(errorMessage(e));
    }
  }

  onMount(loadMore);

  // Load the next page as the end of the grid scrolls into view.
  $effect(() => {
    if (!sentinel) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) loadMore();
      },
      { rootMargin: "800px" },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  });
</script>

<div class="toolbar">
  <input type="search" placeholder="Filter loaded games" bind:value={filter} />
  <span class="count">
    {topics.length} games
    {#if totalPages}· {pagesLoaded} of {totalPages} pages{/if}
  </span>
  <button type="button" class="subtle" onclick={saveSnapshot} disabled={!pagesLoaded}>Save HTML</button>
</div>

<div class="grid">
  {#each shown as topic (topic.id)}
    <GameCard {topic} {onselect} />
  {/each}
  {#if loading}
    {#each { length: topics.length ? 4 : 12 } as _, i (i)}
      <div class="placeholder"></div>
    {/each}
  {/if}
</div>

{#if error}
  <div class="message">
    <p>Couldn't load the game list: {error}</p>
    <button type="button" onclick={loadMore}>Try again</button>
  </div>
{:else if !loading && nextStart !== null}
  <div class="message">
    <button type="button" onclick={loadMore}>Load more</button>
  </div>
{/if}
<div bind:this={sentinel}></div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }
  input {
    flex: 0 1 320px;
  }
  .count {
    color: var(--muted);
    font-size: 12px;
    margin-right: auto;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 16px;
  }

  .placeholder {
    aspect-ratio: 2 / 3;
    border-radius: 10px;
    background: var(--surface);
    animation: pulse 1.2s ease-in-out infinite alternate;
  }
  @keyframes pulse {
    from {
      opacity: 0.4;
    }
    to {
      opacity: 0.9;
    }
  }

  .message {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 24px;
    color: var(--muted);
  }
</style>
