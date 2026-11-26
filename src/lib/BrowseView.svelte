<script lang="ts">
  import { MAIN_FORUM_ID } from "./api";
  import { englishSections, forumListing, labelCounts } from "./forum";
  import GameGrid from "./GameGrid.svelte";
  import LabelChips from "./LabelChips.svelte";
  import type { ForumSort } from "./nav.svelte";
  import type { ForumSection, TopicSummary } from "./types";
  import { latest } from "./visits.svelte";

  let {
    forumId,
    sort,
    label,
    onselect,
    onchange,
  }: {
    forumId: number;
    sort: ForumSort;
    label: string | null;
    onselect: (topic: TopicSummary) => void;
    onchange: (change: { forumId?: number; sort?: ForumSort; label?: string | null }) => void;
  } = $props();

  const SORTS: [ForumSort, string][] = [
    ["activity", "Latest activity"],
    ["replies", "Most replies"],
    ["views", "Most views"],
  ];

  let sections = $state<ForumSection[]>([]);

  $effect(() => {
    forumListing(MAIN_FORUM_ID, "activity", 0)
      .then((page) => (sections = englishSections(page.sections)))
      .catch(() => {});
  });

  const sectionName = $derived(sections.find((s) => s.id === forumId)?.name ?? (forumId === MAIN_FORUM_ID ? "Main Forum" : `Forum ${forumId}`));
  const labels = $derived(labelCounts([...latest.values()].filter((t) => t.kind === "normal")).slice(0, 14));
  const order = $derived(sort === "activity" ? "" : sort === "replies" ? "&sk=r&sd=d" : "&sk=v&sd=d");
</script>

<div class="browse">
  <div class="controls">
    <label>
      <span>Section</span>
      <select value={forumId} onchange={(e) => onchange({ forumId: Number(e.currentTarget.value), label: null })}>
        {#if !sections.some((s) => s.id === forumId)}<option value={forumId}>{sectionName}</option>{/if}
        {#each sections as section (section.id)}
          <option value={section.id}>{"\u00a0\u00a0".repeat(Math.max(0, section.depth - 1))}{section.name}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>Sort</span>
      <select value={sort} onchange={(e) => onchange({ sort: e.currentTarget.value as ForumSort })}>
        {#each SORTS as [value, text] (value)}<option {value}>{text}</option>{/each}
      </select>
    </label>
  </div>

  <LabelChips {labels} selected={label} onselect={(next) => onchange({ label: next })} />

  {#key `${forumId}:${sort}:${label}`}
    <GameGrid
      {onselect}
      fetchPage={(start) => forumListing(forumId, sort, start)}
      snapshotPath={(start) => `viewforum.php?f=${forumId}&start=${start}${order}`}
      heading={label ? `${sectionName}: ${label}` : sectionName}
      filter={label ? (t) => t.labels.includes(label) : null}
      emptyText={label ? `No “${label}” topics on the pages loaded so far.` : "No topics here."}
    />
  {/key}
</div>

<style>
  .browse {
    display: grid;
    gap: 20px;
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  select {
    font: inherit;
    font-size: 13px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
  }
</style>
