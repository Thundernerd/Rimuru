import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

import { api } from "./api";
import { onNewRelease } from "./refresh.svelte";
import type { Release, TopicSummary } from "./types";

export const notifications = $state({ enabled: true });

let permitted: boolean | null = null;

async function allowed(): Promise<boolean> {
  if (permitted === null) {
    permitted = (await isPermissionGranted()) || (await requestPermission()) === "granted";
  }
  return permitted;
}

function describe(release: Release): string {
  if (release.build) return `build ${release.build}`;
  if (release.version) return `v${release.version.replace(/^v/i, "")}`;
  return release.title ?? "";
}

async function notify(topic: TopicSummary, release: Release) {
  if (!notifications.enabled) return;
  try {
    const fresh = await api.unnotified([release.post_id]);
    if (!fresh.length) return;
    await api.markNotified(fresh);
    if (!(await allowed())) return;
    const detail = describe(release);
    sendNotification({ title: topic.name, body: detail ? `New release (${detail})` : "New release" });
  } catch {}
}

export function startNotifications(): () => void {
  return onNewRelease(notify);
}
