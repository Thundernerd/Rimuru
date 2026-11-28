import type { DownloadProgress } from "./types";
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
