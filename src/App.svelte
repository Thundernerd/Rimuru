<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage, MAIN_FORUM_ID } from "./lib/api";
  import GameGrid from "./lib/GameGrid.svelte";
  import GameView from "./lib/GameView.svelte";
  import { refreshSession, session, signIn } from "./lib/session.svelte";
  import { setStatus, status } from "./lib/status.svelte";
  import type { TopicSummary } from "./lib/types";

  let selected = $state<TopicSummary | null>(null);
  /** Bumped on sign-in so the grid reloads with the signed-in view. */
  let gridKey = $state(0);
  let main = $state<HTMLElement>();
  let gridScroll = 0;

  /** The search being shown, if any; the forum listing stays loaded below it. */
  let search = $state<{ query: string; allForums: boolean } | null>(null);
  let searchText = $state("");
  let allForums = $state(false);
  /** Bumped per search, so the same query can be searched again. */
  let searchRun = $state(0);

  const forumPage = (start: number) => api.forumPage(MAIN_FORUM_ID, start);
  const forumPath = (start: number) => `viewforum.php?f=${MAIN_FORUM_ID}&start=${start}`;

  function submitSearch(e: SubmitEvent) {
    e.preventDefault();
    const query = searchText.trim();
    if (!query) return clearSearch();
    search = { query, allForums };
    searchRun++;
    selected = null;
    main?.scrollTo({ top: 0 });
  }

  function clearSearch() {
    search = null;
    searchText = "";
    selected = null;
  }

  function searchPath(start: number) {
    const params = new URLSearchParams({ keywords: search?.query ?? "" });
    if (!search?.allForums) params.append("fid[]", String(MAIN_FORUM_ID));
    params.append("sr", "topics");
    params.append("sf", "titleonly");
    params.append("start", String(start));
    return `search.php?${params}`;
  }

  function select(topic: TopicSummary) {
    gridScroll = main?.scrollTop ?? 0;
    selected = topic;
    main?.scrollTo({ top: 0 });
  }

  function back() {
    selected = null;
    // Restore the grid's position after it is shown again.
    requestAnimationFrame(() => main?.scrollTo({ top: gridScroll }));
  }

  let wasLoggedOut = false;
  $effect(() => {
    const state = session.current?.state;
    if (state === "logged_out") wasLoggedOut = true;
    if (state === "logged_in" && wasLoggedOut) {
      wasLoggedOut = false;
      gridKey++;
    }
  });

  onMount(() => {
    refreshSession();
    // Each page the forum window finishes may change the session (security
    // check passed, signed in, signed out).
    const unlisten = api.onForumPageLoaded(() => refreshSession());
    return () => {
      unlisten.then((u) => u());
    };
  });

  function showForum() {
    api.showForumWindow("index.php").catch((e) => setStatus(errorMessage(e)));
  }
</script>

<header>
  <h1>RinRu Desktop</h1>
  <form class="search" onsubmit={submitSearch}>
    <input
      type="search"
      placeholder="Search games on the forum"
      bind:value={searchText}
      aria-label="Search games"
    />
    <label class="scope" title="Search topic titles in all forums, not just the main game forum">
      <input type="checkbox" bind:checked={allForums} /> All forums
    </label>
    <button type="submit">Search</button>
  </form>
  <div class="session">
    {#if session.current?.state === "logged_in"}
      <span>Signed in{session.current.username ? ` as ${session.current.username}` : ""}</span>
    {:else if session.current?.state === "logged_out"}
      <span>Not signed in</span>
      <button type="button" class="primary" onclick={signIn}>Sign in</button>
    {:else}
      <span>Connecting…</span>
    {/if}
    <button type="button" class="subtle" onclick={showForum} title="Show the forum's browser window">
      Forum window
    </button>
  </div>
</header>

<main bind:this={main}>
  <!-- Kept mounted while a game is open, so its loaded pages and scroll survive. -->
  <div hidden={selected !== null || search !== null}>
    {#key gridKey}
      <GameGrid onselect={select} fetchPage={forumPage} snapshotPath={forumPath} />
    {/key}
  </div>
  {#if search}
    <div hidden={selected !== null}>
      {#key searchRun}
        {@const { query, allForums: everywhere } = search}
        <div class="search-bar">
          <button type="button" onclick={clearSearch}>‹ All games</button>
        </div>
        <GameGrid
          onselect={select}
          fetchPage={(start) => api.searchTopics(query, start, everywhere)}
          snapshotPath={searchPath}
          heading={`Results for “${query}”${everywhere ? " in all forums" : ""}`}
          emptyText="No topics with that in their title."
          errorText="The search didn't go through"
        />
      {/key}
    </div>
  {/if}
  {#if selected}
    <GameView topic={selected} onback={back} />
  {/if}
</main>

<footer>{status.text}</footer>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  h1 {
    font-size: 16px;
    margin: 0;
    white-space: nowrap;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    max-width: 520px;
    margin: 0 auto;
  }
  .search input[type="search"] {
    flex: 1;
    min-width: 120px;
  }
  .scope {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .search-bar {
    margin-bottom: 12px;
  }
  .session {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
  }
  main {
    overflow: auto;
    padding: 20px;
    min-height: 0;
  }
  footer {
    padding: 6px 20px;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 12px;
    min-height: 1.5em;
  }
</style>
