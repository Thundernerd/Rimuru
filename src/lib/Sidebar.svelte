<script lang="ts">
  import { api, errorMessage } from "./api";
  import { artFor, requestArt } from "./cardArt.svelte";
  import { followedTopics, follows } from "./follows.svelte";
  import Icon from "./Icon.svelte";
  import { current, type View } from "./nav.svelte";
  import { session, signIn } from "./session.svelte";
  import { setStatus } from "./status.svelte";
  import type { TopicSummary } from "./types";
  import { latest, newReplies, recent } from "./visits.svelte";

  /**
   * Navigation, followed and recently viewed games (with a dot for new
   * replies), and the forum account.
   */
  let {
    onnavigate,
    onsearch,
    mac,
  }: { onnavigate: (view: View) => void; onsearch: () => void; mac: boolean } = $props();

  const following = $derived(followedTopics());
  // Fewer recent games when the followed list takes room too.
  const shown = $derived(
    recent.visits
      .map((v) => v.topic && (latest.get(v.topic.id) ?? v.topic))
      .filter((t): t is TopicSummary => !!t && !follows.has(t.id))
      .slice(0, following.length ? 5 : 8),
  );
  const view = $derived(current());
  const openGame = $derived(view.kind === "game" ? view.topic.id : null);

  // Thumbnails for the list.
  $effect(() => {
    for (const topic of [...following, ...shown]) requestArt(topic.name, () => true);
  });

  const username = $derived(session.current?.state === "logged_in" ? session.current.username : null);

  function showForum() {
    api.showForumWindow("index.php").catch((e) => setStatus(errorMessage(e)));
  }
</script>

<aside class:mac>
  <div class="brand" data-tauri-drag-region>
    <span class="logo"><Icon name="layers" size={18} /></span>
    <span class="name">RinRu<span class="dim">Desktop</span></span>
  </div>

  <nav>
    <button type="button" class:active={view.kind === "home"} onclick={() => onnavigate({ kind: "home" })}>
      <Icon name="home" /> Home
    </button>
    <button
      type="button"
      class:active={view.kind === "browse"}
      onclick={() => onnavigate({ kind: "browse", label: null })}
    >
      <Icon name="grid" /> Browse
    </button>
    <button type="button" class:active={view.kind === "search"} onclick={onsearch}>
      <Icon name="search" /> Search
    </button>
    <button
      type="button"
      class:active={view.kind === "following"}
      onclick={() => onnavigate({ kind: "following" })}
    >
      <Icon name="star" /> Following
    </button>
    <button type="button" class:active={view.kind === "recent"} onclick={() => onnavigate({ kind: "recent" })}>
      <Icon name="clock" /> Recently viewed
    </button>
  </nav>

  <div class="lists">
    {#if following.length}
      <section>
        <div class="eyebrow">Following <span class="count">{following.length}</span></div>
        {@render games(following)}
      </section>
    {/if}
    <!-- Followed games aren't repeated here; hide the heading if that leaves nothing. -->
    {#if shown.length || !following.length}
      <section>
        <div class="eyebrow">
          Recently viewed {#if recent.visits.length}<span class="count">{recent.visits.length}</span>{/if}
        </div>
        {#if shown.length}
          {@render games(shown)}
        {:else if recent.loaded}
          <p class="empty">Games you open show up here.</p>
        {/if}
      </section>
    {/if}
  </div>

  <footer>
    <span class="avatar">{(username ?? "?").slice(0, 1).toUpperCase()}</span>
    <span class="who">
      {#if session.current?.state === "logged_in"}
        <strong>{username ?? "Signed in"}</strong>
        <small>cs.rin.ru</small>
      {:else if session.current?.state === "logged_out"}
        <strong>Not signed in</strong>
        <button type="button" class="link" onclick={signIn}>Sign in</button>
      {:else}
        <strong>Connecting…</strong>
      {/if}
    </span>
    <button type="button" class="subtle icon" onclick={showForum} title="Show the forum's browser window">
      <Icon name="globe" />
    </button>
  </footer>
</aside>

{#snippet games(topics: TopicSummary[])}
  <ul>
    {#each topics as topic (topic.id)}
      {@const art = artFor(topic.name)}
      {@const fresh = newReplies(topic)}
      <li>
        <button
          type="button"
          class:active={openGame === topic.id}
          title={fresh ? `${topic.title}: ${fresh} new replies` : topic.title}
          onclick={() => onnavigate({ kind: "game", topic })}
        >
          <span class="thumb" style:--hue={(topic.id * 47) % 360}>
            {#if art?.portrait || art?.capsule}<img src={art.portrait ?? art.capsule} alt="" />{/if}
          </span>
          <span class="title">{topic.name}</span>
          {#if fresh}<span class="dot" aria-label={`${fresh} new replies`}></span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

<style>
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 56px;
    padding: 0 18px;
    flex-shrink: 0;
  }
  /* Room for the window buttons on macOS. */
  aside.mac .brand {
    height: 92px;
    align-items: flex-end;
    padding-bottom: 14px;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    color: var(--accent-text);
    background: linear-gradient(135deg, #5ad7f7, #2b8fd6);
    pointer-events: none;
  }
  .name {
    font-weight: 800;
    letter-spacing: 0.02em;
    pointer-events: none;
  }
  .dim {
    margin-left: 4px;
    font-weight: 500;
    color: var(--muted);
  }

  nav {
    display: grid;
    gap: 2px;
    padding: 6px 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    border: none;
    background: none;
    color: var(--muted);
    text-align: left;
  }
  nav button:hover:not(:disabled) {
    background: var(--surface);
    color: var(--text);
  }
  nav button.active {
    color: var(--text);
    background: var(--surface);
  }
  nav button.active::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }

  .lists {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 10px;
    display: grid;
    align-content: start;
    gap: 18px;
  }
  .lists .eyebrow {
    padding: 0 8px 8px;
    text-transform: none;
    letter-spacing: 0;
    font-size: 12px;
  }
  .count {
    margin-left: 4px;
    font-family: var(--mono);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }
  li button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    background: none;
    color: var(--text);
    text-align: left;
    font-size: 13px;
  }
  li button:hover:not(:disabled),
  li button.active {
    background: var(--surface);
  }
  .thumb {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    overflow: hidden;
    background: linear-gradient(135deg, hsl(var(--hue) 45% 34%), hsl(calc(var(--hue) + 40) 50% 18%));
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    flex-shrink: 0;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warn);
  }
  .empty {
    margin: 0;
    padding: 0 8px;
    font-size: 12px;
    color: var(--muted);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
  }
  .avatar {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    font-weight: 700;
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid rgb(63 200 240 / 0.3);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: grid;
    line-height: 1.25;
  }
  .who strong {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who small {
    font-size: 11px;
    color: var(--accent);
  }
  .link {
    justify-self: start;
    padding: 0;
    border: none;
    background: none;
    font-size: 12px;
    color: var(--accent);
  }
  .link:hover:not(:disabled) {
    background: none;
    text-decoration: underline;
  }
  .icon {
    display: grid;
    place-items: center;
    padding: 6px;
  }
</style>
