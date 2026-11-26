import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

import type { CommandError, ForumPage, GameFirstPage, Release, Session, SteamApp, SteamMatch } from "./types";

/** The forum's main game listing ("Main Forum"). */
export const MAIN_FORUM_ID = 10;

export const FORUM_PAGE_LOADED = "forum-page-loaded";

/** Resolves on the forum window's next finished page load. */
function nextForumPageLoad(timeoutMs = 20000): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      unlisten.then((u) => u());
      reject({ kind: "timeout", message: "the forum window did not finish loading" } satisfies CommandError);
    }, timeoutMs);
    const unlisten = listen(FORUM_PAGE_LOADED, () => {
      clearTimeout(timer);
      unlisten.then((u) => u());
      resolve();
    });
  });
}

/**
 * Invokes a forum command. If the site's security check is in the way, the
 * backend has already told the forum window to reload; wait for that and
 * retry once.
 */
async function forum<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if ((e as CommandError).kind !== "security_check") throw e;
    await nextForumPageLoad();
    return await invoke<T>(cmd, args);
  }
}

export function errorMessage(e: unknown): string {
  return (e as CommandError)?.message ?? String(e);
}

export const api = {
  session: () => forum<Session>("forum_session"),
  forumPage: (forumId: number, start: number) => forum<ForumPage>("fetch_forum", { forumId, start }),
  gameFirstPage: (topicId: number, replies: number | null) =>
    forum<GameFirstPage>("game_first_page", { topicId, replies }),
  gameReleases: (topicId: number, start: number, replies: number | null) =>
    forum<Release[]>("game_releases", { topicId, start, replies }),
  steamApp: (appId: number) => invoke<SteamApp | null>("steam_app", { appId }),
  steamMatch: (name: string) => invoke<SteamMatch | null>("steam_match", { name }),
  rememberSteamMatch: (name: string, found: SteamMatch) => invoke<void>("remember_steam_match", { name, found }),
  saveSnapshot: (path: string) => forum<string>("save_snapshot", { path }),
  showForumWindow: (path?: string) => invoke<void>("show_forum_window", { path }),
  hideForumWindow: () => invoke<void>("hide_forum_window"),
  openInBrowser: (url: string) => openUrl(url),
  onForumPageLoaded: (handler: () => void) => listen(FORUM_PAGE_LOADED, handler),
};
