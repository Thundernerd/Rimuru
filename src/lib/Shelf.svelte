<script lang="ts">
  import GameCard from "./GameCard.svelte";
  import Icon from "./Icon.svelte";
  import type { TopicSummary } from "./types";

  let {
    title,
    subtitle = null,
    topics,
    onselect,
    onseeall = null,
  }: {
    title: string;
    subtitle?: string | null;
    topics: TopicSummary[];
    onselect: (topic: TopicSummary) => void;
    onseeall?: (() => void) | null;
  } = $props();

  let row = $state<HTMLElement>();

  function page(direction: number) {
    row?.scrollBy({ left: direction * row.clientWidth * 0.85, behavior: "smooth" });
  }
</script>

{#if topics.length}
  <section>
    <header>
      <h2>{title}</h2>
      {#if subtitle}<span class="subtitle">{subtitle}</span>{/if}
      <span class="spacer"></span>
      {#if onseeall}<button type="button" class="subtle" onclick={onseeall}>See all ›</button>{/if}
      <button type="button" class="subtle arrow" aria-label="Scroll left" onclick={() => page(-1)}>
        <Icon name="back" size={16} />
      </button>
      <button type="button" class="subtle arrow" aria-label="Scroll right" onclick={() => page(1)}>
        <Icon name="forward" size={16} />
      </button>
    </header>
    <div class="row" bind:this={row}>
      {#each topics as topic (topic.id)}
        <div class="slot"><GameCard {topic} {onselect} /></div>
      {/each}
    </div>
  </section>
{/if}

<style>
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  .subtitle {
    font-size: 13px;
    color: var(--muted);
  }
  .spacer {
    flex: 1;
  }
  .arrow {
    display: grid;
    place-items: center;
    padding: 4px;
    align-self: center;
  }
  .row {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 170px;
    gap: 16px;
    overflow-x: auto;
    scroll-snap-type: x proximity;
    padding: 4px 2px 10px;
    scrollbar-width: none;
  }
  .row::-webkit-scrollbar {
    display: none;
  }
  .slot {
    display: grid;
    scroll-snap-align: start;
  }
</style>
