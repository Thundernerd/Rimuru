<script lang="ts">
  import { onDestroy } from "svelte";

  import type { Movie } from "./types";

  /**
   * Plays Steam trailers in an overlay. Steam serves them as HLS streams:
   * WebKit (macOS) plays those natively; other webviews use hls.js, loaded
   * only when needed. Older entries may have a plain MP4 instead.
   */
  let { movies, start = 0, onclose }: { movies: Movie[]; start?: number; onclose: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let index = $state(start);
  let video = $state<HTMLVideoElement>();
  let error = $state<string | null>(null);
  let hls: { destroy(): void } | null = null;

  const movie = $derived(movies[index]);

  function teardown() {
    hls?.destroy();
    hls = null;
  }

  async function attach(el: HTMLVideoElement, m: Movie) {
    teardown();
    error = null;
    if (m.hls && el.canPlayType("application/vnd.apple.mpegurl")) {
      el.src = m.hls;
    } else if (m.hls) {
      const { default: Hls } = await import("hls.js");
      if (!Hls.isSupported()) {
        if (m.mp4) el.src = m.mp4;
        else error = "This system can't play Steam's trailer streams.";
        return;
      }
      const player = new Hls({ capLevelToPlayerSize: true });
      player.on(Hls.Events.ERROR, (_event, data) => {
        if (data.fatal) error = "The trailer couldn't be loaded.";
      });
      player.loadSource(m.hls);
      player.attachMedia(el);
      hls = player;
    } else if (m.mp4) {
      el.src = m.mp4;
    }
    el.play().catch(() => {});
  }

  $effect(() => {
    if (video && movie) attach(video, movie);
  });

  onDestroy(teardown);

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="overlay" role="dialog" aria-modal="true" aria-label="Trailer">
  <button type="button" class="backdrop" aria-label="Close" onclick={onclose}></button>
  <div class="player">
    <!-- svelte-ignore a11y_media_has_caption -->
    <video bind:this={video} controls playsinline poster={movie?.thumbnail ?? undefined}></video>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="bar">
      <span class="name">{movie?.name}</span>
      {#if movies.length > 1}
        <div class="list">
          {#each movies as m, i (i)}
            <button type="button" class:active={i === index} onclick={() => (index = i)}>
              {#if m.thumbnail}<img src={m.thumbnail} alt="" />{/if}
              <span>{m.name}</span>
            </button>
          {/each}
        </div>
      {/if}
      <button type="button" class="close" onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    padding: 32px;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    border: none;
    border-radius: 0;
    background: rgb(0 0 0 / 0.88);
  }
  .backdrop:hover {
    background: rgb(0 0 0 / 0.88);
  }
  .player {
    position: relative;
    width: min(100%, 1100px);
    color: #f3f4f8;
  }
  video {
    display: block;
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 10px;
    background: #000;
    box-shadow: 0 20px 60px rgb(0 0 0 / 0.6);
  }
  .error {
    margin: 8px 0 0;
    color: var(--warn);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-top: 10px;
  }
  .name {
    font-weight: 600;
    margin-right: auto;
  }
  .list {
    display: flex;
    gap: 6px;
    overflow-x: auto;
  }
  .list button,
  .close {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #f3f4f8;
    background: rgb(255 255 255 / 0.08);
    border-color: rgb(255 255 255 / 0.18);
  }
  .list button.active {
    border-color: var(--accent);
  }
  .list img {
    height: 28px;
    border-radius: 3px;
  }
</style>
