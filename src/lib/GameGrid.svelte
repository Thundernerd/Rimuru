<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage } from "./api";
  import GameCard from "./GameCard.svelte";
  import { setStatus } from "./status.svelte";
  import { loadVisits, rememberListed } from "./visits.svelte";
  import type { Listing, TopicSummary } from "./types";

  /**
   * A grid of game cards that loads its listing a page at a time as you
   * scroll. `fetchPage` gets a page by `start=` offset; `snapshotPath`
   * gives the forum path of a page, for "Save HTML".
   */
  let {
    onselect,
    fetchPage,
    snapshotPath,
    heading = null,
    emptyText = "No games here.",
    errorText = "Couldn't load the game list",
    showSnapshot = true,
    filter = null,
  }: {
    onselect: (topic: TopicSummary) => void;
    fetchPage: (start: number) => Promise<Listing>;
    snapshotPath: (start: number) => string;
    heading?: string | null;
    emptyText?: string;
    errorText?: string;
    /** Show "Save HTML" (not for lists that aren't a forum page). */
    showSnapshot?: boolean;
    /** Only show topics this accepts (pages still load in full). */
    filter?: ((topic: TopicSummary) => boolean) | null;
  } = $props();

  /** Pages loaded in a row by themselves when a filter leaves the screen empty. */
  const MAX_AUTO_LOADS = 5;
  let autoLoads = 0;

  let topics = $state<TopicSummary[]>([]);
  /** `start=` of the next forum page to load; null once the last is loaded. */
  let nextStart = $state<number | null>(0);
  /** `start=` of the most recently loaded page, for saving snapshots. */
  let lastStart = $state(0);
  let pagesLoaded = $state(0);
  let totalPages = $state<number | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let totalMatches = $state<number | null>(null);
  let sentinel = $state<HTMLElement>();

  const shown = $derived(filter ? topics.filter(filter) : topics);

  /** The end of the grid is (nearly) on screen. */
  function nearEnd(): boolean {
    const rect = sentinel?.getBoundingClientRect();
    return !!rect && rect.top < window.innerHeight + 800;
  }

  /** Loads the next page, one at a time. */
  async function loadMore() {
    if (loading || nextStart === null) return;
    loading = true;
    error = null;
    const start = nextStart;
    try {
      const page = await fetchPage(start);
      // New posts move topics between pages, so a later page can repeat some.
      const seen = new Set(topics.map((t) => t.id));
      topics.push(...page.topics.filter((t) => !seen.has(t.id)));
      loadVisits(page.topics.map((t) => t.id));
      rememberListed(page.topics);
      nextStart = page.pagination.next_start;
      lastStart = start;
      pagesLoaded += 1;
      totalPages = page.pagination.total_pages;
      totalMatches = page.total_matches ?? null;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loading = false;
    }
    // A filter can hide most of a page, leaving the end of the grid in view
    // without the observer firing again: keep loading, a few pages at most.
    if (filter && !error && nextStart !== null && autoLoads < MAX_AUTO_LOADS) {
      await Promise.resolve();
      if (nearEnd()) {
        autoLoads++;
        loadMore();
        return;
      }
    }
    autoLoads = 0;
  }

  async function saveSnapshot() {
    try {
      setStatus(`Saved ${await api.saveSnapshot(snapshotPath(lastStart))}`);
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
  {#if heading}<h2>{heading}</h2>{/if}
  <span class="count">
    {#if totalMatches !== null}{totalMatches} matches ·{/if}
    {#if filter}{shown.length} of {topics.length} games{:else}{topics.length} games{/if}
    {#if totalPages}· {pagesLoaded} of {totalPages} pages{/if}
  </span>
  {#if showSnapshot}
    <button type="button" class="subtle" onclick={saveSnapshot} disabled={!pagesLoaded}>Save HTML</button>
  {/if}
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
    <p>{errorText}: {error}</p>
    <button type="button" onclick={loadMore}>Try again</button>
  </div>
{:else if !loading && pagesLoaded && !shown.length && nextStart === null}
  <div class="message"><p>{emptyText}</p></div>
{:else if !loading && nextStart !== null}
  <div class="message">
    {#if pagesLoaded && !shown.length}<p>{emptyText}</p>{/if}
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
  h2 {
    margin: 0;
    font-size: 16px;
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
