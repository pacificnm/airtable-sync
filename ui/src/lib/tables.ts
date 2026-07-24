import { runCommand } from "./airtable";
import { extractJson } from "./json";

/** One cached Airtable table in a `list-tables` listing. */
export type AirtableTableSummary = {
  name: string;
  table_id: string;
  enabled: boolean;
  allow_create: boolean;
  allow_update: boolean;
  field_count: number;
};

/** Structured `airtable list-tables --json` payload. */
export type ListTablesResult = {
  database_path: string;
  base_id: string;
  tables: AirtableTableSummary[];
};

export type ListTablesOutcome =
  | { kind: "result"; result: ListTablesResult }
  | { kind: "error"; message: string };

/** Runs `airtable list-tables --json` and parses the structured result. */
export async function listTables(): Promise<ListTablesOutcome> {
  const run = await runCommand(["airtable", "list-tables"], true);
  const parsed = extractJson<ListTablesResult>(run.stdout);
  if (parsed && Array.isArray(parsed.tables)) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "airtable list-tables produced no output",
  };
}
