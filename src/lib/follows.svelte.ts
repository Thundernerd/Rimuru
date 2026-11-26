import { SvelteMap } from "svelte/reactivity";

import { api, errorMessage } from "./api";
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
  }
}

export function followedTopics(): TopicSummary[] {
  return [...follows.values()]
    .sort((a, b) => b.followed_at - a.followed_at)
    .map((f) => latest.get(f.topic.id) ?? f.topic)
    .sort((a, b) => Number(newReplies(b) > 0) - Number(newReplies(a) > 0));
}
