<script lang="ts">
  import { api, errorMessage } from "./api";
  import { rememberArt } from "./cardArt.svelte";
  import GameDetails from "./GameDetails.svelte";
  import GameHero from "./GameHero.svelte";
  import ReleaseList from "./ReleaseList.svelte";
  import Screenshots from "./Screenshots.svelte";
  import { setStatus } from "./status.svelte";
  import type { Artwork, GameFirstPage, Release, SteamApp, TopicSummary } from "./types";

  let { topic, onback }: { topic: TopicSummary; onback: () => void } = $props();

  /** Pages read for releases: page 1 plus up to this many from the end. */
  const RECENT_PAGES = 4;

  let first = $state<GameFirstPage | null>(null);
  let steam = $state<SteamApp | null>(null);
  let art = $state<Artwork | null>(null);
  /** Releases from the most recent pages, newest first; page 1's come after. */
  let recent = $state<Release[]>([]);
  let pagesRead = $state(0);
  let pagesPlanned = $state(1);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let attempt = $state(0);
  let releasesSection = $state<HTMLElement>();

  const releases = $derived([...recent, ...(first?.releases ?? [])]);

  // Load whenever the topic changes (or on retry); responses for a topic
  // that's no longer shown are dropped.
  $effect(() => {
    const id = topic.id;
    const replies = topic.replies;
    const name = topic.name;
    void attempt;
    let current = true;

    first = null;
    steam = null;
    art = null;
    recent = [];
    pagesRead = 0;
    pagesPlanned = 1;
    error = null;
    loading = true;

    (async () => {
      const page = await api.gameFirstPage(id, replies);
      if (!current) return;
      first = page;
      pagesRead = 1;

      loadSteam(page, name, () => current);

      // Then the most recent pages, newest first, one at a time.
      const starts: number[] = [];
      if (page.per_page) {
        for (let s = page.last_start; s > 0 && starts.length < RECENT_PAGES; s -= page.per_page) starts.push(s);
      }
      pagesPlanned = 1 + starts.length;
      for (const start of starts) {
        const found = await api.gameReleases(id, start, replies);
        if (!current) return;
        recent.push(...found);
        pagesRead += 1;
      }
    })()
      .catch((e) => current && (error = errorMessage(e)))
      .finally(() => current && (loading = false));

    return () => {
      current = false;
    };
  });

  async function loadSteam(page: GameFirstPage, name: string, isCurrent: () => boolean) {
    try {
      const appId = page.info?.steam_app_id ?? (await api.steamMatch(name))?.app_id;
      if (!appId || !isCurrent()) return;
      // Artwork (cover, backdrop) and details in parallel.
      api
        .steamArtwork([appId])
        .then((found) => isCurrent() && (art = found[appId] ?? null))
        .catch(() => {});
      const app = await api.steamApp(appId);
      if (!isCurrent()) return;
      steam = app;
      // The topic links this Steam app itself: use its artwork for the
      // card, which fills in or corrects a match by name.
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
</script>

<GameHero
  {topic}
  forum={first?.info ?? null}
  {steam}
  {art}
  latest={releases[0] ?? null}
  releaseCount={releases.length}
  loadingReleases={loading}
  {onback}
  onsnapshot={saveSnapshot}
  onallreleases={() => releasesSection?.scrollIntoView({ behavior: "smooth", block: "start" })}
/>

<div class="page">
  {#if steam?.screenshots.length}
    <Screenshots shots={steam.screenshots} />
  {/if}

  <div class="columns">
    <div bind:this={releasesSection} class="releases">
      {#if first}
        <ReleaseList
          {releases}
          {pagesRead}
          {pagesPlanned}
          totalPages={first.total_pages}
          {loading}
          {error}
          onretry={() => attempt++}
        />
      {:else if error}
        <div class="message">
          <p>Couldn't load this topic: {error}</p>
          <button type="button" onclick={() => attempt++}>Try again</button>
        </div>
      {:else}
        <p class="muted">Loading the topic…</p>
      {/if}
    </div>
    <aside>
      <GameDetails forum={first?.info ?? null} {steam} />
    </aside>
  </div>
</div>

<style>
  .page {
    display: grid;
    /* minmax(0, …): don't let the screenshot strip widen the page. */
    grid-template-columns: minmax(0, 1fr);
    gap: 28px;
    padding-top: 24px;
  }
  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 380px);
    gap: 24px;
    align-items: start;
  }
  @media (max-width: 900px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }
  .releases {
    scroll-margin-top: 12px;
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
