<script lang="ts">
  import LinkButton from "./LinkButton.svelte";
  import TrailerPlayer from "./TrailerPlayer.svelte";
  import type { Artwork, GameInfo, Release, SteamApp, TopicSummary } from "./types";

  /**
   * The top of a game's page: key art behind the game's cover, title and
   * facts, and the latest release with its download links.
   */
  let {
    topic,
    forum,
    steam,
    art,
    latest,
    latestIsNew,
    releaseCount,
    loadingReleases,
    onback,
    onsnapshot,
    onallreleases,
  }: {
    topic: TopicSummary;
    forum: GameInfo | null;
    steam: SteamApp | null;
    art: Artwork | null;
    latest: Release | null;
    /** The latest release wasn't there on the previous visit. */
    latestIsNew: boolean;
    releaseCount: number;
    loadingReleases: boolean;
    onback: () => void;
    onsnapshot: () => void;
    onallreleases: () => void;
  } = $props();

  const PLATFORM_NAMES: Record<string, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };

  const title = $derived(steam?.name || forum?.title || topic.name);
  const backdrop = $derived(
    art?.hero ?? steam?.screenshots[0]?.full ?? steam?.header_image ?? forum?.header_image ?? null,
  );
  const cover = $derived(art?.portrait ?? null);
  const header = $derived(steam?.header_image ?? forum?.header_image ?? art?.header ?? null);
  const appId = $derived(steam?.app_id ?? forum?.steam_app_id ?? null);

  const labels = $derived(topic.labels);
  const facts = $derived(
    [
      (steam?.platforms ?? forum?.platforms)?.map((p) => PLATFORM_NAMES[p] ?? p).join(" · "),
      steam?.release_date
        ? steam.release_date + (steam.coming_soon ? " (coming soon)" : "")
        : forum?.release_date,
      steam ? (steam.is_free ? "Free" : steam.price) : null,
      steam?.metacritic ? `Metacritic ${steam.metacritic}` : null,
      topic.replies !== null ? `${topic.replies} replies` : null,
      topic.last_post_at ? `Updated ${topic.last_post_at}` : null,
    ].filter((f): f is string => !!f),
  );
  const people = $derived(
    [
      ["Developer", steam?.developers.join(", ") || forum?.developer],
      ["Publisher", steam?.publishers.join(", ") || forum?.publisher],
    ].filter((p): p is [string, string] => !!p[1]),
  );
  const genres = $derived(steam?.genres.length ? steam.genres : (forum?.genres ?? []));
  const summary = $derived(steam?.short_description ?? null);

  const latestFacts = $derived(
    latest
      ? [
          latest.version &&
            (/^v?\d+(\.\d+)+/i.test(latest.version) ? `v${latest.version.replace(/^v/i, "")}` : latest.version),
          latest.build && `Build ${latest.build}`,
          latest.size,
        ].filter((f): f is string => !!f)
      : [],
  );

  let backdropLoaded = $state(false);
  let showTrailer = $state(false);
  const movies = $derived(steam?.movies ?? []);
</script>

<section class="hero">
  {#if backdrop}
    {#key backdrop}
      <img
        class="backdrop"
        class:loaded={backdropLoaded}
        src={backdrop}
        alt=""
        onload={() => (backdropLoaded = true)}
      />
    {/key}
  {/if}
  <div class="shade"></div>

  <div class="inner">
    <div class="topbar">
      <button type="button" class="glass" onclick={onback}>‹ All games</button>
      <button type="button" class="glass subtle" onclick={onsnapshot}>Save HTML</button>
    </div>

    <div class="body">
      <div class="side">
        <div class="cover">
          {#if cover}
            <img src={cover} alt="" />
          {:else if header}
            <img class="cover-blur" src={header} alt="" />
            <img class="cover-header" src={header} alt="" />
          {/if}
        </div>
        {#if movies.length}
          <button type="button" class="glass trailer" onclick={() => (showTrailer = true)}>
            ▶ Watch trailer
          </button>
        {/if}
      </div>

      <div class="info">
        {#if labels.length}
          <div class="labels">
            {#each labels as label (label)}<span class="label">{label}</span>{/each}
          </div>
        {/if}
        <h1>{title}</h1>

        {#if facts.length}
          <p class="facts">
            {#each facts as fact, i (fact)}{#if i}<span class="dot">•</span>{/if}<span>{fact}</span>{/each}
          </p>
        {/if}

        {#if summary}<p class="summary">{summary}</p>{/if}

        {#if people.length || genres.length}
          <div class="chips">
            {#each people as [role, name] (role)}
              <span class="chip"><span class="role">{role}</span>{name}</span>
            {/each}
            {#each genres as genre (genre)}<span class="chip">{genre}</span>{/each}
          </div>
        {/if}

        <div class="latest">
          {#if latest}
            <div class="latest-head">
              <span class="kicker">
                Latest release{#if latestIsNew}<span class="new">New</span>{/if}
              </span>
              <span class="when">{[latest.author && `by ${latest.author}`, latest.posted].filter(Boolean).join(" · ")}</span>
            </div>
            <p class="latest-title">{latest.title ?? title}</p>
            {#if latest.tags.length || latestFacts.length}
              <div class="latest-facts">
                {#each latest.tags as tag (tag)}<span class="pill accent">{tag}</span>{/each}
                {#each latestFacts as fact (fact)}<span class="pill">{fact}</span>{/each}
              </div>
            {/if}
            <div class="downloads">
              {#each latest.links as link (link.url)}
                <LinkButton
                  url={link.url}
                  label={(link.flagged_unsafe ? "⚠ " : "") + (link.label !== link.host ? link.label : link.host)}
                  confirm={link.flagged_unsafe}
                  primary
                />
              {/each}
              {#if releaseCount > 1}
                <button type="button" class="glass all" onclick={onallreleases}>
                  All {releaseCount} releases ↓
                </button>
              {/if}
            </div>
          {:else if loadingReleases}
            <span class="kicker">Latest release</span>
            <p class="muted">Looking for releases…</p>
          {:else}
            <span class="kicker">Latest release</span>
            <p class="muted">No posts with download links found yet.</p>
          {/if}
        </div>

        <div class="links">
          {#if appId}
            <LinkButton url={steam?.store_url ?? `https://store.steampowered.com/app/${appId}`} label="Steam store" />
            <LinkButton url={forum?.steamdb_url ?? `https://steamdb.info/app/${appId}`} label="SteamDB" />
          {/if}
          <LinkButton url={topic.url} label="Forum topic" />
          {#if steam?.website}<LinkButton url={steam.website} label="Website" />{/if}
        </div>
      </div>
    </div>
  </div>
</section>

{#if showTrailer}
  <TrailerPlayer {movies} onclose={() => (showTrailer = false)} />
{/if}

<style>
  /* Always dark over the key art, whatever the app theme. */
  .hero {
    position: relative;
    margin: -20px -20px 0;
    padding: 16px 28px 32px;
    color: #f3f4f8;
    background: #0d1018;
    overflow: hidden;
    isolation: isolate;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 30%;
    z-index: -2;
    opacity: 0;
    transform: scale(1.04);
    transition:
      opacity 500ms ease,
      transform 1200ms ease;
  }
  .backdrop.loaded {
    opacity: 1;
    transform: none;
  }
  .shade {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      linear-gradient(to right, rgb(10 12 20 / 0.94) 0%, rgb(10 12 20 / 0.78) 45%, rgb(10 12 20 / 0.5) 100%),
      linear-gradient(to top, var(--bg) 0%, transparent 18%);
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    margin-bottom: 20px;
  }
  .glass,
  .hero :global(button) {
    background: rgb(255 255 255 / 0.08);
    border-color: rgb(255 255 255 / 0.18);
    color: #f3f4f8;
    backdrop-filter: blur(6px);
  }
  .hero :global(button:hover:not(:disabled)) {
    background: rgb(255 255 255 / 0.18);
  }
  .hero :global(button.primary) {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .hero :global(button.primary:hover) {
    filter: brightness(1.1);
    background: var(--accent);
  }
  .glass.subtle {
    border-color: transparent;
    background: none;
    color: rgb(255 255 255 / 0.7);
  }

  .body {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 28px;
    align-items: start;
  }
  @media (max-width: 760px) {
    .body {
      grid-template-columns: 1fr;
    }
    .side {
      width: 160px;
    }
  }

  .side {
    display: grid;
    gap: 10px;
  }
  .trailer {
    padding: 8px 12px;
    font-weight: 600;
  }
  .cover {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: 12px;
    overflow: hidden;
    background: rgb(255 255 255 / 0.06);
    box-shadow: 0 18px 40px rgb(0 0 0 / 0.5);
  }
  .cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover .cover-blur {
    filter: blur(16px) brightness(0.6);
    transform: scale(1.2);
  }
  .cover .cover-header {
    object-fit: contain;
  }

  .info {
    min-width: 0;
  }
  .labels {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 6px;
  }
  .label {
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    color: #ffd27a;
    background: rgb(255 190 60 / 0.14);
    border: 1px solid rgb(255 190 60 / 0.35);
  }
  h1 {
    margin: 0;
    font-size: 30px;
    line-height: 1.15;
    letter-spacing: -0.01em;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 10px 0 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.72);
  }
  .dot {
    opacity: 0.5;
  }
  .summary {
    max-width: 70ch;
    margin: 14px 0 0;
    line-height: 1.5;
    color: rgb(255 255 255 / 0.85);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
  }
  .chip {
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 12px;
    background: rgb(255 255 255 / 0.08);
    border: 1px solid rgb(255 255 255 / 0.12);
  }
  .role {
    color: rgb(255 255 255 / 0.55);
    margin-right: 6px;
  }

  .latest {
    margin-top: 20px;
    padding: 14px 16px;
    max-width: 720px;
    border-radius: 12px;
    background: rgb(255 255 255 / 0.07);
    border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(10px);
  }
  .latest-head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
  }
  .kicker {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #7ee2a8;
  }
  .new {
    margin-left: 8px;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 10px;
    letter-spacing: 0.04em;
    color: #0d1018;
    background: #7ee2a8;
  }
  .when {
    font-size: 12px;
    color: rgb(255 255 255 / 0.6);
  }
  .latest-title {
    margin: 6px 0 0;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .latest-facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .pill {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 11px;
    color: rgb(255 255 255 / 0.8);
    border: 1px solid rgb(255 255 255 / 0.25);
  }
  .pill.accent {
    color: #7ee2a8;
    border-color: rgb(126 226 168 / 0.5);
  }
  .downloads {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }
  .all {
    margin-left: auto;
  }
  .muted {
    margin: 6px 0 0;
    color: rgb(255 255 255 / 0.6);
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
  }
</style>
