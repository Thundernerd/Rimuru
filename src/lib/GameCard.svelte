<script lang="ts">
  import { artFor, matches, requestArt } from "./cardArt.svelte";
  import type { TopicSummary } from "./types";
  import { isFollowed } from "./follows.svelte";
  import { updateAvailable } from "./owned.svelte";
  import Icon from "./Icon.svelte";
  import { newReplies } from "./visits.svelte";

  let { topic, onselect }: { topic: TopicSummary; onselect: (topic: TopicSummary) => void } = $props();

  const hue = $derived((topic.id * 47) % 360);
  const tags = $derived([...topic.prefixes.filter((p) => p.toLowerCase() !== "info"), ...topic.labels]);

  const art = $derived(artFor(topic.name));
  const fresh = $derived(newReplies(topic));
  const update = $derived(updateAvailable(topic.id));
  const portrait = $derived(art?.portrait ?? null);
  const fallback = $derived(portrait ? null : (art?.header ?? art?.capsule ?? null));
  let loaded = $state<string | null>(null);
  let failed = $state<string | null>(null);
  const hasArt = $derived(!!(portrait || fallback));
  const artShown = $derived(hasArt && loaded === (portrait ?? fallback) && failed !== loaded);

  const initial = $derived(topic.name.match(/[\p{L}\p{N}]/u)?.[0]?.toUpperCase() ?? "?");
  const longestWord = $derived(Math.max(4, ...topic.name.split(/\s+/).map((w) => w.length)));
  const titleSize = $derived.by(() => {
    const length = topic.name.length;
    if (length <= 10) return 28;
    if (length <= 18) return 24;
    if (length <= 30) return 20;
    if (length <= 45) return 17;
    return 15;
  });


  let element = $state<HTMLElement>();
  let visible = false;
  let requested = false;

  $effect(() => {
    if (!element) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        visible = entry.isIntersecting;
        if (!visible || requested || matches.has(topic.name)) {
          if (visible && matches.has(topic.name)) requestArt(topic.name, () => true);
          return;
        }
        requested = true;
        requestArt(topic.name, () => visible).then((looked) => {
          if (!looked) requested = false;
        });
      },
      { rootMargin: "300px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  });
</script>

<button
  bind:this={element}
  type="button"
  class="card"
  class:art-shown={artShown}
  style:--hue={hue}
  title={topic.title}
  onclick={() => onselect(topic)}
>
  <div class="cover" aria-hidden="true">
    <span class="monogram">{initial}</span>
    <span class="cover-title" style:--size={`${titleSize}px`} style:--longest={longestWord}>{topic.name}</span>
    {#if topic.labels.length}<span class="cover-label">{topic.labels[0]}</span>{/if}
  </div>

  {#if portrait}
    {#key portrait}
      <img
        class="portrait"
        src={portrait}
        alt=""
        class:loaded={loaded === portrait}
        onload={() => (loaded = portrait)}
        onerror={() => (failed = portrait)}
      />
    {/key}
  {:else if fallback}
    {#key fallback}
      <img class="backdrop" src={fallback} alt="" class:loaded={loaded === fallback} />
      <img
        class="landscape"
        src={fallback}
        alt=""
        class:loaded={loaded === fallback}
        onload={() => (loaded = fallback)}
        onerror={() => (failed = fallback)}
      />
    {/key}
  {/if}

  {#if update}
    <span class="fresh update" title={`A newer release than yours: ${update.title ?? "new build"}`}>Update available</span>
  {:else if fresh}
    <span class="fresh" title="Replies since you last opened this game">
      {fresh} new {fresh === 1 ? "reply" : "replies"}
    </span>
  {/if}

  {#if isFollowed(topic.id)}
    <span class="followed" title="Following"><Icon name="star" size={13} /></span>
  {/if}

  <div class="overlay">
    {#if artShown}<h3>{topic.name}</h3>{/if}
    {#if tags.length}
      <div class="tags">
        {#each tags as tag (tag)}
          <span class="tag">{tag}</span>
        {/each}
      </div>
    {/if}
    <p class="meta">
      {#if topic.last_post_at}Updated {topic.last_post_at}{/if}
      {#if topic.replies !== null}<span>· {topic.replies} replies</span>{/if}
    </p>
  </div>
</button>

<style>
  .card {
    position: relative;
    aspect-ratio: 2 / 3;
    padding: 0;
    overflow: hidden;
    border: none;
    border-radius: 10px;
    text-align: left;
    color: #fff;
    background: linear-gradient(160deg, hsl(var(--hue) 45% 34%), hsl(calc(var(--hue) + 40) 50% 14%));
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.2);
    transition:
      transform 150ms ease,
      box-shadow 150ms ease;
  }
  .card:hover {
    transform: translateY(-3px) scale(1.02);
    box-shadow: 0 10px 24px rgb(0 0 0 / 0.3);
  }
  .card:hover:not(:disabled) {
    background: linear-gradient(160deg, hsl(var(--hue) 45% 34%), hsl(calc(var(--hue) + 40) 50% 14%));
  }
  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }

  .cover {
    container-type: inline-size;
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 18px 16px;
    overflow: hidden;
    background:
      radial-gradient(120% 80% at 15% 10%, hsl(var(--hue) 70% 45% / 0.55), transparent 60%),
      radial-gradient(90% 60% at 100% 100%, hsl(calc(var(--hue) + 50) 70% 42% / 0.35), transparent 60%),
      linear-gradient(165deg, hsl(var(--hue) 30% 17%), hsl(calc(var(--hue) + 30) 35% 8%));
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.06);
  }
  .cover::after {
    content: "";
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(135deg, rgb(255 255 255 / 0.035) 0 2px, transparent 2px 14px);
    pointer-events: none;
  }
  .monogram {
    position: absolute;
    right: -6%;
    bottom: -10%;
    font-size: 200px;
    font-weight: 900;
    line-height: 1;
    color: rgb(255 255 255 / 0.07);
    pointer-events: none;
  }
  .cover-title {
    position: relative;
    font-size: min(var(--size), calc((100cqi - 32px) / (var(--longest) * 0.62)));
    font-weight: 800;
    line-height: 1.12;
    letter-spacing: -0.01em;
    text-wrap: balance;
    overflow-wrap: break-word;
    text-shadow: 0 2px 10px rgb(0 0 0 / 0.45);
    display: -webkit-box;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .cover-title::before {
    content: "";
    display: block;
    width: 28px;
    height: 3px;
    margin-bottom: 12px;
    border-radius: 2px;
    background: hsl(var(--hue) 85% 65%);
  }
  .cover-label {
    position: relative;
    align-self: flex-start;
    margin-top: 12px;
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgb(255 255 255 / 0.85);
    background: rgb(0 0 0 / 0.3);
    border: 1px solid rgb(255 255 255 / 0.18);
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    transition: opacity 300ms ease;
  }
  img.loaded {
    opacity: 1;
  }
  .portrait {
    object-fit: cover;
  }
  .backdrop {
    object-fit: cover;
    filter: blur(18px) brightness(0.55);
    transform: scale(1.2);
  }
  .landscape {
    object-fit: contain;
  }

  .fresh {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 700;
    color: var(--accent-text);
    background: var(--accent);
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.35);
  }

  .fresh.update {
    color: #052e16;
    background: var(--good);
  }

  .followed {
    position: absolute;
    top: 8px;
    right: 8px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    color: #ffd27a;
    background: rgb(8 10 16 / 0.7);
    backdrop-filter: blur(4px);
  }
  .followed :global(svg) {
    fill: currentColor;
  }

  .overlay {
    position: absolute;
    inset: auto 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 40px 12px 12px;
    background: linear-gradient(to top, rgb(0 0 0 / 0.9) 0%, rgb(0 0 0 / 0.65) 55%, transparent 100%);
    transition:
      opacity 150ms ease,
      transform 150ms ease;
  }
  .overlay {
    opacity: 0;
    transform: translateY(8px);
  }
  .card:hover .overlay,
  .card:focus-visible .overlay {
    opacity: 1;
    transform: none;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    line-height: 1.3;
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.6);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tags .tag {
    color: rgb(255 255 255 / 0.85);
    border-color: rgb(255 255 255 / 0.35);
  }
  .meta {
    margin: 0;
    font-size: 11px;
    color: rgb(255 255 255 / 0.75);
  }
</style>
