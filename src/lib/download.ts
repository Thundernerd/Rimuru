import type { DownloadProgress, TopicSummary } from "./types";
import { ARCHIVE_EXTENSIONS } from "./archives";

/** A byte count for people: `512 B`, `3.4 MB`. */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return unit === 0 ? `${value} ${units[0]}` : `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
}

/** A download's progress as a percentage when its size is known, or the bytes so far. */
export function formatProgress(p: DownloadProgress): string {
  if (p.total) return `${Math.min(100, Math.floor((p.received / p.total) * 100))}%`;
  return formatBytes(p.received);
}

/** Whether a downloaded file is an archive Rimuru can extract. */
export function isArchive(path: string): boolean {
  const name = path.split(/[\\/]/).pop()?.toLowerCase() ?? "";
  return ARCHIVE_EXTENSIONS.some((ext) => name.endsWith(`.${ext}`));
}

export type DownloadState = "running" | "done" | "handed_off" | "failed" | "cancelled";

/** One download of this session, as the Downloads view lists it. */
export type DownloadEntry = {
  id: number;
  url: string;
  host: string;
  label: string;
  topic: TopicSummary | null;
  state: DownloadState;
  progress: DownloadProgress;
  path?: string;
  error?: string;
  /** Set when JDownloader, not Rimuru, is doing the download. */
  via?: "jdownloader";
};

/** Adds a new running entry. */
export function startEntry(
  entries: DownloadEntry[],
  entry: Pick<DownloadEntry, "id" | "url" | "host" | "label" | "topic">,
): DownloadEntry[] {
  return [...entries, { ...entry, state: "running", progress: { received: 0, total: null } }];
}

function update(entries: DownloadEntry[], id: number, patch: Partial<DownloadEntry>): DownloadEntry[] {
  return entries.map((e) => (e.id === id ? { ...e, ...patch } : e));
}

/** Records a running entry's progress. */
export function progressEntry(entries: DownloadEntry[], id: number, progress: DownloadProgress): DownloadEntry[] {
  return update(entries, id, { progress });
}

/** Marks an entry done, with where its file went. */
export function finishEntry(entries: DownloadEntry[], id: number, path: string): DownloadEntry[] {
  return update(entries, id, { state: "done", path });
}

/** Marks an entry as being downloaded by JDownloader. */
export function viaJdownloader(entries: DownloadEntry[], id: number): DownloadEntry[] {
  return update(entries, id, { via: "jdownloader", progress: { received: 0, total: null } });
}

/** Marks an entry handed to JDownloader, which shows the progress from here on. */
export function handOffEntry(entries: DownloadEntry[], id: number): DownloadEntry[] {
  return update(entries, id, { state: "handed_off" });
}

/** Marks an entry failed, or cancelled when the error says the user cancelled it. */
export function failEntry(
  entries: DownloadEntry[],
  id: number,
  error: string,
  cancelled: boolean,
): DownloadEntry[] {
  return update(entries, id, { state: cancelled ? "cancelled" : "failed", error });
}

/** Running entries first, then the rest, each group newest first. */
export function orderEntries(entries: DownloadEntry[]): DownloadEntry[] {
  const rank = (e: DownloadEntry) => (e.state === "running" ? 0 : 1);
  return [...entries].sort((a, b) => rank(a) - rank(b) || b.id - a.id);
}

export function runningCount(entries: DownloadEntry[]): number {
  return entries.filter((e) => e.state === "running").length;
}

/** The entries that are still running; drops finished, failed and cancelled ones. */
export function clearFinished(entries: DownloadEntry[]): DownloadEntry[] {
  return entries.filter((e) => e.state === "running");
}

/** A file's name from its path. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}
