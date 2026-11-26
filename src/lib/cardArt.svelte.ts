import { SvelteMap } from "svelte/reactivity";

import { api } from "./api";
import type { SteamMatch } from "./types";

/**
 * Steam artwork for game cards, by game name.
 *
 * Lookups run two at a time. The backend paces and caches requests to
 * Steam; this queue keeps cards that scrolled out of view from using up
 * Steam's rate limit: when a card's turn comes and it is no longer wanted,
 * it's skipped and asks again the next time it's visible.
 *
 * `art` is reactive, so a card updates when its entry changes, e.g. after
 * the game is opened and its topic links the right Steam app.
 */

const MAX_ACTIVE = 2;

/** null: looked up, no match. Missing: not looked up (yet). */
export const art = new SvelteMap<string, SteamMatch | null>();

type Job = { name: string; wanted: () => boolean; done: (looked: boolean) => void };

const queue: Job[] = [];
let active = 0;

function pump() {
  while (active < MAX_ACTIVE && queue.length) {
    const job = queue.shift()!;
    if (art.has(job.name)) {
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
        if (!art.has(job.name)) art.set(job.name, match);
        job.done(true);
      })
      .catch(() => job.done(false))
      .finally(() => {
        active--;
        pump();
      });
  }
}

/**
 * Looks `name` up on Steam unless it's known. Resolves `false` if the
 * lookup was skipped or failed, so the card can ask again later.
 */
export function requestArt(name: string, wanted: () => boolean): Promise<boolean> {
  if (art.has(name)) return Promise.resolve(true);
  return new Promise((done) => {
    queue.push({ name, wanted, done });
    pump();
  });
}

/** Sets a card's Steam app from a more reliable source than name search. */
export function rememberArt(name: string, found: SteamMatch) {
  const known = art.get(name);
  if (known?.app_id === found.app_id && known.capsule_image) return;
  art.set(name, found);
  api.rememberSteamMatch(name, found).catch(() => {});
}
