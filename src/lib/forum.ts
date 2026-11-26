import { api, MAIN_FORUM_ID } from "./api";
import type { ForumSort } from "./nav.svelte";
import type { ForumPage, ForumSection } from "./types";

const MAX_AGE_MS = 5 * 60 * 1000;
const pages = new Map<string, { at: number; page: Promise<ForumPage> }>();

export function forumListing(forumId: number, sort: ForumSort, start: number): Promise<ForumPage> {
  const key = `${forumId}:${sort}:${start}`;
  const hit = pages.get(key);
  if (hit && Date.now() - hit.at < MAX_AGE_MS) return hit.page;
  const page = api.forumPage(forumId, start, sort);
  page.catch(() => pages.delete(key));
  pages.set(key, { at: Date.now(), page });
  return page;
}

export function forumPage(start: number): Promise<ForumPage> {
  return forumListing(MAIN_FORUM_ID, "activity", start);
}

export function clearForumPages() {
  pages.clear();
}

export function englishSections(sections: ForumSection[]): ForumSection[] {
  const start = sections.findIndex((s) => s.depth === 0 && /english/i.test(s.name));
  if (start === -1) return sections.filter((s) => s.depth > 0);
  const end = sections.findIndex((s, i) => i > start && s.depth === 0);
  return sections.slice(start + 1, end === -1 ? undefined : end);
}

export function labelCounts(topics: { labels: string[] }[], min = 2): [string, number][] {
  const counts = new Map<string, number>();
  for (const topic of topics) for (const label of new Set(topic.labels)) counts.set(label, (counts.get(label) ?? 0) + 1);
  return [...counts].filter(([, n]) => n >= min).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}
