<script lang="ts">
  import { api, errorMessage } from "./api";
  import { setStatus } from "./status.svelte";

  let {
    url,
    label,
    confirm = false,
    primary = false,
  }: { url: string; label: string; confirm?: boolean; primary?: boolean } = $props();

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

<button type="button" class:armed class:primary title={url} onclick={open} onblur={() => (armed = false)}>
  {armed ? "Click again to open" : label}
</button>

<style>
  button {
    padding: 3px 10px;
    font-size: 12px;
  }
  .primary {
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
  }
  .armed {
    border-color: var(--warn);
    color: var(--warn);
  }
</style>
