import { api, errorMessage } from "./api";
import { setStatus } from "./status.svelte";
import type { Settings } from "./types";

export const DEFAULT_SETTINGS: Settings = {
  link_browser: null,
  refresh_minutes: 15,
  notifications: true,
  dev_tools: false,
  sync_bookmarks: false,
};

export const settings = $state({ value: { ...DEFAULT_SETTINGS }, loaded: false });

export async function loadSettings() {
  try {
    settings.value = await api.getSettings();
  } catch (e) {
    setStatus(`Couldn't load settings: ${errorMessage(e)}`);
  } finally {
    settings.loaded = true;
  }
}

export async function updateSettings(change: Partial<Settings>) {
  const previous = settings.value;
  settings.value = { ...previous, ...change };
  try {
    settings.value = await api.setSettings(settings.value);
  } catch (e) {
    settings.value = previous;
    setStatus(`Couldn't save settings: ${errorMessage(e)}`);
  }
}

export function openLink(url: string): Promise<void> {
  return api.openInBrowser(url, settings.value.link_browser);
}
