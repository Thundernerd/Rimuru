<script lang="ts">
  let {
    shots,
    grid = false,
    title = "Screenshots",
  }: { shots: { thumbnail: string; full: string }[]; grid?: boolean; title?: string | null } = $props();

  let open = $state<number | null>(null);

  function step(by: number) {
    if (open === null) return;
    open = (open + by + shots.length) % shots.length;
  }

  function onkeydown(e: KeyboardEvent) {
    if (open === null) return;
    if (e.key === "Escape") open = null;
    else if (e.key === "ArrowRight") step(1);
    else if (e.key === "ArrowLeft") step(-1);
  }
</script>

<svelte:window {onkeydown} />

<section>
  {#if title}<h2>{title}</h2>{/if}
  <div class="strip" class:grid>
    {#each shots as shot, i (shot.full)}
      <button type="button" onclick={() => (open = i)} aria-label={`Screenshot ${i + 1}`}>
        <img src={shot.thumbnail} alt="" loading="lazy" />
      </button>
    {/each}
  </div>
</section>

{#if open !== null}
  <div class="lightbox" role="dialog" aria-modal="true" aria-label="Screenshot">
    <button type="button" class="backdrop" aria-label="Close" onclick={() => (open = null)}></button>
    <img src={shots[open].full} alt="" />
    {#if shots.length > 1}
      <button type="button" class="nav prev" aria-label="Previous" onclick={() => step(-1)}>‹</button>
      <button type="button" class="nav next" aria-label="Next" onclick={() => step(1)}>›</button>
    {/if}
    <span class="count">{open + 1} / {shots.length}</span>
  </div>
{/if}

<style>
  h2 {
    margin: 0 0 10px;
    font-size: 16px;
  }
  .strip {
    display: flex;
    gap: 10px;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    padding-bottom: 8px;
  }
  .strip button {
    flex: 0 0 auto;
    padding: 0;
    border: none;
    border-radius: 8px;
    overflow: hidden;
    scroll-snap-align: start;
    background: var(--surface);
    transition: transform 150ms ease;
  }
  .strip button:hover {
    transform: translateY(-2px);
  }
  .strip.grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    overflow: visible;
  }
  .strip.grid img {
    width: 100%;
    height: auto;
  }
  .strip img {
    display: block;
    height: 150px;
    aspect-ratio: 16 / 9;
    object-fit: cover;
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: grid;
    place-items: center;
    padding: 40px 72px;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    border: none;
    border-radius: 0;
    background: rgb(0 0 0 / 0.88);
    cursor: zoom-out;
  }
  .backdrop:hover {
    background: rgb(0 0 0 / 0.88);
  }
  .lightbox img {
    position: relative;
    max-width: 100%;
    max-height: 100%;
    border-radius: 8px;
    box-shadow: 0 20px 60px rgb(0 0 0 / 0.6);
    pointer-events: none;
  }
  .nav {
    position: absolute;
    top: 50%;
    translate: 0 -50%;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    font-size: 24px;
    line-height: 1;
    color: #fff;
    background: rgb(255 255 255 / 0.12);
    border-color: rgb(255 255 255 / 0.2);
  }
  .nav:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.25);
  }
  .prev {
    left: 16px;
  }
  .next {
    right: 16px;
  }
  .count {
    position: absolute;
    bottom: 14px;
    font-size: 12px;
    color: rgb(255 255 255 / 0.7);
  }
</style>
