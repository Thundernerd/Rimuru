<script lang="ts">
  import { api, errorMessage } from "./api";
  import { setStatus } from "./status.svelte";

  /** Opens `url` in the system browser; with `confirm`, only on a second click. */
  let { url, label, confirm = false }: { url: string; label: string; confirm?: boolean } = $props();

  let armed = $state(false);

  function open() {
    if (confirm && !armed) {
      armed = true;
      return;
    }
    armed = false;
    api.openInBrowser(url).catch((e) => setStatus(errorMessage(e)));
  }
</script>

<button type="button" class:armed title={url} onclick={open} onblur={() => (armed = false)}>
  {armed ? "Click again to open" : label}
</button>

<style>
  button {
    padding: 2px 10px;
    font-size: 12px;
  }
  .armed {
    border-color: var(--warn);
    color: var(--warn);
  }
</style>
