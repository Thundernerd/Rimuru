<script lang="ts">
  import { api, errorMessage } from "./api";
  import { rememberArt } from "./cardArt.svelte";
  import GameInfoPanel from "./GameInfoPanel.svelte";
  import ReleaseList from "./ReleaseList.svelte";
  import { setStatus } from "./status.svelte";
  import type { GameFirstPage, Release, SteamApp, TopicSummary } from "./types";

  let { topic, onback }: { topic: TopicSummary; onback: () => void } = $props();

  /** Pages read for releases: page 1 plus up to this many from the end. */
  const RECENT_PAGES = 4;

  let first = $state<GameFirstPage | null>(null);
  let steam = $state<SteamApp | null>(null);
  let steamLoading = $state(false);
  /** Releases from the most recent pages, newest first; page 1's come after. */
  let recent = $state<Release[]>([]);
  let pagesRead = $state(0);
  let pagesPlanned = $state(1);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let attempt = $state(0);

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
    steamLoading = true;
    try {
      const appId = page.info?.steam_app_id ?? (await api.steamMatch(name))?.app_id;
      if (appId && isCurrent()) {
        const app = await api.steamApp(appId);
        if (isCurrent()) steam = app;
        // The topic links this Steam app itself: use its artwork for the
        // card, which fills in or corrects a match by name.
        if (app?.capsule_image && page.info?.steam_app_id === app.app_id) {
          rememberArt(name, { app_id: app.app_id, name: app.name, capsule_image: app.capsule_image });
        }
      }
    } catch (e) {
      if (isCurrent()) setStatus(`Steam details unavailable: ${errorMessage(e)}`);
    } finally {
      if (isCurrent()) steamLoading = false;
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

<div class="toolbar">
  <button type="button" onclick={onback}>‹ All games</button>
  <button type="button" class="subtle" onclick={saveSnapshot}>Save HTML</button>
</div>

<div class="layout">
  <GameInfoPanel {topic} forum={first?.info ?? null} {steam} loading={!first || steamLoading} />
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
    <p class="muted">Loading releases…</p>
  {/if}
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(280px, 400px) 1fr;
    gap: 24px;
    align-items: start;
  }
  .layout > :global(.panel) {
    position: sticky;
    top: 0;
  }
  @media (max-width: 800px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .layout > :global(.panel) {
      position: static;
    }
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
