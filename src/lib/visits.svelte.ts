import { SvelteMap } from "svelte/reactivity";

import { api } from "./api";
import type { TopicSummary, Visit } from "./types";

export const visits = new SvelteMap<number, Visit>();

export const latest = new SvelteMap<number, TopicSummary>();

export const recent = $state({ visits: [] as Visit[], loaded: false });

const RECENT_LIMIT = 50;

export function rememberListed(topics: TopicSummary[]) {
  for (const topic of topics) latest.set(topic.id, topic);
}

export async function loadVisits(topicIds: number[]) {
  const missing = topicIds.filter((id) => !visits.has(id));
  if (!missing.length) return;
  try {
    const found = await api.topicVisits(missing);
    for (const [id, visit] of Object.entries(found)) visits.set(Number(id), visit);
  } catch {}
}

export async function loadRecent() {
  try {
    recent.visits = await api.recentVisits(RECENT_LIMIT);
    for (const visit of recent.visits) {
      if (visit.topic && !visits.has(visit.topic.id)) visits.set(visit.topic.id, visit);
    }
  } catch {} finally {
    recent.loaded = true;
  }
}

export async function previousVisit(topicId: number): Promise<Visit | null> {
  await loadVisits([topicId]);
  return visits.get(topicId) ?? null;
}

export async function recordVisit(topic: TopicSummary, releaseIds: number[]) {
  try {
    const visit = await api.recordVisit(topic, releaseIds);
    visits.set(topic.id, visit);
    recent.visits = [visit, ...recent.visits.filter((v) => v.topic?.id !== topic.id)].slice(0, RECENT_LIMIT);
  } catch {}
}

/** Empties "Recently viewed". The `visits` map stays, as it holds the new-reply baselines. */
export async function clearRecent() {
  await api.clearVisits();
  recent.visits = [];
}

export function newReplies(topic: TopicSummary): number {
  const visit = visits.get(topic.id);
  const now = latest.get(topic.id) ?? topic;
  if (!visit || visit.replies === null || now.replies === null) return 0;
  return Math.max(0, now.replies - visit.replies);
}
