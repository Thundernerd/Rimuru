import { api, errorMessage } from "./api";
import { chooseExtractDir } from "./extract.svelte";
import { settings } from "./settings.svelte";
import { setStatus } from "./status.svelte";
import {
  clearFinished,
  failEntry,
  finishEntry,
  handOffEntry,
  progressEntry,
  startEntry,
  viaJdownloader,
  type DownloadEntry,
} from "./download";
import type { DownloadLink, DownloadProgress, TopicSummary } from "./types";

type Running = { id: number; progress: DownloadProgress; via?: "jdownloader" };

/** The in-app downloads running now, by link. */
export const downloads = $state<{
  running: Record<string, Running>;
  done: Record<string, string>;
  entries: DownloadEntry[];
}>({
  running: {},
  done: {},
  entries: [],
});

/** Drops finished, failed and cancelled downloads from the Downloads view's list. */
export function clearFinishedDownloads() {
  downloads.entries = clearFinished(downloads.entries);
}

let nextId = 1;

/**
 * Downloads a post's download link into the folder set in Settings, asking for that folder
 * first when none is set. Returns where the file went, or null when it failed or was cancelled;
 * either way the status bar says how it went.
 */
export async function downloadLink(link: DownloadLink, topic: TopicSummary | null): Promise<string | null> {
  const url = link.url;
  if (downloads.running[url]) return null;
  if (!settings.value.extract_dir && !(await chooseExtractDir())) return null;
  const id = nextId++;
  downloads.running[url] = { id, progress: { received: 0, total: null } };
  delete downloads.done[url];
  downloads.entries = startEntry(downloads.entries, { id, url, host: link.host, label: link.label, topic });
  const onProgress = (progress: DownloadProgress) => {
    const running = downloads.running[url];
    if (running?.id === id) running.progress = progress;
    downloads.entries = progressEntry(downloads.entries, id, progress);
  };
  try {
    let path: string;
    try {
      path = await api.downloadFile(id, url, onProgress);
    } catch (e) {
      if (!settings.value.jdownloader || (e as { kind?: string })?.kind !== "download_unsupported") throw e;
      // The setting is on and the link needs a browser: JDownloader is the fallback.
      downloads.entries = viaJdownloader(downloads.entries, id);
      const running = downloads.running[url];
      if (running) running.via = "jdownloader";
      const outcome = await api.sendToJdownloader(id, url, topic?.name ?? null, topic?.url ?? null, onProgress);
      if ("handedOff" in outcome) {
        downloads.entries = handOffEntry(downloads.entries, id);
        setStatus("Sent to JDownloader; progress shows there");
        return null;
      }
      path = outcome.path;
    }
    downloads.done[url] = path;
    downloads.entries = finishEntry(downloads.entries, id, path);
    setStatus(`Downloaded to ${path}${downloads.entries.find((e) => e.id === id)?.via ? " via JDownloader" : ""}`);
    return path;
  } catch (e) {
    downloads.entries = failEntry(
      downloads.entries,
      id,
      errorMessage(e),
      (e as { kind?: string })?.kind === "download_cancelled",
    );
    setStatus(`Couldn't download: ${errorMessage(e)}. "Open in browser" still works.`);
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
