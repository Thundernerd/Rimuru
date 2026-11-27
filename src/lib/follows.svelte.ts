import { SvelteMap } from "svelte/reactivity";

import { api, errorMessage } from "./api";
import { session } from "./session.svelte";
import { settings } from "./settings.svelte";
import { setStatus } from "./status.svelte";
import type { Follow, TopicSummary } from "./types";
import { latest, newReplies } from "./visits.svelte";

export const follows = new SvelteMap<number, Follow>();

export async function loadFollows() {
  try {
    for (const follow of await api.followedTopics()) follows.set(follow.topic.id, follow);
  } catch (e) {
    setStatus(`Couldn't load followed games: ${errorMessage(e)}`);
  }
}

export function isFollowed(topicId: number): boolean {
  return follows.has(topicId);
}

function syncingBookmarks(): boolean {
  return settings.value.sync_bookmarks && session.current?.state === "logged_in";
}

export async function toggleFollow(topic: TopicSummary) {
  const was = follows.get(topic.id);
  try {
    if (was) {
      follows.delete(topic.id);
      await api.unfollowTopic(topic.id);
    } else {
      follows.set(topic.id, { topic, followed_at: Date.now() / 1000 });
      follows.set(topic.id, await api.followTopic(topic));
    }
  } catch (e) {
    if (was) follows.set(topic.id, was);
    else follows.delete(topic.id);
    setStatus(`Couldn't update following: ${errorMessage(e)}`);
    return;
  }
  if (syncingBookmarks()) await pushBookmark(topic.id, !was);
}

// The local follow is kept either way: the forum bookmark only mirrors it.
async function pushBookmark(topicId: number, bookmarked: boolean) {
  try {
    await api.setForumBookmark(topicId, bookmarked);
  } catch (e) {
    setStatus(`Couldn't update the forum bookmark: ${errorMessage(e)}`);
  }
}

let importedFor: string | null = null;

/** Follows the topics bookmarked on the forum account, once per signed-in user. */
export async function importBookmarks() {
  const current = session.current;
  if (!syncingBookmarks() || current?.state !== "logged_in") {
    if (current?.state === "logged_out") importedFor = null;
    return;
  }
  const who = current.username ?? "";
  if (importedFor === who) return;
  importedFor = who;
  try {
    for (const follow of await api.importForumBookmarks()) follows.set(follow.topic.id, follow);
  } catch (e) {
    importedFor = null;
    setStatus(`Couldn't import forum bookmarks: ${errorMessage(e)}`);
  }
}

export function followedTopics(): TopicSummary[] {
  return [...follows.values()]
    .sort((a, b) => b.followed_at - a.followed_at)
    .map((f) => latest.get(f.topic.id) ?? f.topic)
    .sort((a, b) => Number(newReplies(b) > 0) - Number(newReplies(a) > 0));
}
