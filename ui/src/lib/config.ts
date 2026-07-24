import { runCommand } from "./airtable";
import { extractJson } from "./json";

/** One structured issue from `config validate --json`. */
export type ConfigIssue = {
  field: string | null;
  message: string;
  help: string | null;
  severity: "error" | "warning";
};

/** Structured `config validate --json` payload. */
export type ConfigValidateResult = {
  valid: boolean;
  table_count: number;
  sync_count: number;
  errors: ConfigIssue[];
  warnings: ConfigIssue[];
};

export type ConfigValidateOutcome =
  | { kind: "result"; result: ConfigValidateResult }
  | { kind: "error"; message: string };

/** Runs `config validate --json` and parses the structured result. */
export async function configValidate(): Promise<ConfigValidateOutcome> {
  const run = await runCommand(["config", "validate"], true);
  const parsed = extractJson<ConfigValidateResult>(run.stdout);
  if (parsed) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "config validate produced no output",
  };
}

/** One configured `[airtable.tables.<name>]` entry, redacted for display. */
export type ConfigShowTable = {
  table_id: string;
  sync: boolean;
  primary_key_field: string | null;
};

/** Redacted `config show --json` payload. */
export type ConfigShowView = {
  config_path: string | null;
  airtable: {
    api_url: string;
    token: string;
    token_env: string | null;
    base_id: string;
    tables: Record<string, ConfigShowTable>;
  };
  sync: {
    dry_run: boolean;
    continue_on_error: boolean;
    max_parallel_tables: number;
    max_parallel_updates: number;
    create_change_plan: boolean;
  };
  csv: {
    location_data_file: string;
    space_data_file: string;
  };
  database: {
    provider: string;
    database_path: string;
    schema: string;
  };
  logging: {
    level: string;
    directory: string;
  };
};

/** Structured `config show --json` payload. */
export type ConfigShowResult = {
  valid: boolean;
  config: ConfigShowView;
  errors: ConfigIssue[];
  warnings: ConfigIssue[];
};

export type ConfigShowOutcome =
  | { kind: "result"; result: ConfigShowResult }
  | { kind: "error"; message: string };

/** Runs `config show --json` and parses the structured result. */
export async function configShow(): Promise<ConfigShowOutcome> {
  const run = await runCommand(["config", "show"], true);
  const parsed = extractJson<ConfigShowResult>(run.stdout);
  if (parsed) {
    return { kind: "result", result: parsed };
  }
  return {
    kind: "error",
    message: run.error?.trim() || run.stdout.trim() || "config show produced no output",
  };
}

export type ConfigSetOutcome = { success: true } | { success: false; message: string };

/**
 * Sets one allow-listed `config.toml` key (e.g. `airtable.base_id` or
 * `airtable.tables.<name>.sync`), preserving the rest of the file's
 * formatting. An empty `value` unsets an optional field.
 */
export async function configSet(key: string, value: string): Promise<ConfigSetOutcome> {
  const run = await runCommand(["config", "set", key, value], true);
  if (run.success) {
    return { success: true };
  }
  return {
    success: false,
    message: run.error?.trim() || run.stdout.trim() || `config set ${key} failed`,
  };
}
