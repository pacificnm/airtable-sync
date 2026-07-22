import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";

/** Result of a dispatched `airtable-sync` subcommand. */
export type RunResult = {
  success: boolean;
  stdout: string;
  error: string | null;
};

/** A subcommand exposed by the core command tree. */
export type SubcommandInfo = {
  name: string;
  about: string;
};

/** A top-level command group and its subcommands. */
export type GroupInfo = {
  name: string;
  about: string;
  subcommands: SubcommandInfo[];
};

const PLUGIN = "plugin:airtable-sync";

/**
 * Runs an `airtable-sync` subcommand in the Rust host.
 *
 * @param args subcommand path + flags, e.g. `["sync", "dry-run"]`
 * @param json request machine-readable output (default true)
 */
export async function runCommand(
  args: string[],
  json = true,
): Promise<RunResult> {
  if (!isTauri()) {
    return {
      success: false,
      stdout: "",
      error: "Not running inside the desktop host (Tauri).",
    };
  }
  return invoke<RunResult>(`${PLUGIN}|airtable_sync_run`, { args, json });
}

/** Fetches the full command tree so the ribbon can be built dynamically. */
export async function fetchCommandGroups(): Promise<GroupInfo[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<GroupInfo[]>(`${PLUGIN}|airtable_sync_command_groups`);
}
