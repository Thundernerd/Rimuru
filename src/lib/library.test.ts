import { beforeEach, describe, expect, it, vi } from "vitest";

import type { Release, TopicSummary, Visit } from "./types";

const api = vi.hoisted(() => ({
  followTopic: vi.fn(),
  unfollowTopic: vi.fn(),
  followedTopics: vi.fn(),
  markOwned: vi.fn(),
  clearOwned: vi.fn(),
  ownedReleases: vi.fn(),
}));

vi.mock("./api", () => ({
  api,
  errorMessage: (e: unknown) => String((e as { message?: string })?.message ?? e),
}));

type Follows = typeof import("./follows.svelte");
type Visits = typeof import("./visits.svelte");
type OwnedStore = typeof import("./owned.svelte");
type Status = typeof import("./status.svelte");
let follows: Follows;
let visits: Visits;
let owned: OwnedStore;
let status: Status;

const topic = (id: number, replies: number | null = 10): TopicSummary => ({
  id,
  kind: "normal",
  title: `[Info] Game ${id}`,
  name: `Game ${id}`,
  prefixes: ["Info"],
  labels: [],
  url: `https://cs.rin.ru/forum/viewtopic.php?t=${id}`,
  author: null,
  created: null,
  replies,
  views: 1,
  last_post_at: null,
  last_poster: null,
});

const visit = (t: TopicSummary, replies: number | null): Visit => ({
  replies,
  release_ids: [],
  visited_at: 1,
  topic: t,
});

const release = (post_id: number, build: string | null): Release => ({
  post_id,
  author: null,
  posted: null,
  title: `Release ${post_id}`,
  details: [],
  version: null,
  build,
  size: null,
  tags: [],
  links: [],
});

beforeEach(async () => {
  vi.resetModules();
  Object.values(api).forEach((fn) => fn.mockReset());
  visits = await import("./visits.svelte");
  follows = await import("./follows.svelte");
  owned = await import("./owned.svelte");
  status = await import("./status.svelte");
});

describe("new replies", () => {
  it("counts replies since the last visit, using the latest listing", () => {
    const game = topic(1, 10);
    visits.visits.set(1, visit(game, 7));
    expect(visits.newReplies(game)).toBe(3);
    visits.rememberListed([topic(1, 15)]);
    expect(visits.newReplies(game)).toBe(8);
  });

  it("is zero for unvisited topics and unknown counts", () => {
    expect(visits.newReplies(topic(2, 10))).toBe(0);
    visits.visits.set(3, visit(topic(3), null));
    expect(visits.newReplies(topic(3, 10))).toBe(0);
  });

  it("never goes negative", () => {
    visits.visits.set(4, visit(topic(4), 20));
    expect(visits.newReplies(topic(4, 18))).toBe(0);
  });
});

describe("following", () => {
  it("follows and unfollows", async () => {
    const game = topic(1);
    api.followTopic.mockResolvedValue({ topic: game, followed_at: 100 });
    api.unfollowTopic.mockResolvedValue(undefined);

    await follows.toggleFollow(game);
    expect(follows.isFollowed(1)).toBe(true);
    expect(api.followTopic).toHaveBeenCalledWith(game);

    await follows.toggleFollow(game);
    expect(follows.isFollowed(1)).toBe(false);
    expect(api.unfollowTopic).toHaveBeenCalledWith(1);
  });

  it("rolls back and reports when saving fails", async () => {
    api.followTopic.mockRejectedValue(new Error("disk full"));
    await follows.toggleFollow(topic(1));
    expect(follows.isFollowed(1)).toBe(false);
    expect(status.status.text).toContain("disk full");
  });

  it("lists games with new replies first, then the most recently followed", () => {
    follows.follows.set(1, { topic: topic(1), followed_at: 300 });
    follows.follows.set(2, { topic: topic(2), followed_at: 200 });
    follows.follows.set(3, { topic: topic(3), followed_at: 100 });
    visits.visits.set(3, visit(topic(3), 5));
    expect(follows.followedTopics().map((t) => t.id)).toEqual([3, 1, 2]);
  });
});

describe("releases you have", () => {
  it("compares builds, falling back to post order", () => {
    expect(owned.isNewer(release(1, "200"), release(2, "100"))).toBe(true);
    expect(owned.isNewer(release(2, "100"), release(1, "200"))).toBe(false);
    expect(owned.isNewer(release(2, null), release(1, "200"))).toBe(true);
    expect(owned.isNewer(release(2, "16 November"), release(1, "200"))).toBe(true);
  });

  it("shows an update only when a newer release than yours is known", async () => {
    api.markOwned.mockImplementation(async (topicId: number, r: Release) => ({
      topic_id: topicId,
      post_id: r.post_id,
      build: r.build,
      version: r.version,
      title: r.title,
      marked_at: 1,
    }));
    owned.noteReleases(1, [release(10, "100")]);
    await owned.toggleOwned(1, release(10, "100"));
    expect(owned.isOwned(1, 10)).toBe(true);
    expect(owned.updateAvailable(1)).toBeNull();

    owned.noteReleases(1, [release(11, "150"), release(10, "100")]);
    expect(owned.updateAvailable(1)?.post_id).toBe(11);

    await owned.toggleOwned(1, release(11, "150"));
    expect(owned.updateAvailable(1)).toBeNull();
  });

  it("keeps the newest release seen across pages", () => {
    owned.noteReleases(1, [release(20, "300")]);
    owned.noteReleases(1, [release(5, "100")]);
    expect(owned.newest.get(1)?.post_id).toBe(20);
  });
});
