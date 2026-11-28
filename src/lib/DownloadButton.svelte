<script lang="ts">
  import { formatProgress, isArchive } from "./download";
  import { cancelDownload, downloadLink, downloads } from "./download.svelte";
  import { extractDownload, extraction } from "./extract.svelte";

  let { url, confirm = false }: { url: string; confirm?: boolean } = $props();

  let armed = $state(false);

  const running = $derived(downloads.running[url]);
  const done = $derived(downloads.done[url]);

  function start() {
    if (confirm && !armed) {
      armed = true;
      return;
    }
    armed = false;
    downloadLink(url);
  }
</script>

{#if running}
  <button type="button" class="busy" onclick={() => cancelDownload(url)} title="Cancel this download">
    Downloading {formatProgress(running.progress)} · Cancel
  </button>
{:else}
  <button
    type="button"
    class:armed
    onclick={start}
    onblur={() => (armed = false)}
    title="Download into the folder set in Settings. Hosts that need a wait, captcha or login only open in the browser."
  >
    {armed ? "Click again to download" : done ? "Download again" : "Download"}
  </button>
  {#if done && isArchive(done)}
    <button
      type="button"
      onclick={() => extractDownload(done)}
      disabled={extraction.running}
      title={`Extract ${done} into the folder set in Settings`}
    >
      {extraction.running ? "Extracting…" : "Extract"}
    </button>
  {/if}
{/if}

<style>
  button {
    padding: 3px 10px;
    font-size: 12px;
  }
  .busy {
    font-variant-numeric: tabular-nums;
  }
  .armed {
    border-color: var(--warn);
    color: var(--warn);
  }
</style>
