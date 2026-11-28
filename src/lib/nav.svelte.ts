import type { TopicSummary } from "./types";

export type ForumSort = "activity" | "replies" | "views";

export type View =
  | { kind: "home" }
  | { kind: "browse"; forumId: number; sort: ForumSort; label: string | null }
  | { kind: "recent" }
  | { kind: "following" }
  | { kind: "downloads" }
  | { kind: "settings" }
  | { kind: "search"; query: string; allForums: boolean; run: number }
  | { kind: "game"; topic: TopicSummary };

type Entry = { view: View; scroll: number };

export const nav = $state({
  entries: [{ view: { kind: "home" }, scroll: 0 }] as Entry[],
  index: 0,
});

export function current(): View {
  return nav.entries[nav.index].view;
}

export function canGoBack() {
  return nav.index > 0;
}

export function canGoForward() {
  return nav.index < nav.entries.length - 1;
}

let runs = 0;

export function go(view: View, scroll: number) {
  nav.entries[nav.index].scroll = scroll;
  if (sameView(current(), view)) return;
  if (view.kind === "search") view = { ...view, run: ++runs };
  nav.entries = [...nav.entries.slice(0, nav.index + 1), { view, scroll: 0 }];
  nav.index = nav.entries.length - 1;
}

export function replace(view: View) {
  nav.entries[nav.index] = { view, scroll: 0 };
}

export function back(scroll: number) {
  if (!canGoBack()) return;
  nav.entries[nav.index].scroll = scroll;
  nav.index--;
}

export function forward(scroll: number) {
  if (!canGoForward()) return;
  nav.entries[nav.index].scroll = scroll;
  nav.index++;
}

export function savedScroll(): number {
  return nav.entries[nav.index].scroll;
}

export function underlying(): Exclude<View, { kind: "game" }> {
  for (let i = nav.index; i >= 0; i--) {
    const view = nav.entries[i].view;
    if (view.kind !== "game") return view;
  }
  return { kind: "home" };
}

function sameView(a: View, b: View): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "game" && b.kind === "game") return a.topic.id === b.topic.id;
  if (a.kind === "browse" && b.kind === "browse") {
    return a.forumId === b.forumId && a.sort === b.sort && a.label === b.label;
  }
  return a.kind === "home" || a.kind === "recent" || a.kind === "following" || a.kind === "downloads" || a.kind === "settings";
}
