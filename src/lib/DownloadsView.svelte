<script lang="ts">
  import { baseName, formatProgress, isArchive, orderEntries } from "./download";
  import { cancelDownload, clearFinishedDownloads, downloads } from "./download.svelte";
  import { extractDownload, extraction } from "./extract.svelte";
  import type { TopicSummary } from "./types";

  let { onselect }: { onselect: (topic: TopicSummary) => void } = $props();

  const entries = $derived(orderEntries(downloads.entries));
  const finished = $derived(entries.some((e) => e.state !== "running"));
</script>

<div class="view">
  <header>
    <h2>Downloads</h2>
    <button type="button" class="subtle" onclick={clearFinishedDownloads} disabled={!finished}>Clear finished</button>
  </header>

  {#if !entries.length}
    <p class="empty">Downloads from this session show up here.</p>
  {:else}
    <ul>
      {#each entries as e (e.id)}
        <li>
          <div class="main">
            <strong>{e.path ? baseName(e.path) : `${e.label} · ${e.host}`}</strong>
            {#if e.topic}
              <button type="button" class="game" onclick={() => e.topic && onselect(e.topic)}>{e.topic.name}</button>
            {/if}
            {#if e.state === "running"}
              <span class="detail">
                {e.via ? "via JDownloader · " : ""}{formatProgress(e.progress)}
                {#if e.progress.total}<progress max={e.progress.total} value={e.progress.received}></progress>{/if}
              </span>
            {:else if e.state === "done"}
              <span class="detail path">{e.path}</span>
            {:else if e.state === "handed_off"}
              <span class="detail">Sent to JDownloader; progress shows there</span>
            {:else if e.state === "cancelled"}
              <span class="detail">Cancelled</span>
            {:else}
              <span class="detail error">{e.error}</span>
            {/if}
          </div>
          <div class="actions">
            {#if e.state === "running"}
              <button type="button" onclick={() => cancelDownload(e.url)}>Cancel</button>
            {:else if e.state === "done" && e.path && isArchive(e.path)}
              {@const path = e.path}
              <button type="button" onclick={() => extractDownload(path)} disabled={extraction.running}>
                {extraction.running ? "Extracting…" : "Extract"}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .view {
    padding: 20px 24px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border, #444);
    border-radius: 8px;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .detail {
    font-size: 12px;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
    display: flex;
    gap: 8px;
    align-items: center;
    overflow-wrap: anywhere;
  }
  .error {
    color: var(--warn);
    opacity: 1;
  }
  .game {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: none;
    font-size: 12px;
    text-decoration: underline;
    cursor: pointer;
    color: inherit;
  }
  button {
    padding: 3px 10px;
    font-size: 12px;
  }
  .empty {
    opacity: 0.7;
  }
</style>
