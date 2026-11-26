import { SvelteMap } from "svelte/reactivity";

import { api } from "./api";
import type { TopicSummary, Visit } from "./types";

/**
 * When game topics were last opened, and what they had then. Reactive, so
 * cards update when a visit is recorded.
 */
export const visits = new SvelteMap<number, Visit>();

/** Loads stored visits for topics not loaded yet. */
export async function loadVisits(topicIds: number[]) {
  const missing = topicIds.filter((id) => !visits.has(id));
  if (!missing.length) return;
  try {
    const found = await api.topicVisits(missing);
    for (const [id, visit] of Object.entries(found)) visits.set(Number(id), visit);
  } catch {
    // Badges are a nicety; leave them off if this fails.
  }
}

/** The visit before this one, for a topic (null if never opened). */
export async function previousVisit(topicId: number): Promise<Visit | null> {
  await loadVisits([topicId]);
  return visits.get(topicId) ?? null;
}

export async function recordVisit(topic: TopicSummary, releaseIds: number[]) {
  try {
    visits.set(topic.id, await api.recordVisit(topic.id, topic.replies, releaseIds));
  } catch {
    // Not fatal: the next visit just won't know about this one.
  }
}

/** Replies posted since the last visit, or 0 (also for unvisited topics). */
export function newReplies(topic: TopicSummary): number {
  const visit = visits.get(topic.id);
  if (!visit || visit.replies === null || topic.replies === null) return 0;
  return Math.max(0, topic.replies - visit.replies);
}
