import { SvelteMap } from "svelte/reactivity";

import { api } from "./api";
import type { Artwork, SteamMatch } from "./types";

/**
 * Steam artwork for game cards, in two steps:
 *
 * 1. Game name → Steam app id, by name search. Lookups run two at a time.
 *    The backend paces and caches requests to Steam; this queue keeps
 *    cards that scrolled out of view from using up Steam's rate limit:
 *    when a card's turn comes and it is no longer wanted, it's skipped and
 *    asks again the next time it's visible.
 * 2. App id → image URLs. Ids are collected for a moment and fetched in
 *    one batch, since Steam returns artwork for many apps per request.
 *
 * Both maps are reactive, so cards update as entries arrive or change,
 * e.g. after a game is opened and its topic links the right Steam app.
 */

const MAX_ACTIVE = 2;
const BATCH_DELAY_MS = 150;

/** Game name → app id; null: no match. Missing: not looked up (yet). */
export const matches = new SvelteMap<string, number | null>();
/** App id → artwork. */
export const artwork = new SvelteMap<number, Artwork>();

// ---------------------------------------------------------------- names

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

/**
 * Looks `name` up on Steam unless it's known. Resolves `false` if the
 * lookup was skipped or failed, so the card can ask again later.
 */
export function requestArt(name: string, wanted: () => boolean): Promise<boolean> {
  if (matches.has(name)) {
    const appId = matches.get(name);
    if (appId != null) requestArtwork(appId);
    return Promise.resolve(true);
  }
  return new Promise((done) => {
    queue.push({ name, wanted, done });
    pump();
  });
}

/** Sets a card's Steam app from a more reliable source than name search. */
export function rememberArt(name: string, found: SteamMatch) {
  if (matches.get(name) === found.app_id) return;
  setMatch(name, found.app_id);
  api.rememberSteamMatch(name, found).catch(() => {});
}

// ---------------------------------------------------------------- artwork

const pending = new Set<number>();
const requested = new Set<number>();
let timer: ReturnType<typeof setTimeout> | null = null;

function requestArtwork(appId: number) {
  if (artwork.has(appId) || requested.has(appId)) return;
  pending.add(appId);
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
    // Ids that didn't come back may be asked for again later.
    .finally(() => ids.forEach((id) => requested.delete(id)));
}

/** The artwork for a game card, once known. */
export function artFor(name: string): Artwork | null {
  const appId = matches.get(name);
  return appId == null ? null : (artwork.get(appId) ?? null);
}
