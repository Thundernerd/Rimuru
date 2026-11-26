<script lang="ts">
  import Icon from "./Icon.svelte";
  import LinkButton from "./LinkButton.svelte";
  import type { Artwork, GameInfo, SteamApp, TopicSummary } from "./types";

  let {
    topic,
    forum,
    steam,
    art,
    appId,
    stats,
    hasTrailer,
    latestIsNew,
    followed,
    onfollow,
    ontrailer,
    onsnapshot,
  }: {
    topic: TopicSummary;
    forum: GameInfo | null;
    steam: SteamApp | null;
    art: Artwork | null;
    appId: number | null;
    stats: [string, string, boolean?][];
    hasTrailer: boolean;
    latestIsNew: boolean;
    followed: boolean;
    onfollow: () => void;
    ontrailer: () => void;
    onsnapshot: () => void;
  } = $props();

  const PLATFORM_NAMES: Record<string, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };

  const title = $derived(steam?.name || forum?.title || topic.name);
  const backdrop = $derived(
    art?.hero ?? steam?.screenshots[0]?.full ?? steam?.header_image ?? forum?.header_image ?? null,
  );
  const logo = $derived(
    appId ? `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${appId}/logo.png` : null,
  );
  let logoFailed = $state<string | null>(null);
  let logoLoaded = $state<string | null>(null);
  const showLogo = $derived(!!logo && logoFailed !== logo);

  const facts = $derived(
    [
      (steam?.platforms ?? forum?.platforms)?.map((p) => PLATFORM_NAMES[p] ?? p).join(" · "),
      steam?.release_date
        ? steam.release_date + (steam.coming_soon ? " (coming soon)" : "")
        : forum?.release_date,
      steam ? (steam.is_free ? "Free" : steam.price) : null,
      (steam?.genres.length ? steam.genres : forum?.genres)?.slice(0, 3).join(" · "),
    ].filter((f): f is string => !!f),
  );

  let backdropLoaded = $state(false);
</script>

<section class="hero">
  {#if backdrop}
    {#key backdrop}
      <img class="backdrop" class:loaded={backdropLoaded} src={backdrop} alt="" onload={() => (backdropLoaded = true)} />
    {/key}
  {/if}
  <div class="shade"></div>

  <button type="button" class="snapshot" onclick={onsnapshot}>Save HTML</button>

  <div class="bottom">
    <div class="identity">
      <div class="badges">
        {#if latestIsNew}<span class="badge new">New release</span>{/if}
        {#each topic.labels as label (label)}<span class="badge">{label}</span>{/each}
        {#if topic.last_post_at}<span class="eyebrow">Updated {topic.last_post_at}</span>{/if}
      </div>

      {#if showLogo && logo}
        <img
          class="logo"
          class:loaded={logoLoaded === logo}
          src={logo}
          alt={title}
          onload={() => (logoLoaded = logo)}
          onerror={() => (logoFailed = logo)}
        />
      {/if}
      <h1 class:hidden-by-logo={showLogo && logoLoaded === logo}>{title}</h1>

      {#if facts.length}<p class="facts">{facts.join("  ·  ")}</p>{/if}

      <div class="actions">
        {#if hasTrailer}
          <button type="button" class="primary" onclick={ontrailer}>▶ Watch trailer</button>
        {/if}
        <button
          type="button"
          class="follow"
          class:on={followed}
          aria-pressed={followed}
          onclick={onfollow}
          title={followed ? "Stop following this game" : "Follow this game: it shows in the sidebar, with new replies marked"}
        >
          <Icon name="star" size={15} />
          {followed ? "Following" : "Follow"}
        </button>
        {#if appId}
          <LinkButton url={steam?.store_url ?? `https://store.steampowered.com/app/${appId}`} label="Steam store" />
          <LinkButton url={forum?.steamdb_url ?? `https://steamdb.info/app/${appId}`} label="SteamDB" />
        {/if}
        <LinkButton url={topic.url} label="Forum topic" />
        {#if steam?.website}<LinkButton url={steam.website} label="Website" />{/if}
      </div>
    </div>

    {#if stats.length}
      <dl class="stats">
        {#each stats as [label, value, mono] (label)}
          <div>
            <dt>{label}</dt>
            <dd class:mono>{value}</dd>
          </div>
        {/each}
      </dl>
    {/if}
  </div>
</section>

<style>
  .hero {
    position: relative;
    margin: -20px -28px 0;
    min-height: 380px;
    padding: 24px 28px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
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
      linear-gradient(to top, var(--bg) 0%, rgb(11 14 20 / 0.75) 30%, rgb(11 14 20 / 0.15) 70%),
      linear-gradient(to right, rgb(11 14 20 / 0.7) 0%, transparent 60%);
  }

  .snapshot {
    position: absolute;
    top: 14px;
    right: 20px;
    font-size: 12px;
    color: rgb(255 255 255 / 0.65);
    background: none;
    border-color: transparent;
  }
  .snapshot:hover:not(:disabled) {
    color: #fff;
    background: rgb(255 255 255 / 0.1);
  }

  .bottom {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    flex-wrap: wrap;
  }
  .identity {
    min-width: 0;
    max-width: 720px;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    color: #ffd27a;
    background: rgb(255 190 60 / 0.14);
    border: 1px solid rgb(255 190 60 / 0.35);
  }
  .badge.new {
    color: #052e16;
    background: var(--good);
    border-color: var(--good);
  }
  .eyebrow {
    color: rgb(255 255 255 / 0.7);
  }

  .logo {
    display: block;
    max-width: min(440px, 80%);
    max-height: 150px;
    object-fit: contain;
    object-position: left bottom;
    filter: drop-shadow(0 4px 18px rgb(0 0 0 / 0.6));
    opacity: 0;
    transition: opacity 300ms ease;
  }
  .logo.loaded {
    opacity: 1;
  }
  h1 {
    margin: 0;
    font-size: 40px;
    line-height: 1.08;
    letter-spacing: -0.015em;
    text-shadow: 0 2px 14px rgb(0 0 0 / 0.55);
  }
  h1.hidden-by-logo {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .facts {
    margin: 12px 0 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.72);
    white-space: pre-wrap;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 18px;
  }
  .actions :global(button) {
    padding: 8px 14px;
    font-size: 13px;
    color: #f3f4f8;
    background: rgb(255 255 255 / 0.08);
    border-color: rgb(255 255 255 / 0.18);
    backdrop-filter: blur(6px);
  }
  .actions :global(button:hover:not(:disabled)) {
    background: rgb(255 255 255 / 0.18);
  }
  .actions :global(button.primary) {
    font-weight: 600;
    color: var(--accent-text);
    background: var(--accent);
    border-color: var(--accent);
  }
  .actions :global(button.primary:hover:not(:disabled)) {
    background: var(--accent);
    filter: brightness(1.08);
  }

  .actions .follow {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .actions .follow.on {
    color: #ffd27a;
    border-color: rgb(255 190 60 / 0.5);
    background: rgb(255 190 60 / 0.14);
  }
  .actions .follow.on :global(svg) {
    fill: currentColor;
  }
  .stats {
    display: flex;
    margin: 0;
  }
  .stats div {
    padding: 0 20px;
    border-left: 1px solid rgb(255 255 255 / 0.14);
  }
  .stats div:first-child {
    border-left: none;
  }
  dt {
    font-size: 11px;
    color: rgb(255 255 255 / 0.6);
  }
  dd {
    margin: 4px 0 0;
    font-size: 18px;
    font-weight: 600;
    white-space: nowrap;
  }
  dd.mono {
    font-family: var(--mono);
    font-size: 15px;
  }
</style>
