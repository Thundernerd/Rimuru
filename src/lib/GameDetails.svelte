<script lang="ts">
  import type { GameInfo, SteamApp } from "./types";

  /** Cards with the longer details: about, features, requirements, languages. */
  let { forum, steam }: { forum: GameInfo | null; steam: SteamApp | null } = $props();

  const about = $derived(forum?.description ?? null);
  const features = $derived(forum?.features.length ? forum.features : (steam?.categories ?? []));
  const requirements = $derived(forum?.system_requirements ?? []);
  const languages = $derived(steam?.languages.length ? steam.languages : (forum?.languages ?? []));
  const extra = $derived(forum?.fields ?? []);

  let aboutOpen = $state(false);
</script>

<div class="details">
  {#if about}
    <section class="card about" class:open={aboutOpen}>
      <h3>About the game</h3>
      <p class="text">{about}</p>
      {#if about.length > 600}
        <button type="button" class="subtle more" onclick={() => (aboutOpen = !aboutOpen)}>
          {aboutOpen ? "Show less" : "Show more"}
        </button>
      {/if}
    </section>
  {/if}

  {#if requirements.length}
    <section class="card">
      <h3>System requirements</h3>
      <div class="reqs">
        {#each requirements as set, i (i)}
          <div>
            {#if set.label}<h4>{set.label}</h4>{/if}
            <dl>
              {#each set.items as item, j (j)}
                <dt>{item.key}</dt>
                <dd>{item.value}</dd>
              {/each}
            </dl>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if features.length}
    <section class="card">
      <h3>Features</h3>
      <div class="tags">
        {#each features as f (f)}<span class="tag">{f}</span>{/each}
      </div>
    </section>
  {/if}

  {#if languages.length || extra.length}
    <section class="card">
      {#if languages.length}
        <h3>Languages</h3>
        <p class="langs">{languages.join(", ")}</p>
      {/if}
      {#if extra.length}
        <dl>
          {#each extra as f (f.key)}
            <dt>{f.key}</dt>
            <dd>{f.value}</dd>
          {/each}
        </dl>
      {/if}
    </section>
  {/if}
</div>

<style>
  .details {
    display: grid;
    gap: 16px;
  }
  .card {
    padding: 16px 18px;
    border-radius: 12px;
    background: var(--surface);
  }
  h3 {
    margin: 0 0 10px;
    font-size: 14px;
  }
  h4 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .text {
    margin: 0;
    white-space: pre-wrap;
    line-height: 1.55;
    font-size: 13px;
  }
  .about:not(.open) .text {
    max-height: 14em;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, #000 70%, transparent);
  }
  .more {
    margin-top: 6px;
    padding-left: 0;
  }
  .reqs {
    display: grid;
    gap: 14px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 12px;
    margin: 0;
    font-size: 12px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .langs {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.5;
  }
</style>
