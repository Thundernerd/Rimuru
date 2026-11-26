import type { TopicSummary } from "./types";

/**
 * Browser-style navigation history for the main area: going somewhere
 * drops the forward entries, back/forward move through the rest. Each
 * entry remembers its scroll position.
 */

export type View =
  | { kind: "home" }
  | { kind: "recent" }
  /** `run` makes a repeated search of the same text a new search. */
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

/** Goes to `view`, remembering where the current one was scrolled to. */
export function go(view: View, scroll: number) {
  nav.entries[nav.index].scroll = scroll;
  // Opening the same game or section again is a no-op, not a new entry.
  if (sameView(current(), view)) return;
  if (view.kind === "search") view = { ...view, run: ++runs };
  nav.entries = [...nav.entries.slice(0, nav.index + 1), { view, scroll: 0 }];
  nav.index = nav.entries.length - 1;
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

/** Where the current entry was scrolled to when it was left. */
export function savedScroll(): number {
  return nav.entries[nav.index].scroll;
}

/** The most recent search at or before the current entry, if any. */
export function activeSearch(): Extract<View, { kind: "search" }> | null {
  for (let i = nav.index; i >= 0; i--) {
    const view = nav.entries[i].view;
    if (view.kind === "search") return view;
    if (view.kind !== "game") return null;
  }
  return null;
}

function sameView(a: View, b: View): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "game" && b.kind === "game") return a.topic.id === b.topic.id;
  return a.kind === "home" || a.kind === "recent";
}
