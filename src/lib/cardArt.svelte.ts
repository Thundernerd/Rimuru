import { SvelteMap, SvelteSet } from "svelte/reactivity";

import { api } from "./api";
import type { Artwork, SteamMatch } from "./types";

const MAX_ACTIVE = 2;
const BATCH_DELAY_MS = 150;

export const matches = new SvelteMap<string, number | null>();
export const artwork = new SvelteMap<number, Artwork>();

// Names whose Steam match is still being looked up, and app ids whose artwork is
// still being fetched, so views can tell art that is coming from art that never will.
const lookingUp = new SvelteSet<string>();
const fetching = new SvelteSet<number>();

type Job = { name: string; wanted: () => boolean; done: (looked: boolean) => void };

const queue: Job[] = [];
let active = 0;

function pump() {
  while (active < MAX_ACTIVE && queue.length) {
    const job = queue.shift()!;
    if (matches.has(job.name)) {
      job.done(true);
      continue;
    }
    if (!job.wanted()) {
      job.done(false);
      continue;
    }
    active++;
    api
      .steamMatch(job.name)
      .then((match) => {
        if (!matches.has(job.name)) setMatch(job.name, match?.app_id ?? null);
        job.done(true);
      })
      .catch(() => job.done(false))
      .finally(() => {
        active--;
        pump();
      });
  }
}

function setMatch(name: string, appId: number | null) {
  matches.set(name, appId);
  if (appId !== null) requestArtwork(appId);
}

export function requestArt(name: string, wanted: () => boolean): Promise<boolean> {
  if (matches.has(name)) {
    const appId = matches.get(name);
    if (appId != null) requestArtwork(appId);
    return Promise.resolve(true);
  }
  return new Promise((resolve) => {
    lookingUp.add(name);
    const done = (looked: boolean) => {
      if (!queue.some((job) => job.name === name)) lookingUp.delete(name);
      resolve(looked);
    };
    queue.push({ name, wanted, done });
    pump();
  });
}

export function rememberArt(name: string, found: SteamMatch) {
  if (matches.get(name) === found.app_id) return;
  setMatch(name, found.app_id);
  api.rememberSteamMatch(name, found).catch(() => {});
}

const pending = new Set<number>();
const requested = new Set<number>();
let timer: ReturnType<typeof setTimeout> | null = null;

function requestArtwork(appId: number) {
  if (artwork.has(appId) || requested.has(appId)) return;
  pending.add(appId);
  fetching.add(appId);
  timer ??= setTimeout(flush, BATCH_DELAY_MS);
}

function flush() {
  timer = null;
  const ids = [...pending];
  pending.clear();
  ids.forEach((id) => requested.add(id));
  api
    .steamArtwork(ids)
    .then((found) => {
      for (const [id, art] of Object.entries(found)) artwork.set(Number(id), art);
    })
    .catch(() => {})
    .finally(() =>
      ids.forEach((id) => {
        requested.delete(id);
        if (!pending.has(id)) fetching.delete(id);
      }),
    );
}

export function artFor(name: string): Artwork | null {
  const appId = matches.get(name);
  return appId == null ? null : (artwork.get(appId) ?? null);
}

/** Whether art for `name` is still on its way: its match or its artwork is being fetched. */
export function artLoading(name: string): boolean {
  if (lookingUp.has(name)) return true;
  const appId = matches.get(name);
  return appId != null && fetching.has(appId);
}
