import { api, MAIN_FORUM_ID } from "./api";
import type { ForumPage } from "./types";

/**
 * Pages of the main game forum, shared by Home and Browse so opening the
 * app fetches page 1 once. Kept for a few minutes; failed loads aren't kept.
 */
const MAX_AGE_MS = 5 * 60 * 1000;
const pages = new Map<number, { at: number; page: Promise<ForumPage> }>();

export function forumPage(start: number): Promise<ForumPage> {
  const hit = pages.get(start);
  if (hit && Date.now() - hit.at < MAX_AGE_MS) return hit.page;
  const page = api.forumPage(MAIN_FORUM_ID, start);
  page.catch(() => pages.delete(start));
  pages.set(start, { at: Date.now(), page });
  return page;
}

/** Forget loaded pages, e.g. after signing in. */
export function clearForumPages() {
  pages.clear();
}

/**
 * Title labels ("Early Access", "VR", …) by how many topics have them,
 * most common first; only labels on at least `min` topics.
 */
export function labelCounts(topics: { labels: string[] }[], min = 2): [string, number][] {
  const counts = new Map<string, number>();
  for (const topic of topics) for (const label of new Set(topic.labels)) counts.set(label, (counts.get(label) ?? 0) + 1);
  return [...counts].filter(([, n]) => n >= min).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}
