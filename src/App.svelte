<script lang="ts">
  import { onMount, tick, untrack } from "svelte";

  import { api, MAIN_FORUM_ID } from "./lib/api";
  import BrowseView from "./lib/BrowseView.svelte";
  import { loadFollows } from "./lib/follows.svelte";
  import { loadOwned } from "./lib/owned.svelte";
  import FollowingView from "./lib/FollowingView.svelte";
  import { clearForumPages } from "./lib/forum";
  import GameGrid from "./lib/GameGrid.svelte";
  import GameView from "./lib/GameView.svelte";
  import HomeView from "./lib/HomeView.svelte";
  import { back, current, forward, go, nav, replace, savedScroll, underlying, type View } from "./lib/nav.svelte";
  import RecentView from "./lib/RecentView.svelte";
  import { refreshSession, session } from "./lib/session.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import { status } from "./lib/status.svelte";
  import TitleBar from "./lib/TitleBar.svelte";
  import type { TopicSummary } from "./lib/types";
  import { loadRecent } from "./lib/visits.svelte";

  const mac = navigator.userAgent.includes("Mac");

  let main = $state<HTMLElement>();
  let searchInput = $state<HTMLInputElement>();
  let gridKey = $state(0);

  const view = $derived(current());
  const below = $derived(underlying());
  const search = $derived(below.kind === "search" ? below : null);
  const browse = $derived(below.kind === "browse" ? below : null);

  const scrollTop = () => main?.scrollTop ?? 0;
  const navigate = (to: View) => go(to, scrollTop());
  const openGame = (topic: TopicSummary) => navigate({ kind: "game", topic });
  const goBack = () => back(scrollTop());
  const goForward = () => forward(scrollTop());

  function searchPath(query: string, allForums: boolean) {
    return (start: number) => {
      const params = new URLSearchParams({ keywords: query });
      if (!allForums) params.append("fid[]", String(MAIN_FORUM_ID));
      params.append("sr", "topics");
      params.append("sf", "titleonly");
      params.append("start", String(start));
      return `search.php?${params}`;
    };
  }

  function focusSearch() {
    searchInput?.focus();
    searchInput?.select();
  }

  $effect(() => {
    void nav.index;
    void nav.entries;
    untrack(() => {
      const top = savedScroll();
      tick().then(() => main?.scrollTo({ top }));
    });
  });

  let wasLoggedOut = false;
  $effect(() => {
    const state = session.current?.state;
    if (state === "logged_out") wasLoggedOut = true;
    if (state === "logged_in" && wasLoggedOut) {
      wasLoggedOut = false;
      clearForumPages();
      gridKey++;
    }
  });

  function onkeydown(e: KeyboardEvent) {
    const mod = mac ? e.metaKey : e.ctrlKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      focusSearch();
    } else if ((mod && e.key === "[") || (e.altKey && e.key === "ArrowLeft")) {
      e.preventDefault();
      goBack();
    } else if ((mod && e.key === "]") || (e.altKey && e.key === "ArrowRight")) {
      e.preventDefault();
      goForward();
    }
  }

  function onmouseup(e: MouseEvent) {
    if (e.button === 3) goBack();
    else if (e.button === 4) goForward();
  }

  onMount(() => {
    refreshSession();
    loadRecent();
    loadFollows();
    loadOwned();
    const unlisten = api.onForumPageLoaded(() => refreshSession());
    return () => {
      unlisten.then((u) => u());
    };
  });
</script>

<svelte:window {onkeydown} {onmouseup} />

<div class="shell">
  <Sidebar onnavigate={navigate} onsearch={focusSearch} {mac} />

  <div class="column">
    <TitleBar
      onback={goBack}
      onforward={goForward}
      onsearch={(query, allForums) => navigate({ kind: "search", query, allForums, run: 0 })}
      {mac}
      bind:input={searchInput}
    />

    <main bind:this={main}>
      {#key gridKey}
        <div hidden={view.kind !== "home"}>
          <HomeView onselect={openGame} onnavigate={navigate} />
        </div>

        {#if browse}
          <div hidden={view.kind !== "browse"}>
            <BrowseView
              label={browse.label}
              onselect={openGame}
              onlabel={(label) => replace({ kind: "browse", label })}
            />
          </div>
        {/if}
      {/key}

      {#if search}
        <div hidden={view.kind !== "search"}>
          {#key search.run}
            <GameGrid
              onselect={openGame}
              fetchPage={(start) => api.searchTopics(search.query, start, search.allForums)}
              snapshotPath={searchPath(search.query, search.allForums)}
              heading={`Results for “${search.query}”${search.allForums ? " in all forums" : ""}`}
              emptyText="No topics with that in their title."
              errorText="The search didn't go through"
            />
          {/key}
        </div>
      {/if}

      {#if view.kind === "recent"}
        <RecentView onselect={openGame} />
      {:else if view.kind === "following"}
        <FollowingView onselect={openGame} />
      {:else if view.kind === "game"}
        {#key view.topic.id}
          <GameView topic={view.topic} />
        {/key}
      {/if}
    </main>

    {#if status.text}<div class="status">{status.text}</div>{/if}
  </div>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 248px minmax(0, 1fr);
    height: 100vh;
  }
  .column {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    min-width: 0;
    min-height: 0;
  }
  main {
    overflow: auto;
    padding: 20px 28px 28px;
    min-height: 0;
  }
  .status {
    padding: 5px 20px;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
