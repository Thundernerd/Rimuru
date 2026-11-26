<script lang="ts">
  import LinkButton from "./LinkButton.svelte";
  import type { Release } from "./types";

  let { release }: { release: Release } = $props();

  const facts = $derived(
    [
      // "1.0.0.6" reads as a version number, "16 November 2026 - ..." doesn't.
      release.version && (/^v?\d+(\.\d+)+/i.test(release.version) ? `v${release.version.replace(/^v/i, "")}` : release.version),
      release.build && `Build ${release.build}`,
      release.size,
    ].filter((f): f is string => !!f),
  );
</script>

<article>
  <header>
    <h4>{release.title ?? "Release"}</h4>
    <p class="byline">
      {[release.author && `by ${release.author}`, release.posted].filter(Boolean).join(" · ")}
    </p>
  </header>

  {#if release.tags.length || facts.length}
    <div class="facts">
      {#each release.tags as tag (tag)}<span class="tag accent">{tag}</span>{/each}
      {#each facts as fact (fact)}<span class="tag">{fact}</span>{/each}
    </div>
  {/if}

  <ul>
    {#each release.links as link (link.url)}
      <li>
        <span class="host">{link.host}</span>
        {#if link.label !== link.host}<span class="label">{link.label}</span>{/if}
        {#if link.flagged_unsafe}<span class="unsafe">⚠ flagged unsafe by the forum</span>{/if}
        <LinkButton url={link.url} label="Open" confirm={link.flagged_unsafe} />
      </li>
    {/each}
  </ul>

  {#if release.details.length > 1}
    <details>
      <summary>Post text</summary>
      <p class="text">{release.details.join("\n")}</p>
    </details>
  {/if}
</article>

<style>
  article {
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--surface);
  }
  h4 {
    margin: 0;
    font-size: 14px;
    overflow-wrap: anywhere;
  }
  .byline {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
  ul {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .host {
    font-weight: 600;
    color: var(--accent);
  }
  .label {
    color: var(--muted);
    font-size: 13px;
  }
  .unsafe {
    color: var(--warn);
    font-size: 12px;
  }
  li :global(button) {
    margin-left: auto;
  }
  details {
    margin-top: 10px;
  }
  .text {
    white-space: pre-wrap;
    margin: 6px 0 0;
    font-size: 13px;
  }
</style>
