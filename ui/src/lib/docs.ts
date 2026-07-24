import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";

/** One entry in the Help sidebar's table of contents. */
export type DocEntry = {
  path: string;
  name: string;
  depth: number;
};

const PLUGIN = "plugin:airtable-sync";

/** Lists the desktop app's bundled documentation (`docs/**\/*.md`). */
export async function docsList(): Promise<DocEntry[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<DocEntry[]>(`${PLUGIN}|airtable_sync_docs_list`);
}

/** Reads one documentation file's raw Markdown, by path relative to the docs root. */
export async function docsRead(path: string): Promise<string> {
  if (!isTauri()) {
    return "Help is available in the desktop app.";
  }
  return invoke<string>(`${PLUGIN}|airtable_sync_docs_read`, { path });
}
