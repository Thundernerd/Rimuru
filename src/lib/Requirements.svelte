<script lang="ts">
  import type { GameInfo } from "./types";

  /** System requirements from the topic's first post, side by side. */
  let { sets }: { sets: GameInfo["system_requirements"] } = $props();
</script>

<div class="sets">
  {#each sets as set, i (i)}
    <section class="card">
      <h3>{set.label || "Requirements"}</h3>
      <dl>
        {#each set.items as item, j (j)}
          <dt>{item.key}</dt>
          <dd>{item.value}</dd>
        {/each}
      </dl>
    </section>
  {/each}
</div>

<style>
  .sets {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 16px;
    align-items: start;
  }
  .card {
    padding: 16px 18px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }
  h3 {
    margin: 0 0 12px;
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 16px;
    margin: 0;
    font-size: 13px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
