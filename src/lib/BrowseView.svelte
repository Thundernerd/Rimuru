<script lang="ts">
  import { forumPage, labelCounts } from "./forum";
  import GameGrid from "./GameGrid.svelte";
  import LabelChips from "./LabelChips.svelte";
  import { MAIN_FORUM_ID } from "./api";
  import type { TopicSummary } from "./types";
  import { latest } from "./visits.svelte";

  /** The main forum's full listing, filterable by title label. */
  let {
    label,
    onselect,
    onlabel,
  }: {
    label: string | null;
    onselect: (topic: TopicSummary) => void;
    onlabel: (label: string | null) => void;
  } = $props();

  // Labels across every listed game seen so far (grows as pages load).
  const labels = $derived(labelCounts([...latest.values()].filter((t) => t.kind === "normal")).slice(0, 14));
</script>

<div class="browse">
  <LabelChips {labels} selected={label} onselect={onlabel} />
  {#key label}
    <GameGrid
      {onselect}
      fetchPage={forumPage}
      snapshotPath={(start) => `viewforum.php?f=${MAIN_FORUM_ID}&start=${start}`}
      heading={label ?? "All games"}
      filter={label ? (t) => t.labels.includes(label) : null}
      emptyText={label ? `No “${label}” games on the pages loaded so far.` : "No games here."}
    />
  {/key}
</div>

<style>
  .browse {
    display: grid;
    gap: 20px;
  }
</style>
