<script lang="ts">
  import GameGrid from "./GameGrid.svelte";
  import type { Listing, TopicSummary } from "./types";
  import { latest, loadRecent, recent } from "./visits.svelte";

  /** Games opened before, most recent first, as one page of cards. */
  let { onselect }: { onselect: (topic: TopicSummary) => void } = $props();

  async function fetchPage(): Promise<Listing> {
    if (!recent.loaded) await loadRecent();
    return {
      topics: recent.visits
        .map((v) => v.topic && (latest.get(v.topic.id) ?? v.topic))
        .filter((t): t is TopicSummary => !!t),
      pagination: { start: 0, next_start: null, previous_start: null, last_start: 0, page: null, total_pages: null },
    };
  }
</script>

<GameGrid
  {onselect}
  {fetchPage}
  snapshotPath={() => ""}
  heading="Recently viewed"
  emptyText="Games you open show up here."
  showSnapshot={false}
/>
