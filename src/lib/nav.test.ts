import { beforeEach, describe, expect, it, vi } from "vitest";

import type { TopicSummary } from "./types";

type Nav = typeof import("./nav.svelte");
let nav: Nav;

const topic = (id: number): TopicSummary => ({
  id,
  kind: "normal",
  title: `[Info] Game ${id}`,
  name: `Game ${id}`,
  prefixes: ["Info"],
  labels: [],
  url: `https://cs.rin.ru/forum/viewtopic.php?t=${id}`,
  author: null,
  created: null,
  replies: 1,
  views: 1,
  last_post_at: null,
  last_poster: null,
});

beforeEach(async () => {
  vi.resetModules();
  nav = await import("./nav.svelte");
});

describe("navigation history", () => {
  it("starts at home with nowhere to go", () => {
    expect(nav.current()).toEqual({ kind: "home" });
    expect(nav.canGoBack()).toBe(false);
    expect(nav.canGoForward()).toBe(false);
  });

  it("goes back and forward, remembering scroll positions", () => {
    nav.go({ kind: "recent" }, 120);
    nav.go({ kind: "game", topic: topic(1) }, 340);
    expect(nav.current().kind).toBe("game");

    nav.back(55);
    expect(nav.current()).toEqual({ kind: "recent" });
    expect(nav.savedScroll()).toBe(340);

    nav.back(0);
    expect(nav.current()).toEqual({ kind: "home" });
    expect(nav.savedScroll()).toBe(120);

    nav.forward(0);
    nav.forward(0);
    expect(nav.current().kind).toBe("game");
    expect(nav.savedScroll()).toBe(55);
  });

  it("drops forward entries when going somewhere new", () => {
    nav.go({ kind: "recent" }, 0);
    nav.back(0);
    nav.go({ kind: "following" }, 0);
    expect(nav.canGoForward()).toBe(false);
    nav.back(0);
    expect(nav.current()).toEqual({ kind: "home" });
  });

  it("ignores going to the page that is already open", () => {
    nav.go({ kind: "game", topic: topic(1) }, 0);
    nav.go({ kind: "game", topic: topic(1) }, 0);
    nav.back(0);
    expect(nav.current()).toEqual({ kind: "home" });
  });

  it("numbers searches so the same text can be searched again", () => {
    nav.go({ kind: "search", query: "slot", allForums: false, run: 0 }, 0);
    const first = nav.current();
    nav.go({ kind: "search", query: "slot", allForums: false, run: 0 }, 0);
    const second = nav.current();
    expect(first.kind === "search" && second.kind === "search" && second.run > first.run).toBe(true);
  });

  it("replaces the current entry without adding history", () => {
    nav.go({ kind: "browse", forumId: 10, sort: "activity", label: null }, 0);
    nav.replace({ kind: "browse", forumId: 10, sort: "activity", label: "VR" });
    expect(nav.current()).toEqual({ kind: "browse", forumId: 10, sort: "activity", label: "VR" });
    nav.back(0);
    expect(nav.current()).toEqual({ kind: "home" });
  });

  it("knows the page a game was opened from", () => {
    nav.go({ kind: "search", query: "slot", allForums: true, run: 0 }, 0);
    nav.go({ kind: "game", topic: topic(1) }, 0);
    nav.go({ kind: "game", topic: topic(2) }, 0);
    expect(nav.underlying()).toMatchObject({ kind: "search", query: "slot" });
  });

  it("treats the downloads view like the other plain views", () => {
    nav.go({ kind: "downloads" }, 0);
    nav.go({ kind: "downloads" }, 5);
    expect(nav.current()).toEqual({ kind: "downloads" });
    expect(nav.canGoBack()).toBe(true);
    nav.back(0);
    expect(nav.current()).toEqual({ kind: "home" });
    expect(nav.canGoBack()).toBe(false);
  });
});
