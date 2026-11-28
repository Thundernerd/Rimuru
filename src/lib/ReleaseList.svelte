<script lang="ts">
  import ReleaseCard from "./ReleaseCard.svelte";
  import type { Release, TopicSummary } from "./types";

  let {
    releases,
    topicId,
    topic = null,
    newIds,
    pagesRead,
    pagesPlanned,
    totalPages,
    loading,
    error,
    onretry,
    canLoadOlder = false,
    loadingOlder = false,
    onloadolder = () => {},
  }: {
    releases: Release[];
    topicId: number;
    topic?: TopicSummary | null;
    newIds: Set<number>;
    pagesRead: number;
    pagesPlanned: number;
    totalPages: number | null;
    loading: boolean;
    error: string | null;
    onretry: () => void;
    canLoadOlder?: boolean;
    loadingOlder?: boolean;
    onloadolder?: () => void;
  } = $props();
</script>

<section>
  <h3>
    All releases <span class="muted">({releases.length})</span>
    {#if newIds.size}<span class="new-count">{newIds.size} new since your last visit</span>{/if}
  </h3>
  {#if totalPages && totalPages > 1}
    <p class="muted small">
      {#if loading}
        Reading the most recent pages… {pagesRead} of {pagesPlanned}
      {:else}
        Read {pagesRead} of {totalPages} pages.
      {/if}
    </p>
  {/if}

  {#each releases as release (release.post_id)}
    <ReleaseCard {release} {topicId} {topic} isNew={newIds.has(release.post_id)} />
  {:else}
    {#if !loading && !error}
      <p class="muted">No posts with download links on the pages read.</p>
    {/if}
  {/each}

  {#if loading || loadingOlder}
    <div class="placeholder"></div>
  {/if}
  {#if canLoadOlder && !loading}
    <div class="older">
      <button type="button" onclick={onloadolder} disabled={loadingOlder}>
        {loadingOlder ? "Loading older releases…" : "Load older releases"}
      </button>
    </div>
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
  .new-count {
    margin-left: 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }
  .muted {
    color: var(--muted);
  }
  .small {
    margin: -6px 0 0;
    font-size: 12px;
  }
  .older {
    display: flex;
    justify-content: center;
    padding: 8px 0;
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
