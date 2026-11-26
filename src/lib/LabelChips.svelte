<script lang="ts">
  let {
    labels,
    selected = null,
    onselect,
  }: { labels: [string, number][]; selected?: string | null; onselect: (label: string | null) => void } = $props();
</script>

{#if labels.length}
  <div class="chips">
    {#each labels as [label, count] (label)}
      <button
        type="button"
        class:on={selected === label}
        onclick={() => onselect(selected === label ? null : label)}
        aria-pressed={selected === label}
      >
        {label}<span class="count">{count}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 999px;
    font-size: 13px;
    color: var(--text);
    background: var(--surface);
  }
  button.on {
    color: var(--accent);
    border-color: rgb(63 200 240 / 0.6);
    background: var(--accent-soft);
  }
  .count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }
  button.on .count {
    color: var(--accent);
  }
</style>
