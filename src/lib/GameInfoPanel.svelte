<script lang="ts">
  import LinkButton from "./LinkButton.svelte";
  import type { GameInfo, SteamApp, TopicSummary } from "./types";

  /**
   * A game's details, from Steam where available (it's more current than
   * the forum's copy) and from the topic's first post otherwise.
   */
  let {
    topic,
    forum,
    steam,
    loading,
  }: { topic: TopicSummary; forum: GameInfo | null; steam: SteamApp | null; loading: boolean } = $props();

  const PLATFORM_NAMES: Record<string, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };
  const list = (values: string[] | undefined) => (values?.length ? values.join(", ") : null);

  const header = $derived(steam?.header_image ?? forum?.header_image ?? null);
  const title = $derived(steam?.name || forum?.title || topic.name);
  const appId = $derived(steam?.app_id ?? forum?.steam_app_id ?? null);
  const tags = $derived([...topic.prefixes.filter((p) => p.toLowerCase() !== "info"), ...topic.labels]);

  const facts = $derived(
    (
      [
        ["Developer", list(steam?.developers) ?? forum?.developer],
        ["Publisher", list(steam?.publishers) ?? forum?.publisher],
        [
          "Release date",
          steam?.release_date ? steam.release_date + (steam.coming_soon ? " (coming soon)" : "") : forum?.release_date,
        ],
        ["Genres", list(steam?.genres) ?? list(forum?.genres)],
        ["Platforms", list((steam?.platforms ?? forum?.platforms)?.map((p) => PLATFORM_NAMES[p] ?? p))],
        ["Price", steam ? (steam.is_free ? "Free" : steam.price) : null],
        ["Metacritic", steam?.metacritic?.toString()],
        ["Languages", list(steam?.languages) ?? list(forum?.languages)],
        ...(forum?.fields.map((f) => [f.key, f.value]) ?? []),
      ] as [string, string | null | undefined][]
    ).filter((row): row is [string, string] => !!row[1]),
  );
  const features = $derived(forum?.features.length ? forum.features : (steam?.categories ?? []));

  let viewing = $state<string | null>(null);
</script>

<section class="panel">
  {#if header}
    <img class="header" src={header} alt="" />
  {:else}
    <div class="header placeholder"></div>
  {/if}

  <h2>{title}</h2>
  {#if tags.length}
    <div class="tags">{#each tags as tag (tag)}<span class="tag">{tag}</span>{/each}</div>
  {/if}

  {#if steam?.short_description}
    <p class="summary">{steam.short_description}</p>
  {/if}

  <div class="links">
    {#if appId}
      <LinkButton url={steam?.store_url ?? `https://store.steampowered.com/app/${appId}`} label="Steam store" />
      <LinkButton url={forum?.steamdb_url ?? `https://steamdb.info/app/${appId}`} label="SteamDB" />
    {/if}
    <LinkButton url={topic.url} label="Forum topic" />
    {#if steam?.website}<LinkButton url={steam.website} label="Website" />{/if}
  </div>

  {#if facts.length}
    <dl>
      {#each facts as [key, value] (key)}
        <dt>{key}</dt>
        <dd>{value}</dd>
      {/each}
    </dl>
  {:else if loading}
    <p class="muted">Loading details…</p>
  {:else}
    <p class="muted">No game details in this topic's first post, and no Steam match.</p>
  {/if}

  {#if steam?.screenshots.length}
    <div class="shots">
      {#each steam.screenshots as shot (shot.full)}
        <button type="button" onclick={() => (viewing = shot.full)}>
          <img src={shot.thumbnail} alt="" loading="lazy" />
        </button>
      {/each}
    </div>
  {/if}

  {#if forum?.description}
    <details>
      <summary>About</summary>
      <p class="text">{forum.description}</p>
    </details>
  {/if}
  {#if features.length}
    <details>
      <summary>Features</summary>
      <ul>{#each features as f (f)}<li>{f}</li>{/each}</ul>
    </details>
  {/if}
  {#if forum?.system_requirements.length}
    <details>
      <summary>System requirements</summary>
      {#each forum.system_requirements as set, i (i)}
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

  {#if steam}
    <p class="source">Details from Steam where available, otherwise from the forum.</p>
  {/if}
</section>

{#if viewing}
  <button type="button" class="lightbox" onclick={() => (viewing = null)} aria-label="Close screenshot">
    <img src={viewing} alt="" />
  </button>
{/if}

<svelte:window onkeydown={(e) => e.key === "Escape" && (viewing = null)} />

<style>
  .header {
    width: 100%;
    aspect-ratio: 460 / 215;
    object-fit: cover;
    border-radius: 10px;
    display: block;
  }
  .header.placeholder {
    background: var(--surface);
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
  .summary {
    margin: 10px 0 0;
    line-height: 1.45;
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

  .shots {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 6px;
    margin-bottom: 8px;
  }
  .shots button {
    padding: 0;
    border: none;
    flex: 0 0 auto;
    background: none;
  }
  .shots img {
    height: 72px;
    border-radius: 4px;
    display: block;
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
  .muted,
  .source {
    color: var(--muted);
  }
  .source {
    font-size: 11px;
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: grid;
    place-items: center;
    padding: 32px;
    border: none;
    border-radius: 0;
    background: rgb(0 0 0 / 0.85);
    cursor: zoom-out;
  }
  .lightbox:hover {
    background: rgb(0 0 0 / 0.85);
  }
  .lightbox img {
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
  }
</style>
