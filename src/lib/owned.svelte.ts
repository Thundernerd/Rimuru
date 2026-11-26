import { SvelteMap } from "svelte/reactivity";

import { api, errorMessage } from "./api";
import { setStatus } from "./status.svelte";
import type { Owned, Release } from "./types";

export const owned = new SvelteMap<number, Owned>();
export const newest = new SvelteMap<number, Release>();

type Comparable = { post_id: number; build: string | null };

export function isNewer(a: Comparable, b: Comparable): boolean {
  const buildA = a.build && /^\d+$/.test(a.build) ? Number(a.build) : null;
  const buildB = b.build && /^\d+$/.test(b.build) ? Number(b.build) : null;
  if (buildA !== null && buildB !== null && buildA !== buildB) return buildA > buildB;
  return a.post_id > b.post_id;
}

export async function loadOwned() {
  try {
    for (const entry of await api.ownedReleases()) owned.set(entry.topic_id, entry);
  } catch (e) {
    setStatus(`Couldn't load the releases you have: ${errorMessage(e)}`);
  }
}

export function noteReleases(topicId: number, releases: Release[]) {
  const latest = releases.reduce<Release | null>((best, r) => (!best || isNewer(r, best) ? r : best), null);
  const known = newest.get(topicId);
  if (latest && (!known || isNewer(latest, known))) newest.set(topicId, latest);
}

export function isOwned(topicId: number, postId: number): boolean {
  return owned.get(topicId)?.post_id === postId;
}

export function updateAvailable(topicId: number): Release | null {
  const have = owned.get(topicId);
  const latest = newest.get(topicId);
  return have && latest && isNewer(latest, have) ? latest : null;
}

export async function toggleOwned(topicId: number, release: Release) {
  const was = owned.get(topicId);
  try {
    if (was?.post_id === release.post_id) {
      owned.delete(topicId);
      await api.clearOwned(topicId);
    } else {
      owned.set(topicId, await api.markOwned(topicId, release));
    }
  } catch (e) {
    if (was) owned.set(topicId, was);
    else owned.delete(topicId);
    setStatus(`Couldn't update the release you have: ${errorMessage(e)}`);
  }
}
