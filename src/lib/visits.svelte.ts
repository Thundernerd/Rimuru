import { SvelteMap } from "svelte/reactivity";

import { api } from "./api";
import type { TopicSummary, Visit } from "./types";

/**
 * When game topics were last opened, and what they had then. Reactive, so
 * cards and the sidebar update when a visit is recorded.
 */
export const visits = new SvelteMap<number, Visit>();

/** The latest listing of each topic seen in any grid, for current reply counts. */
export const latest = new SvelteMap<number, TopicSummary>();

/** Recently opened games, newest first. */
export const recent = $state({ visits: [] as Visit[], loaded: false });

const RECENT_LIMIT = 50;

export function rememberListed(topics: TopicSummary[]) {
  for (const topic of topics) latest.set(topic.id, topic);
}

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

export async function loadRecent() {
  try {
    recent.visits = await api.recentVisits(RECENT_LIMIT);
    for (const visit of recent.visits) {
      if (visit.topic && !visits.has(visit.topic.id)) visits.set(visit.topic.id, visit);
    }
  } catch {
    // Leave the list empty.
  } finally {
    recent.loaded = true;
  }
}

/** The visit before this one, for a topic (null if never opened). */
export async function previousVisit(topicId: number): Promise<Visit | null> {
  await loadVisits([topicId]);
  return visits.get(topicId) ?? null;
}

export async function recordVisit(topic: TopicSummary, releaseIds: number[]) {
  try {
    const visit = await api.recordVisit(topic, releaseIds);
    visits.set(topic.id, visit);
    recent.visits = [visit, ...recent.visits.filter((v) => v.topic?.id !== topic.id)].slice(0, RECENT_LIMIT);
  } catch {
    // Not fatal: the next visit just won't know about this one.
  }
}

/**
 * Replies posted since the last visit, or 0 (also for unvisited topics).
 * Uses the most recent listing of the topic when there is one.
 */
export function newReplies(topic: TopicSummary): number {
  const visit = visits.get(topic.id);
  const now = latest.get(topic.id) ?? topic;
  if (!visit || visit.replies === null || now.replies === null) return 0;
  return Math.max(0, now.replies - visit.replies);
}
