import { api } from "./api";
import type { SteamMatch } from "./types";

/**
 * Steam lookups for card images, two at a time. The backend paces and
 * caches requests to Steam; this queue keeps cards that scrolled out of
 * view from using up Steam's rate limit: when a card's turn comes and it
 * is no longer wanted, it's skipped (resolves `undefined`) and asks again
 * the next time it's visible.
 */

const MAX_ACTIVE = 2;

type Job = {
  name: string;
  wanted: () => boolean;
  resolve: (match: SteamMatch | null | undefined) => void;
  reject: (error: unknown) => void;
};

const known = new Map<string, SteamMatch | null>();
const queue: Job[] = [];
let active = 0;

function pump() {
  while (active < MAX_ACTIVE && queue.length) {
    const job = queue.shift()!;
    if (known.has(job.name)) {
      job.resolve(known.get(job.name));
      continue;
    }
    if (!job.wanted()) {
      job.resolve(undefined);
      continue;
    }
    active++;
    api
      .steamMatch(job.name)
      .then((match) => {
        known.set(job.name, match);
        job.resolve(match);
      })
      .catch(job.reject)
      .finally(() => {
        active--;
        pump();
      });
  }
}

export function cardArt(name: string, wanted: () => boolean): Promise<SteamMatch | null | undefined> {
  if (known.has(name)) return Promise.resolve(known.get(name));
  return new Promise((resolve, reject) => {
    queue.push({ name, wanted, resolve, reject });
    pump();
  });
}
