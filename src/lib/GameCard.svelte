<script lang="ts">
  import { artFor, matches, requestArt } from "./cardArt.svelte";
  import type { TopicSummary } from "./types";
  import { isFollowed } from "./follows.svelte";
  import Icon from "./Icon.svelte";
  import { newReplies } from "./visits.svelte";

  let { topic, onselect }: { topic: TopicSummary; onselect: (topic: TopicSummary) => void } = $props();

  // Placeholder until (or unless) Steam has artwork: a colour per topic.
  const hue = $derived((topic.id * 47) % 360);
  // "[Info]" is on nearly every game topic, so it says nothing.
  const tags = $derived([...topic.prefixes.filter((p) => p.toLowerCase() !== "info"), ...topic.labels]);

  const art = $derived(artFor(topic.name));
  const fresh = $derived(newReplies(topic));
  /** Portrait art fills the card; otherwise the header sits on a blurred copy of itself. */
  const portrait = $derived(art?.portrait ?? null);
  const fallback = $derived(portrait ? null : (art?.header ?? art?.capsule ?? null));
  const hasArt = $derived(!!(portrait || fallback));

  /** The image that finished loading, so a replaced image fades in too. */
  let loaded = $state<string | null>(null);
  let failed = $state<string | null>(null);

  let element = $state<HTMLElement>();
  let visible = false;
  let requested = false;

  // Look the game up on Steam once the card is (nearly) on screen.
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
          if (!looked) requested = false; // skipped or failed; ask again when visible
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
  class:has-art={hasArt && failed !== (portrait ?? fallback)}
  style:--hue={hue}
  title={topic.title}
  onclick={() => onselect(topic)}
>
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

  {#if fresh}
    <span class="fresh" title="Replies since you last opened this game">
      {fresh} new {fresh === 1 ? "reply" : "replies"}
    </span>
  {/if}

  {#if isFollowed(topic.id)}
    <span class="followed" title="Following"><Icon name="star" size={13} /></span>
  {/if}

  <div class="overlay">
    <h3>{topic.name}</h3>
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
  /* Steam's portrait "library capsule" is 600×900. */
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

  /* Title and details: always shown on placeholders, on hover/focus over art. */
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
  .has-art .overlay {
    opacity: 0;
    transform: translateY(8px);
  }
  .has-art:hover .overlay,
  .has-art:focus-visible .overlay {
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
