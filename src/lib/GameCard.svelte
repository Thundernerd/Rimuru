<script lang="ts">
  import type { TopicSummary } from "./types";

  let { topic, onselect }: { topic: TopicSummary; onselect: (topic: TopicSummary) => void } = $props();

  // Until cards get real artwork: a colour per topic and the name's initials.
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
</script>

<button type="button" class="card" title={topic.title} onclick={() => onselect(topic)}>
  <div class="art" style:--hue={hue}>
    <span>{initials}</span>
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

  /* Steam header images are 460×215. */
  .art {
    aspect-ratio: 460 / 215;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, hsl(var(--hue) 45% 32%), hsl(calc(var(--hue) + 40) 50% 18%));
  }
  .art span {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 1px;
    color: rgb(255 255 255 / 0.85);
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
