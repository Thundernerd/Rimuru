<script lang="ts">
  import { errorMessage } from "./api";
  import FeaturedCarousel from "./FeaturedCarousel.svelte";
  import { followedTopics } from "./follows.svelte";
  import { forumPage, labelCounts } from "./forum";
  import LabelChips from "./LabelChips.svelte";
  import type { View } from "./nav.svelte";
  import Shelf from "./Shelf.svelte";
  import type { TopicSummary } from "./types";
  import { latest, loadVisits, newReplies, recent, rememberListed } from "./visits.svelte";

  /** Featured games, label chips and shelves, from page 1 of the forum. */
  let {
    onselect,
    onnavigate,
  }: { onselect: (topic: TopicSummary) => void; onnavigate: (view: View) => void } = $props();

  const SHELF = 20;

  let topics = $state<TopicSummary[]>([]);
  let error = $state<string | null>(null);
  let attempt = $state(0);

  $effect(() => {
    void attempt;
    error = null;
    forumPage(0)
      .then((page) => {
        topics = page.topics;
        rememberListed(page.topics);
        loadVisits(page.topics.map((t) => t.id));
      })
      .catch((e) => (error = errorMessage(e)));
  });

  const labels = $derived(labelCounts(topics).slice(0, 10));
  const viewed = $derived(
    recent.visits
      .map((v) => v.topic && (latest.get(v.topic.id) ?? v.topic))
      .filter((t): t is TopicSummary => !!t),
  );
  const following = $derived(followedTopics());
  // Followed games have their own shelf, so leave them out of this one.
  const withNewReplies = $derived(viewed.filter((t) => newReplies(t) > 0 && !following.some((f) => f.id === t.id)));
</script>

<div class="home">
  {#if error}
    <div class="message">
      <p>Couldn't load the latest games: {error}</p>
      <button type="button" onclick={() => attempt++}>Try again</button>
    </div>
  {:else}
    <FeaturedCarousel {topics} {onselect} />

    <LabelChips {labels} onselect={(label) => onnavigate({ kind: "browse", label })} />

    <Shelf
      title="Following"
      subtitle="New replies first"
      topics={following.slice(0, SHELF)}
      {onselect}
      onseeall={() => onnavigate({ kind: "following" })}
    />
    <Shelf
      title="New since your last visit"
      subtitle="Games you opened that have new replies"
      topics={withNewReplies.slice(0, SHELF)}
      {onselect}
    />
    <Shelf
      title="Updated recently"
      subtitle="Latest activity in the main forum"
      topics={topics.slice(0, SHELF)}
      {onselect}
      onseeall={() => onnavigate({ kind: "browse", label: null })}
    />
    <Shelf
      title="Recently viewed"
      topics={viewed.slice(0, SHELF)}
      {onselect}
      onseeall={() => onnavigate({ kind: "recent" })}
    />
  {/if}
</div>

<style>
  .home {
    display: grid;
    gap: 32px;
    grid-template-columns: minmax(0, 1fr);
  }
  .message {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    color: var(--muted);
  }
</style>
