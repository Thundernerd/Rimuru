<script lang="ts">
  import { SvelteMap } from "svelte/reactivity";

  import { api } from "./api";
  import { artFor, artLoading, matches, requestArt } from "./cardArt.svelte";
  import { thumbSlots } from "./featured";
  import type { SteamApp, TopicSummary } from "./types";

  let { topics, onselect }: { topics: TopicSummary[]; onselect: (topic: TopicSummary) => void } = $props();

  const CANDIDATES = 16;
  const FEATURED = 6;
  const INTERVAL_MS = 8000;

  const details = new SvelteMap<number, SteamApp | null>();

  $effect(() => {
    for (const topic of topics.slice(0, CANDIDATES)) requestArt(topic.name, () => true);
  });

  const featured = $derived(
    topics
      .slice(0, CANDIDATES)
      .filter((t) => artFor(t.name)?.hero)
      .slice(0, FEATURED),
  );

  // While art for the candidates is still arriving, keep the row at FEATURED slots,
  // with empty ones for entries yet to come, so it doesn't grow as images land.
  const loading = $derived(topics.length === 0 || topics.slice(0, CANDIDATES).some((t) => artLoading(t.name)));
  const slots = $derived(thumbSlots(featured, loading, FEATURED));

  let index = $state(0);
  let paused = $state(false);
  let tickKey = $state(0);
  const item = $derived(featured[Math.min(index, featured.length - 1)] ?? null);
  const art = $derived(item ? artFor(item.name) : null);
  const appId = $derived(item ? (matches.get(item.name) ?? null) : null);
  const app = $derived(appId ? (details.get(appId) ?? null) : null);

  $effect(() => {
    const id = appId;
    if (!id || details.has(id)) return;
    details.set(id, null);
    api
      .steamApp(id)
      .then((found) => details.set(id, found))
      .catch(() => details.delete(id));
  });

  $effect(() => {
    void tickKey;
    if (paused || featured.length < 2) return;
    const timer = setTimeout(() => {
      index = (index + 1) % featured.length;
      tickKey++;
    }, INTERVAL_MS);
    return () => clearTimeout(timer);
  });

  function show(i: number) {
    index = i;
    tickKey++;
  }

  const facts = $derived(
    item
      ? [
          app?.genres.slice(0, 3).join(" · "),
          item.replies !== null ? `${item.replies} replies` : null,
          app ? (app.is_free ? "Free" : app.price) : null,
        ].filter((f): f is string => !!f)
      : [],
  );
</script>

{#if item && art?.hero}
  <section class="featured" onmouseenter={() => (paused = true)} onmouseleave={() => (paused = false)} aria-label="Featured games">
    {#key item.id}
      <img class="backdrop" src={art.hero} alt="" />
    {/key}
    <div class="shade"></div>

    <div class="content">
      <div class="eyebrow-row">
        {#if item.last_post_at}<span class="eyebrow">Updated {item.last_post_at}</span>{/if}
        {#each item.labels as label (label)}<span class="label">{label}</span>{/each}
      </div>
      <h2>{item.name}</h2>
      {#if app?.short_description}<p class="summary">{app.short_description}</p>{/if}
      {#if facts.length}<p class="facts">{facts.join(" · ")}</p>{/if}
      <div class="actions">
        <button type="button" class="primary" onclick={() => onselect(item)}>Open game</button>
      </div>
    </div>

    {#if slots.length > 1}
      <div class="thumbs">
        {#each featured as topic, i (topic.id)}
          {@const thumb = artFor(topic.name)}
          <button
            type="button"
            class="thumb"
            class:active={i === index}
            onclick={() => show(i)}
            aria-label={`Show ${topic.name}`}
            title={topic.name}
          >
            <img src={thumb?.header ?? thumb?.capsule ?? ""} alt="" />
            {#if i === index}
              {#key tickKey}<span class="progress" class:paused style:--duration={`${INTERVAL_MS}ms`}></span>{/key}
            {/if}
          </button>
        {/each}
        {#each slots.slice(featured.length) as _, i (i)}
          <span class="thumb slot"><span class="frame"></span></span>
        {/each}
      </div>
    {/if}
  </section>
{:else}
  <div class="featured placeholder" aria-hidden="true">
    <div class="thumbs">
      {#each thumbSlots([], true, FEATURED) as _, i (i)}
        <span class="thumb slot"><span class="frame"></span></span>
      {/each}
    </div>
  </div>
{/if}

<style>
  .featured {
    position: relative;
    height: 380px;
    border-radius: 16px;
    overflow: hidden;
    isolation: isolate;
    background: var(--surface);
  }
  .placeholder {
    animation: pulse 1.2s ease-in-out infinite alternate;
  }
  @keyframes pulse {
    from {
      opacity: 0.5;
    }
    to {
      opacity: 0.9;
    }
  }
  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 35%;
    z-index: -2;
    animation: fade-in 600ms ease;
  }
  @keyframes fade-in {
    from {
      opacity: 0;
      transform: scale(1.03);
    }
  }
  .shade {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      linear-gradient(to right, rgb(8 10 16 / 0.92) 0%, rgb(8 10 16 / 0.7) 40%, rgb(8 10 16 / 0.1) 75%),
      linear-gradient(to top, rgb(8 10 16 / 0.7) 0%, transparent 40%);
  }
  .content {
    position: absolute;
    left: 36px;
    bottom: 34px;
    max-width: min(560px, 55%);
  }
  .eyebrow-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }
  .eyebrow {
    color: rgb(255 255 255 / 0.75);
  }
  .label {
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid rgb(63 200 240 / 0.35);
  }
  h2 {
    margin: 0;
    font-size: 36px;
    line-height: 1.1;
    letter-spacing: -0.01em;
    text-shadow: 0 2px 12px rgb(0 0 0 / 0.5);
  }
  .summary {
    margin: 12px 0 0;
    line-height: 1.5;
    color: rgb(255 255 255 / 0.82);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .facts {
    margin: 10px 0 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.65);
  }
  .actions {
    margin-top: 18px;
  }
  .actions button {
    padding: 10px 22px;
    font-size: 14px;
  }

  .thumbs {
    position: absolute;
    right: 24px;
    bottom: 24px;
    display: flex;
    gap: 8px;
  }
  .thumb {
    position: relative;
    width: 96px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 8px;
    overflow: hidden;
    background: #000;
    opacity: 0.55;
    transition: opacity 150ms ease;
  }
  .thumbs button:hover:not(:disabled),
  .thumbs button.active {
    opacity: 1;
    background: #000;
  }
  .thumbs button.active {
    border-color: rgb(255 255 255 / 0.9);
  }
  .slot {
    display: block;
    background: rgb(255 255 255 / 0.06);
    opacity: 1;
  }
  .thumbs img,
  .slot .frame {
    display: block;
    width: 100%;
    aspect-ratio: 460 / 215;
    object-fit: cover;
  }
  .progress {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 3px;
    width: 100%;
    background: var(--accent);
    transform-origin: left;
    animation: progress var(--duration) linear forwards;
  }
  .progress.paused {
    animation-play-state: paused;
  }
  @keyframes progress {
    from {
      transform: scaleX(0);
    }
  }
  @media (max-width: 1100px) {
    .thumb {
      width: 72px;
    }
  }
</style>
