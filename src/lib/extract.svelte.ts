import { open } from "@tauri-apps/plugin-dialog";

import { api, errorMessage } from "./api";
import { ARCHIVE_EXTENSIONS } from "./archives";
import { settings, updateSettings } from "./settings.svelte";
import { setStatus } from "./status.svelte";

export const extraction = $state({ running: false });

/** Asks for the folder archives are extracted into and saves it in Settings. */
export async function chooseExtractDir(): Promise<boolean> {
  const dir = await open({
    directory: true,
    title: "Extract downloads to",
    defaultPath: settings.value.extract_dir ?? undefined,
  });
  if (typeof dir !== "string") return false;
  await updateSettings({ extract_dir: dir });
  return true;
}

/**
 * Extracts a downloaded archive, `path` or else one the user picks, into the folder set in
 * Settings, asking for that folder first when none is set. Returns a message saying how it went, or null when the
 * user cancelled.
 */
export async function extractDownload(path?: string): Promise<string | null> {
  if (!settings.value.extract_dir && !(await chooseExtractDir())) return null;
  const file =
    path ??
    (await open({
      title: "Extract a downloaded archive",
      filters: [{ name: "Archives", extensions: ARCHIVE_EXTENSIONS }],
    }));
  if (typeof file !== "string") return null;
  extraction.running = true;
  let message: string;
  try {
    const out = await api.extractArchive(file);
    message = `Extracted to ${out}`;
  } catch (e) {
    message = `Couldn't extract: ${errorMessage(e)}`;
  } finally {
    extraction.running = false;
  }
  setStatus(message);
  return message;
}
