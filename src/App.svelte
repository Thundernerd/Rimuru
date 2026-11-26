<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage } from "./lib/api";
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
  <div hidden={selected !== null}>
    {#key gridKey}
      <GameGrid onselect={select} />
    {/key}
  </div>
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
    margin: 0 auto 0 0;
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
