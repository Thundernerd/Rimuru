import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

import type {
  Artwork,
  CommandError,
  Follow,
  ForumPage,
  GameFirstPage,
  Owned,
  Release,
  SearchPage,
  Settings,
  Session,
  SteamApp,
  SteamMatch,
  TopicSummary,
  Visit,
} from "./types";

export const MAIN_FORUM_ID = 10;

export const FORUM_PAGE_LOADED = "forum-page-loaded";

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
  forumPage: (forumId: number, start: number, sort: string = "activity") =>
    forum<ForumPage>("fetch_forum", { forumId, start, sort }),
  searchTopics: (query: string, start: number, allForums = false) =>
    forum<SearchPage>("search_topics", { query, start, allForums }),
  gameFirstPage: (topicId: number, replies: number | null, refresh = false) =>
    forum<GameFirstPage>("game_first_page", { topicId, replies, refresh }),
  gameReleases: (topicId: number, start: number, replies: number | null, refresh = false) =>
    forum<Release[]>("game_releases", { topicId, start, replies, refresh }),
  steamApp: (appId: number) => invoke<SteamApp | null>("steam_app", { appId }),
  steamMatch: (name: string) => invoke<SteamMatch | null>("steam_match", { name }),
  steamArtwork: (appIds: number[]) => invoke<Record<string, Artwork>>("steam_artwork", { appIds }),
  topicVisits: (topicIds: number[]) => invoke<Record<string, Visit>>("topic_visits", { topicIds }),
  recordVisit: (topic: TopicSummary, releaseIds: number[]) => invoke<Visit>("record_visit", { topic, releaseIds }),
  recentVisits: (limit = 50) => invoke<Visit[]>("recent_visits", { limit }),
  followTopic: (topic: TopicSummary) => invoke<Follow>("follow_topic", { topic }),
  unfollowTopic: (topicId: number) => invoke<void>("unfollow_topic", { topicId }),
  followedTopics: () => invoke<Follow[]>("followed_topics"),
  setForumBookmark: (topicId: number, bookmarked: boolean) =>
    forum<boolean>("set_forum_bookmark", { topicId, bookmarked }),
  importForumBookmarks: () => forum<Follow[]>("import_forum_bookmarks"),
  markOwned: (topicId: number, release: Release) => invoke<Owned>("mark_owned", { topicId, release }),
  clearOwned: (topicId: number) => invoke<void>("clear_owned", { topicId }),
  ownedReleases: () => invoke<Owned[]>("owned_releases"),
  unnotified: (postIds: number[]) => invoke<number[]>("unnotified", { postIds }),
  markNotified: (postIds: number[]) => invoke<void>("mark_notified", { postIds }),
  rememberSteamMatch: (name: string, found: SteamMatch) => invoke<void>("remember_steam_match", { name, found }),
  saveSnapshot: (path: string) => forum<string>("save_snapshot", { path }),
  extractArchive: (path: string) => invoke<string>("extract_archive", { path }),
  showForumWindow: (path?: string) => invoke<void>("show_forum_window", { path }),
  hideForumWindow: () => invoke<void>("hide_forum_window"),
  openInBrowser: (url: string, app?: string | null) => openUrl(url, app ?? undefined),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<Settings>("set_settings", { settings }),
  clearCache: () => invoke<number>("clear_cache"),
  signOut: () => invoke<void>("sign_out"),
  onForumPageLoaded: (handler: () => void) => listen(FORUM_PAGE_LOADED, handler),
};
