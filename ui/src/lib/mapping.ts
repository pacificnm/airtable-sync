import { runCommand } from "./airtable";
import { extractJson } from "./json";

/** One field newly mapped by an auto-map run. */
export type AutoMapField = {
  field_name: string;
  csv_field: string;
  csv_file: string;
};

/** Auto-mapping outcome for one configured table. */
export type AutoMapTable = {
  name: string;
  table_id: string;
  mapped: AutoMapField[];
  already_mapped: number;
  unresolved: string[];
};

/** Structured `mapping auto --json` payload. */
export type AutoMapResult = {
  database_path: string;
  base_id: string;
  tables: AutoMapTable[];
  mapped_total: number;
  unresolved_total: number;
};

export type AutoMapOutcome =
  | { kind: "result"; result: AutoMapResult }
  | { kind: "error"; message: string };

/** Runs `mapping auto --json` and parses the structured result. */
export async function mappingAuto(): Promise<AutoMapOutcome> {
  const run = await runCommand(["mapping", "auto"], true);
  const parsed = extractJson<AutoMapResult>(run.stdout);
  if (parsed && Array.isArray(parsed.tables)) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "mapping auto produced no output",
  };
}

/** One field in a `mapping list` table listing. */
export type MappingListField = {
  field_name: string;
  field_id: string | null;
  field_type: string | null;
  is_key: boolean;
  csv_field: string | null;
  csv_file: string | null;
  sync_enabled: boolean;
};

/** Cached table metadata in a `mapping list` listing. */
export type MappingListTableInfo = {
  name: string;
  table_id: string;
  enabled: boolean;
};

/** Mapping summary counts for one table. */
export type MappingListSummary = {
  fields_total: number;
  mapped: number;
  sync_enabled: number;
};

/** One table's mapping state within an all-tables `mapping list` listing. */
export type MappingListTable = {
  table: MappingListTableInfo;
  summary: MappingListSummary;
  fields: MappingListField[];
};

/** Structured `mapping list --json` payload (no table given — every cached table). */
export type MappingListAllResult = {
  database_path: string;
  base_id: string;
  tables: MappingListTable[];
};

export type MappingListOutcome =
  | { kind: "result"; result: MappingListAllResult }
  | { kind: "error"; message: string };

/** Runs `mapping list --json` (every cached table) and parses the structured result. */
export async function mappingList(): Promise<MappingListOutcome> {
  const run = await runCommand(["mapping", "list"], true);
  const parsed = extractJson<MappingListAllResult>(run.stdout);
  if (parsed && Array.isArray(parsed.tables)) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "mapping list produced no output",
  };
}

/** Mapping counts for one table or the full report. */
export type MappingReportSummary = {
  fields_total: number;
  mapped: number;
  unmapped: number;
  sync_enabled: number;
  mapped_sync_disabled: number;
};

/** One mapped field with sync disabled. */
export type MappedSyncDisabledField = {
  field_name: string;
  csv_field: string;
  csv_file: string | null;
};

/** Mapping report for one cached table. */
export type MappingReportTable = {
  name: string;
  table_id: string;
  enabled: boolean;
  summary: MappingReportSummary;
  unmapped_fields: string[];
  mapped_sync_disabled: MappedSyncDisabledField[];
};

/** Structured `mapping report --json` payload. */
export type MappingReportResult = {
  database_path: string;
  base_id: string;
  summary: MappingReportSummary;
  tables: MappingReportTable[];
};

export type MappingReportOutcome =
  | { kind: "result"; result: MappingReportResult }
  | { kind: "error"; message: string };

/** Runs `mapping report --json` and parses the structured result. */
export async function mappingReport(): Promise<MappingReportOutcome> {
  const run = await runCommand(["mapping", "report"], true);
  const parsed = extractJson<MappingReportResult>(run.stdout);
  if (parsed && Array.isArray(parsed.tables)) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "mapping report produced no output",
  };
}

/** One cached CSV column. */
export type CsvHeader = {
  name: string;
  normalized_name: string;
};

/** Cached headers for one configured CSV file. */
export type CsvFileHeaders = {
  role: "location" | "space" | "unknown";
  filename: string;
  columns: CsvHeader[];
};

/** Structured `csv list-headers --json` payload. */
export type CsvListHeadersResult = {
  database_path: string;
  files: CsvFileHeaders[];
};

export type CsvListHeadersOutcome =
  | { kind: "result"; result: CsvListHeadersResult }
  | { kind: "error"; message: string };

/** Runs `csv list-headers --json` and parses the structured result. */
export async function csvListHeaders(): Promise<CsvListHeadersOutcome> {
  const run = await runCommand(["csv", "list-headers"], true);
  const parsed = extractJson<CsvListHeadersResult>(run.stdout);
  if (parsed && Array.isArray(parsed.files)) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "csv list-headers produced no output",
  };
}

export type MappingSetOutcome = { success: true } | { success: false; message: string };

/**
 * Maps one Airtable field to a CSV column via `mapping set <table> <field>
 * <csv_column> --csv-file <role>`. Leaves `sync_enabled` unchanged (no
 * `--enable`/`--disable` is passed).
 */
export async function mappingSet(
  table: string,
  field: string,
  csvColumn: string,
  csvFileRole: "location" | "space",
): Promise<MappingSetOutcome> {
  const run = await runCommand(
    ["mapping", "set", table, field, csvColumn, "--csv-file", csvFileRole],
    true,
  );
  if (run.success) {
    return { success: true };
  }
  return {
    success: false,
    message: run.error?.trim() || run.stdout.trim() || `mapping set ${table} ${field} failed`,
  };
}

/**
 * Toggles sync for one already-mapped field via `mapping enable`/`mapping
 * disable <table> <field>`. Fails if the field has no CSV mapping yet.
 */
export async function mappingSetSyncEnabled(
  table: string,
  field: string,
  enabled: boolean,
): Promise<MappingSetOutcome> {
  const run = await runCommand(["mapping", enabled ? "enable" : "disable", table, field], true);
  if (run.success) {
    return { success: true };
  }
  return {
    success: false,
    message: run.error?.trim() || run.stdout.trim() || `mapping ${enabled ? "enable" : "disable"} ${table} ${field} failed`,
  };
}
