<script lang="ts">
  import { art, requestArt } from "./cardArt.svelte";
  import type { TopicSummary } from "./types";

  let { topic, onselect }: { topic: TopicSummary; onselect: (topic: TopicSummary) => void } = $props();

  // Placeholder until (or unless) Steam has artwork: a colour and initials.
  const hue = $derived((topic.id * 47) % 360);
  const initials = $derived(
    topic.name
      .split(/\s+/)
      .filter((w) => /^[\p{L}\p{N}]/u.test(w))
      .slice(0, 2)
      .map((w) => w[0])
      .join("")
      .toUpperCase(),
  );
  // "[Info]" is on nearly every game topic, so it says nothing.
  const tags = $derived([...topic.prefixes.filter((p) => p.toLowerCase() !== "info"), ...topic.labels]);

  let element = $state<HTMLElement>();
  let visible = false;
  let requested = false;
  const image = $derived(art.get(topic.name)?.capsule_image ?? null);
  /** The image that finished loading, so a replaced image fades in too. */
  let loaded = $state<string | null>(null);

  // Look the game up on Steam once the card is (nearly) on screen.
  $effect(() => {
    if (!element) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        visible = entry.isIntersecting;
        if (!visible || requested || art.has(topic.name)) return;
        requested = true;
        requestArt(topic.name, () => visible).then((looked) => {
          if (!looked) requested = false; // skipped or failed; ask again when visible
        });
      },
      { rootMargin: "200px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  });
</script>

<button bind:this={element} type="button" class="card" title={topic.title} onclick={() => onselect(topic)}>
  <div class="art" style:--hue={hue}>
    <span>{initials}</span>
    {#if image}
      {#key image}
        <img src={image} alt="" class:loaded={loaded === image} onload={() => (loaded = image)} />
      {/key}
    {/if}
  </div>
  <div class="body">
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
  .card {
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
    text-align: left;
    border-radius: 10px;
    background: var(--surface);
    transition:
      transform 120ms ease,
      box-shadow 120ms ease;
  }
  .card:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.18);
  }
  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Steam's capsule image is 231×87; keeping its shape avoids cropping. */
  .art {
    position: relative;
    aspect-ratio: 231 / 87;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, hsl(var(--hue) 45% 32%), hsl(calc(var(--hue) + 40) 50% 18%));
  }
  .art span {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 1px;
    color: rgb(255 255 255 / 0.85);
  }
  .art img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 250ms ease;
  }
  .art img.loaded {
    opacity: 1;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px 12px;
    flex: 1;
  }
  h3 {
    margin: 0;
    font-size: 14px;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .meta {
    margin: auto 0 0;
    font-size: 12px;
    color: var(--muted);
  }
</style>
