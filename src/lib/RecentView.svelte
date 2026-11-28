<script lang="ts">
  import { errorMessage } from "./api";
  import GameGrid from "./GameGrid.svelte";
  import { setStatus } from "./status.svelte";
  import type { Listing, TopicSummary } from "./types";
  import { clearRecent, latest, loadRecent, recent } from "./visits.svelte";

  let { onselect }: { onselect: (topic: TopicSummary) => void } = $props();

  let armed = $state(false);
  let clearing = $state(false);
  let generation = $state(0);

  async function fetchPage(): Promise<Listing> {
    if (!recent.loaded) await loadRecent();
    return {
      topics: recent.visits
        .map((v) => v.topic && (latest.get(v.topic.id) ?? v.topic))
        .filter((t): t is TopicSummary => !!t),
      pagination: { start: 0, next_start: null, previous_start: null, last_start: 0, page: null, total_pages: null },
    };
  }

  async function clear() {
    if (!armed) {
      armed = true;
      return;
    }
    armed = false;
    clearing = true;
    try {
      await clearRecent();
      generation += 1;
    } catch (e) {
      setStatus(`Couldn't clear recently viewed: ${errorMessage(e)}`);
    } finally {
      clearing = false;
    }
  }
</script>

{#snippet actions()}
  <button
    type="button"
    class="subtle"
    class:armed
    onclick={clear}
    onblur={() => (armed = false)}
    disabled={clearing || !recent.visits.length}
  >
    {armed ? "Click again to clear" : "Clear"}
  </button>
{/snippet}

{#key generation}
  <GameGrid
    {onselect}
    {fetchPage}
    {actions}
    snapshotPath={() => ""}
    heading="Recently viewed"
    emptyText="Games you open show up here."
    showSnapshot={false}
  />
{/key}

<style>
  .armed {
    border-color: var(--warn);
    color: var(--warn);
  }
</style>
