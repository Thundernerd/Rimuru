import { api, errorMessage } from "./api";
import { chooseExtractDir } from "./extract.svelte";
import { settings } from "./settings.svelte";
import { setStatus } from "./status.svelte";
import type { DownloadProgress } from "./types";

type Running = { id: number; progress: DownloadProgress };

/** The in-app downloads running now, by link. */
export const downloads = $state<{ running: Record<string, Running>; done: Record<string, string> }>({
  running: {},
  done: {},
});

let nextId = 1;

/**
 * Downloads a post's download link into the folder set in Settings, asking for that folder
 * first when none is set. Returns where the file went, or null when it failed or was cancelled;
 * either way the status bar says how it went.
 */
export async function downloadLink(url: string): Promise<string | null> {
  if (downloads.running[url]) return null;
  if (!settings.value.extract_dir && !(await chooseExtractDir())) return null;
  const id = nextId++;
  downloads.running[url] = { id, progress: { received: 0, total: null } };
  delete downloads.done[url];
  try {
    const path = await api.downloadFile(id, url, (progress) => {
      const running = downloads.running[url];
      if (running?.id === id) running.progress = progress;
    });
    downloads.done[url] = path;
    setStatus(`Downloaded to ${path}`);
    return path;
  } catch (e) {
    setStatus(`Couldn't download: ${errorMessage(e)}`);
    return null;
  } finally {
    delete downloads.running[url];
  }
}

/** Cancels the running download of a link; its partial file is removed. */
export async function cancelDownload(url: string): Promise<void> {
  const running = downloads.running[url];
  if (!running) return;
  try {
    await api.cancelDownload(running.id);
  } catch (e) {
    setStatus(`Couldn't cancel the download: ${errorMessage(e)}`);
  }
}
