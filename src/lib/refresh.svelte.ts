import { api } from "./api";
import { follows } from "./follows.svelte";
import { clearForumPages, forumPage } from "./forum";
import { noteReleases } from "./owned.svelte";
import type { Release, TopicSummary } from "./types";
import { latest, loadVisits, newReplies, rememberListed, visits } from "./visits.svelte";

export const DEFAULT_REFRESH_MINUTES = 15;

export const refresh = $state({ generation: 0, running: false, lastRun: 0 });

type Listener = (topic: TopicSummary, release: Release) => void;
const listeners = new Set<Listener>();
let timer: ReturnType<typeof setInterval> | null = null;

export function onNewRelease(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function startRefresh(minutes = DEFAULT_REFRESH_MINUTES) {
  stopRefresh();
  timer = setInterval(runRefresh, Math.max(1, minutes) * 60_000);
}

export function stopRefresh() {
  if (timer) clearInterval(timer);
  timer = null;
}

export async function runRefresh() {
  if (refresh.running) return;
  refresh.running = true;
  try {
    clearForumPages();
    const page = await forumPage(0);
    rememberListed(page.topics);
    await loadVisits(page.topics.map((t) => t.id));
    refresh.generation++;
    for (const follow of [...follows.values()]) {
      await checkFollowed(latest.get(follow.topic.id) ?? follow.topic);
    }
  } catch {
  } finally {
    refresh.running = false;
    refresh.lastRun = Date.now();
  }
}

async function checkFollowed(topic: TopicSummary) {
  if (newReplies(topic) <= 0) return;
  const first = await api.gameFirstPage(topic.id, topic.replies);
  let releases = first.releases;
  if (first.per_page && first.last_start > 0) {
    releases = [...(await api.gameReleases(topic.id, first.last_start, topic.replies)), ...releases];
  }
  noteReleases(topic.id, releases);
  const seen = new Set(visits.get(topic.id)?.release_ids ?? []);
  for (const release of releases.filter((r) => !seen.has(r.post_id))) {
    for (const listener of listeners) listener(topic, release);
  }
}
