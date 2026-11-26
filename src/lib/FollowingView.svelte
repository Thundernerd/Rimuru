<script lang="ts">
  import { followedTopics, follows } from "./follows.svelte";
  import GameGrid from "./GameGrid.svelte";
  import type { Listing, TopicSummary } from "./types";

  /** Followed games, those with new replies first, as one page of cards. */
  let { onselect }: { onselect: (topic: TopicSummary) => void } = $props();

  async function fetchPage(): Promise<Listing> {
    return {
      topics: followedTopics(),
      pagination: { start: 0, next_start: null, previous_start: null, last_start: 0, page: null, total_pages: null },
    };
  }
</script>

<!-- Re-list when follows change (e.g. unfollowing from a game page). -->
{#key follows.size}
  <GameGrid
    {onselect}
    {fetchPage}
    snapshotPath={() => ""}
    heading="Following"
    emptyText="Follow games from their page (☆ Follow) to keep them here and in the sidebar."
    showSnapshot={false}
  />
{/key}
