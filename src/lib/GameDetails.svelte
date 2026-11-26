<script lang="ts">
  import type { GameInfo, SteamApp } from "./types";

  /**
   * The game's details for the Overview tab: a facts card, then about,
   * features and languages. `part` picks the column.
   */
  let { forum, steam, part }: { forum: GameInfo | null; steam: SteamApp | null; part: "main" | "side" } = $props();

  const PLATFORM_NAMES: Record<string, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };
  const list = (values: string[] | undefined) => (values?.length ? values.join(", ") : null);

  const about = $derived(forum?.description ?? steam?.short_description ?? null);
  const features = $derived(forum?.features.length ? forum.features : (steam?.categories ?? []));
  const languages = $derived(steam?.languages.length ? steam.languages : (forum?.languages ?? []));
  const facts = $derived(
    (
      [
        ["Developer", list(steam?.developers) ?? forum?.developer],
        ["Publisher", list(steam?.publishers) ?? forum?.publisher],
        [
          "Release date",
          steam?.release_date ? steam.release_date + (steam.coming_soon ? " (coming soon)" : "") : forum?.release_date,
        ],
        ["Genres", list(steam?.genres) ?? list(forum?.genres)],
        ["Platforms", list((steam?.platforms ?? forum?.platforms)?.map((p) => PLATFORM_NAMES[p] ?? p))],
        ["Price", steam ? (steam.is_free ? "Free" : steam.price) : null],
        ["Metacritic", steam?.metacritic?.toString()],
        ["Steam app", (steam?.app_id ?? forum?.steam_app_id)?.toString()],
        ...(forum?.fields.map((f) => [f.key, f.value]) ?? []),
      ] as [string, string | null | undefined][]
    ).filter((row): row is [string, string] => !!row[1]),
  );

  let aboutOpen = $state(false);
</script>

{#if part === "side"}
  {#if facts.length}
    <section class="card">
      <h3>Details</h3>
      <dl>
        {#each facts as [key, value] (key)}
          <dt>{key}</dt>
          <dd class:mono={key === "Steam app"}>{value}</dd>
        {/each}
      </dl>
    </section>
  {/if}
  {#if languages.length}
    <section class="card">
      <h3>Languages</h3>
      <p class="langs">{languages.join(", ")}</p>
    </section>
  {/if}
{:else}
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
  {#if features.length}
    <section class="card">
      <h3>Features</h3>
      <div class="tags">
        {#each features as f (f)}<span class="tag">{f}</span>{/each}
      </div>
    </section>
  {/if}
{/if}

<style>
  .card {
    padding: 16px 18px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }
  h3 {
    margin: 0 0 10px;
    font-size: 14px;
  }
  .text {
    margin: 0;
    white-space: pre-wrap;
    line-height: 1.6;
    font-size: 13px;
    color: #c9cedb;
  }
  .about:not(.open) .text {
    max-height: 16em;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, #000 70%, transparent);
  }
  .more {
    margin-top: 6px;
    padding-left: 0;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 14px;
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
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .langs {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
</style>
