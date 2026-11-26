<script lang="ts">
  import ReleaseCard from "./ReleaseCard.svelte";
  import type { Release } from "./types";

  let {
    releases,
    pagesRead,
    pagesPlanned,
    totalPages,
    loading,
    error,
    onretry,
  }: {
    releases: Release[];
    pagesRead: number;
    pagesPlanned: number;
    totalPages: number | null;
    loading: boolean;
    error: string | null;
    onretry: () => void;
  } = $props();
</script>

<section>
  <h3>Releases <span class="muted">({releases.length})</span></h3>
  {#if totalPages && totalPages > 1}
    <p class="muted small">
      {#if loading}
        Reading the most recent pages… {pagesRead} of {pagesPlanned}
      {:else}
        Read {pagesRead} of {totalPages} pages: the first and the most recent.
      {/if}
    </p>
  {/if}

  {#each releases as release (release.post_id)}
    <ReleaseCard {release} />
  {:else}
    {#if !loading && !error}
      <p class="muted">No posts with download links on the pages read.</p>
    {/if}
  {/each}

  {#if loading}
    <div class="placeholder"></div>
  {/if}
  {#if error}
    <div class="message">
      <p>Couldn't load all releases: {error}</p>
      <button type="button" onclick={onretry}>Try again</button>
    </div>
  {/if}
</section>

<style>
  section {
    display: grid;
    gap: 12px;
  }
  h3 {
    margin: 0;
    font-size: 16px;
  }
  .muted {
    color: var(--muted);
  }
  .small {
    margin: -6px 0 0;
    font-size: 12px;
  }
  .placeholder {
    height: 120px;
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
    align-items: flex-start;
    gap: 8px;
  }
</style>
