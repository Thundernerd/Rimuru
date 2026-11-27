<script lang="ts">
  let { height = 140, radius = 12 }: { height?: number | string; radius?: number } = $props();
</script>

<div
  class="skeleton"
  style:height={typeof height === "number" ? `${height}px` : height}
  style:border-radius={`${radius}px`}
  aria-hidden="true"
></div>

<style>
  .skeleton {
    background: var(--surface);
    animation: pulse 1.2s ease-in-out infinite alternate;
  }
  @keyframes pulse {
    from {
      opacity: 0.4;
    }
    to {
      opacity: 0.9;
    }
  }
</style>
