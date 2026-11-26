<script lang="ts">
  import { untrack } from "svelte";

  import { isNewer, noteReleases, owned, updateAvailable } from "./owned.svelte";

  import { api, errorMessage } from "./api";
  import { rememberArt } from "./cardArt.svelte";
  import GameDetails from "./GameDetails.svelte";
  import GameHero from "./GameHero.svelte";
  import ReleaseCard from "./ReleaseCard.svelte";
  import ReleaseList from "./ReleaseList.svelte";
  import Requirements from "./Requirements.svelte";
  import Screenshots from "./Screenshots.svelte";
  import TrailerPlayer from "./TrailerPlayer.svelte";
  import { setStatus } from "./status.svelte";
  import type { Artwork, GameFirstPage, Release, SteamApp, TopicSummary, Visit } from "./types";
  import { isFollowed, toggleFollow } from "./follows.svelte";
  import { latest as listed, previousVisit, recordVisit } from "./visits.svelte";

  let { topic }: { topic: TopicSummary } = $props();

  const RECENT_PAGES = 4;

  let first = $state<GameFirstPage | null>(null);
  let steam = $state<SteamApp | null>(null);
  let art = $state<Artwork | null>(null);
  let recent = $state<Release[]>([]);
  let pagesRead = $state(0);
  let pagesPlanned = $state(1);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let attempt = $state(0);
  let previous = $state<Visit | null>(null);

  const releases = $derived([...recent, ...(first?.releases ?? [])]);
  const newIds = $derived(
    previous
      ? new Set(releases.filter((r) => !previous!.release_ids.includes(r.post_id)).map((r) => r.post_id))
      : new Set<number>(),
  );

  $effect(() => {
    const shown = topic;
    void attempt;
    let current = true;
    untrack(() => load(shown, () => current));
    return () => {
      current = false;
    };
  });

  function load(shown: TopicSummary, isCurrent: () => boolean) {
    const { id, replies, name } = shown;
    first = null;
    steam = null;
    art = null;
    previous = null;
    recent = [];
    pagesRead = 0;
    pagesPlanned = 1;
    error = null;
    loading = true;

    (async () => {
      const [page, visit] = await Promise.all([api.gameFirstPage(id, replies), previousVisit(id)]);
      if (!isCurrent()) return;
      previous = visit;
      first = page;
      pagesRead = 1;

      loadSteam(page, name, isCurrent);

      const starts: number[] = [];
      if (page.per_page) {
        for (let s = page.last_start; s > 0 && starts.length < RECENT_PAGES; s -= page.per_page) starts.push(s);
      }
      pagesPlanned = 1 + starts.length;
      for (const start of starts) {
        const found = await api.gameReleases(id, start, replies);
        if (!isCurrent()) return;
        recent.push(...found);
        pagesRead += 1;
      }
    })()
      .catch((e) => isCurrent() && (error = errorMessage(e)))
      .finally(() => {
        if (!isCurrent()) return;
        loading = false;
        if (first) recordVisit(shown, releases.map((r) => r.post_id));
      });
  }

  async function loadSteam(page: GameFirstPage, name: string, isCurrent: () => boolean) {
    try {
      const appId = page.info?.steam_app_id ?? (await api.steamMatch(name))?.app_id;
      if (!appId || !isCurrent()) return;
      api
        .steamArtwork([appId])
        .then((found) => isCurrent() && (art = found[appId] ?? null))
        .catch(() => {});
      const app = await api.steamApp(appId);
      if (!isCurrent()) return;
      steam = app;
      if (app?.capsule_image && page.info?.steam_app_id === app.app_id) {
        rememberArt(name, { app_id: app.app_id, name: app.name, capsule_image: app.capsule_image });
      }
    } catch (e) {
      if (isCurrent()) setStatus(`Steam details unavailable: ${errorMessage(e)}`);
    }
  }

  async function saveSnapshot() {
    try {
      setStatus(`Saved ${await api.saveSnapshot(`viewtopic.php?t=${topic.id}&start=0`)}`);
    } catch (e) {
      setStatus(errorMessage(e));
    }
  }

  type Tab = "overview" | "releases" | "media" | "requirements";
  let tab = $state<Tab>("overview");
  let trailerAt = $state<number | null>(null);

  $effect(() => {
    void topic.id;
    tab = "overview";
    trailerAt = null;
  });

  const latest = $derived(releases[0] ?? null);
  const newestBuild = $derived(releases.reduce<Release | null>((best, r) => (!best || isNewer(r, best) ? r : best), null));
  const ownership = $derived(
    owned.has(topic.id) ? (updateAvailable(topic.id) ? "update" : "current") : null,
  ) as "update" | "current" | null;

  $effect(() => {
    const id = topic.id;
    const found = releases;
    untrack(() => noteReleases(id, found));
  });
  const movies = $derived(steam?.movies ?? []);
  const shots = $derived(steam?.screenshots ?? []);
  const requirements = $derived(first?.info?.system_requirements ?? []);
  const appId = $derived(steam?.app_id ?? first?.info?.steam_app_id ?? null);

  const tabs = $derived(
    [
      ["overview", "Overview", null],
      ["releases", "Releases", releases.length || null],
      ["media", "Media", movies.length + shots.length || null],
      ["requirements", "Requirements", null],
    ].filter(([id]) => (id === "media" ? movies.length + shots.length > 0 : id === "requirements" ? requirements.length > 0 : true)) as [
      Tab,
      string,
      number | null,
    ][],
  );

  const stats = $derived(
    [
      ["Replies", topic.replies?.toLocaleString()],
      ["Views", topic.views?.toLocaleString()],
      ["Releases", first ? `${releases.length}${loading ? "…" : ""}` : null],
      ["Latest build", newestBuild?.build ?? newestBuild?.version ?? null, true],
    ].filter((s) => !!s[1]) as [string, string, boolean?][],
  );
</script>

<GameHero
  {topic}
  forum={first?.info ?? null}
  {steam}
  {art}
  {appId}
  {stats}
  hasTrailer={movies.length > 0}
  latestIsNew={!!latest && newIds.has(latest.post_id)}
  {ownership}
  followed={isFollowed(topic.id)}
  onfollow={() => toggleFollow(listed.get(topic.id) ?? topic)}
  ontrailer={() => (trailerAt = 0)}
  onsnapshot={saveSnapshot}
/>

<nav class="tabs" aria-label="Game sections">
  {#each tabs as [id, label, count] (id)}
    <button type="button" class:active={tab === id} aria-current={tab === id} onclick={() => (tab = id)}>
      {label}{#if count}<span class="count">{count}</span>{/if}
    </button>
  {/each}
</nav>

<div class="page">
  {#if error && !first}
    <div class="message">
      <p>Couldn't load this topic: {error}</p>
      <button type="button" onclick={() => attempt++}>Try again</button>
    </div>
  {:else if tab === "overview"}
    <section class="latest">
      <header>
        <h2>Latest release</h2>
        {#if releases.length > 1}
          <button type="button" class="subtle" onclick={() => (tab = "releases")}>All {releases.length} releases ›</button>
        {/if}
      </header>
      {#if latest}
        <ReleaseCard release={latest} topicId={topic.id} isNew={newIds.has(latest.post_id)} featured />
      {:else if loading}
        <div class="placeholder"></div>
      {:else}
        <p class="muted">No posts with download links found on the pages read.</p>
      {/if}
    </section>

    {#if shots.length}
      <Screenshots {shots} />
    {/if}

    <div class="columns">
      <div class="stack"><GameDetails forum={first?.info ?? null} {steam} part="main" /></div>
      <div class="stack"><GameDetails forum={first?.info ?? null} {steam} part="side" /></div>
    </div>
  {:else if tab === "releases" && first}
    <ReleaseList
      {releases}
      topicId={topic.id}
      {newIds}
      {pagesRead}
      {pagesPlanned}
      totalPages={first.total_pages}
      {loading}
      {error}
      onretry={() => attempt++}
    />
  {:else if tab === "media"}
    {#if movies.length}
      <section>
        <h2>Trailers</h2>
        <div class="trailers">
          {#each movies as movie, i (i)}
            <button type="button" onclick={() => (trailerAt = i)}>
              {#if movie.thumbnail}<img src={movie.thumbnail} alt="" loading="lazy" />{/if}
              <span class="play">▶</span>
              <span class="name">{movie.name}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}
    {#if shots.length}
      <Screenshots {shots} grid />
    {/if}
  {:else if tab === "requirements"}
    <Requirements sets={requirements} />
  {/if}
</div>

{#if trailerAt !== null}
  <TrailerPlayer {movies} start={trailerAt} onclose={() => (trailerAt = null)} />
{/if}

<style>
  .tabs {
    display: flex;
    gap: 4px;
    margin: 0 -28px;
    padding: 0 28px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 12px 11px;
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: none;
    color: var(--muted);
    font-weight: 500;
  }
  .tabs button:hover:not(:disabled) {
    background: none;
    color: var(--text);
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }

  .page {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 28px;
    padding-top: 24px;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .latest header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  .placeholder {
    height: 140px;
    border-radius: 12px;
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
  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 360px);
    gap: 20px;
    align-items: start;
  }
  @media (max-width: 960px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }
  .stack {
    display: grid;
    gap: 16px;
  }

  .trailers {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 12px;
  }
  .trailers button {
    position: relative;
    padding: 0;
    border-radius: 10px;
    overflow: hidden;
    aspect-ratio: 16 / 9;
    background: #000;
  }
  .trailers img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.85;
  }
  .play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 30px;
    color: #fff;
    text-shadow: 0 2px 10px rgb(0 0 0 / 0.6);
  }
  .trailers .name {
    position: absolute;
    left: 10px;
    bottom: 8px;
    font-size: 12px;
    font-weight: 600;
    color: #fff;
    text-shadow: 0 1px 4px rgb(0 0 0 / 0.8);
  }

  .muted {
    color: var(--muted);
  }
  .message {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
</style>
