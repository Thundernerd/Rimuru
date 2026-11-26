<script lang="ts">
  import LinkButton from "./LinkButton.svelte";
  import type { Release } from "./types";

  /** A release post; `featured` shows it larger, with a download button per host. */
  let {
    release,
    isNew = false,
    featured = false,
  }: { release: Release; isNew?: boolean; featured?: boolean } = $props();

  const facts = $derived(
    [
      // "1.0.0.6" reads as a version number, "16 November 2026 - ..." doesn't.
      release.version && (/^v?\d+(\.\d+)+/i.test(release.version) ? `v${release.version.replace(/^v/i, "")}` : release.version),
      release.build && `Build ${release.build}`,
      release.size,
    ].filter((f): f is string => !!f),
  );
</script>

<article class:new={isNew} class:featured>
  <header>
    <h4>
      {#if isNew}<span class="new-badge">New</span>{/if}
      {release.title ?? "Release"}
    </h4>
    <p class="byline">
      {[release.author && `by ${release.author}`, release.posted].filter(Boolean).join(" · ")}
    </p>
  </header>

  {#if release.tags.length || facts.length}
    <div class="facts">
      {#each release.tags as tag (tag)}<span class="tag accent">{tag}</span>{/each}
      {#each facts as fact (fact)}<span class="tag mono">{fact}</span>{/each}
    </div>
  {/if}

  {#if featured}
    <div class="downloads">
      {#each release.links as link (link.url)}
        <LinkButton
          url={link.url}
          label={(link.flagged_unsafe ? "⚠ " : "") + (link.label !== link.host ? `${link.label} · ${link.host}` : link.host)}
          confirm={link.flagged_unsafe}
          primary={!link.flagged_unsafe}
        />
      {/each}
    </div>
    {#if release.links.some((l) => l.flagged_unsafe)}
      <p class="unsafe note">⚠ The forum flags some of these hosts as unsafe; those need a second click.</p>
    {/if}
  {:else}
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
  {/if}

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
    border: 1px solid var(--border);
    border-left: 3px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }
  article.featured {
    padding: 18px 20px;
    border-color: rgb(63 200 240 / 0.35);
    border-left-color: var(--accent);
    background: linear-gradient(135deg, rgb(63 200 240 / 0.08), transparent 60%), var(--surface);
  }
  article.featured h4 {
    font-size: 16px;
  }
  .downloads {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }
  .note {
    margin: 10px 0 0;
  }
  article.new {
    border-left-color: var(--accent);
  }
  .new-badge {
    display: inline-block;
    margin-right: 6px;
    padding: 0 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    vertical-align: 2px;
    color: var(--accent-text);
    background: var(--accent);
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
