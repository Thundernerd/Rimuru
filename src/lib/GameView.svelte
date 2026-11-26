<script lang="ts">
  import { api, errorMessage } from "./api";
  import LinkButton from "./LinkButton.svelte";
  import ReleaseCard from "./ReleaseCard.svelte";
  import { setStatus } from "./status.svelte";
  import type { GameDetail, TopicSummary } from "./types";

  let { topic, onback }: { topic: TopicSummary; onback: () => void } = $props();

  const PLATFORM_NAMES: Record<string, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };

  let detail = $state<GameDetail | null>(null);
  let error = $state<string | null>(null);
  let attempt = $state(0);

  // Load whenever the topic changes (or on retry); drop stale responses.
  $effect(() => {
    const id = topic.id;
    void attempt;
    let current = true;
    detail = null;
    error = null;
    api
      .game(id)
      .then((d) => current && (detail = d))
      .catch((e) => current && (error = errorMessage(e)));
    return () => {
      current = false;
    };
  });

  const info = $derived(detail?.info ?? null);
  const facts = $derived(
    info
      ? [
          ["Developer", info.developer],
          ["Publisher", info.publisher],
          ["Release date", info.release_date],
          ["Genres", info.genres.join(", ")],
          ["Platforms", info.platforms.map((p) => PLATFORM_NAMES[p] ?? p).join(", ")],
          ["Languages", info.languages.join(", ")],
          ...info.fields.map((f) => [f.key, f.value]),
        ].filter((row): row is [string, string] => !!row[1])
      : [],
  );
  const tags = $derived([...topic.prefixes.filter((p) => p.toLowerCase() !== "info"), ...topic.labels]);

  async function saveSnapshot() {
    try {
      setStatus(`Saved ${await api.saveSnapshot(`viewtopic.php?t=${topic.id}&start=0`)}`);
    } catch (e) {
      setStatus(errorMessage(e));
    }
  }
</script>

<div class="toolbar">
  <button type="button" onclick={onback}>‹ All games</button>
  <button type="button" class="subtle" onclick={saveSnapshot}>Save HTML</button>
</div>

<div class="layout">
  <section class="info">
    {#if info?.header_image}
      <img class="header" src={info.header_image} alt="" />
    {/if}
    <h2>{info?.title ?? topic.name}</h2>
    {#if tags.length}
      <div class="tags">{#each tags as tag (tag)}<span class="tag">{tag}</span>{/each}</div>
    {/if}

    {#if info}
      <div class="links">
        {#if info.store_url}<LinkButton url={info.store_url} label="Steam store" />{/if}
        {#if info.steamdb_url}<LinkButton url={info.steamdb_url} label="SteamDB" />{/if}
        <LinkButton url={topic.url} label="Forum topic" />
      </div>
      <dl>
        {#each facts as [key, value] (key)}
          <dt>{key}</dt>
          <dd>{value}</dd>
        {/each}
      </dl>
      {#if info.description}
        <details>
          <summary>About</summary>
          <p class="text">{info.description}</p>
        </details>
      {/if}
      {#if info.features.length}
        <details>
          <summary>Features</summary>
          <ul>{#each info.features as f (f)}<li>{f}</li>{/each}</ul>
        </details>
      {/if}
      {#if info.system_requirements.length}
        <details>
          <summary>System requirements</summary>
          {#each info.system_requirements as set, i (i)}
            {#if set.label}<h4>{set.label}</h4>{/if}
            <dl>
              {#each set.items as item, j (j)}
                <dt>{item.key}</dt>
                <dd>{item.value}</dd>
              {/each}
            </dl>
          {/each}
        </details>
      {/if}
    {:else if detail}
      <p class="muted">This topic's first post has no game details.</p>
      <div class="links"><LinkButton url={topic.url} label="Forum topic" /></div>
    {/if}
  </section>

  <section class="releases">
    {#if error}
      <div class="message">
        <p>Couldn't load this topic: {error}</p>
        <button type="button" onclick={() => attempt++}>Try again</button>
      </div>
    {:else if !detail}
      <p class="muted">Loading releases…</p>
    {:else}
      <h3>
        Releases <span class="muted">({detail.releases.length})</span>
      </h3>
      {#if detail.total_pages && detail.pages_read.length < detail.total_pages}
        <p class="muted small">
          Read {detail.pages_read.length} of {detail.total_pages} pages: the first and the most recent.
        </p>
      {/if}
      {#each detail.releases as release (release.post_id)}
        <ReleaseCard {release} />
      {:else}
        <p class="muted">No posts with download links on the pages read.</p>
      {/each}
    {/if}
  </section>
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(280px, 380px) 1fr;
    gap: 24px;
    align-items: start;
  }
  @media (max-width: 800px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .info {
    position: sticky;
    top: 0;
  }
  .header {
    width: 100%;
    border-radius: 10px;
    display: block;
  }
  h2 {
    margin: 12px 0 6px;
    font-size: 20px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 12px 0;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 12px;
    margin: 0 0 12px;
    font-size: 13px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  details {
    border-top: 1px solid var(--border);
    padding: 8px 0;
  }
  summary {
    cursor: pointer;
    font-weight: 600;
  }
  h4 {
    margin: 8px 0 4px;
    font-size: 13px;
  }
  .text {
    white-space: pre-wrap;
    font-size: 13px;
  }
  ul {
    margin: 6px 0 0;
    padding-left: 18px;
    font-size: 13px;
  }

  .releases {
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
  .message {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
</style>
