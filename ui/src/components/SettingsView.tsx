import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Chip } from "../nest-components";
import { configSet, configShow, type ConfigShowOutcome, type ConfigShowView } from "../lib/config";
import { Icon, useToast } from "../shell";
import { faFloppyDisk, faRotate } from "../lib/fontawesome";
import { StatusBanner } from "./ConfigIssues";

type TableForm = {
  table_id: string;
  sync: boolean;
  primary_key_field: string;
};

type ConfigForm = {
  api_url: string;
  token_env: string;
  base_id: string;
  tables: Record<string, TableForm>;
  location_data_file: string;
  space_data_file: string;
  db_provider: string;
  db_database_path: string;
  db_schema: string;
  log_level: string;
  log_directory: string;
};

function toForm(config: ConfigShowView): ConfigForm {
  const tables: Record<string, TableForm> = {};
  for (const [name, table] of Object.entries(config.airtable.tables)) {
    tables[name] = {
      table_id: table.table_id,
      sync: table.sync,
      primary_key_field: table.primary_key_field ?? "",
    };
  }
  return {
    api_url: config.airtable.api_url,
    token_env: config.airtable.token_env ?? "",
    base_id: config.airtable.base_id,
    tables,
    location_data_file: config.csv.location_data_file,
    space_data_file: config.csv.space_data_file,
    db_provider: config.database.provider,
    db_database_path: config.database.database_path,
    db_schema: config.database.schema,
    log_level: config.logging.level,
    log_directory: config.logging.directory,
  };
}

/** `[dotted.config.key, new value]` pairs for every field that changed between `a` and `b`. */
function diffChanges(a: ConfigForm, b: ConfigForm): Array<[string, string]> {
  const changes: Array<[string, string]> = [];
  const push = (key: string, before: string, after: string) => {
    if (before !== after) {
      changes.push([key, after]);
    }
  };

  push("airtable.api_url", a.api_url, b.api_url);
  push("airtable.token_env", a.token_env, b.token_env);
  push("airtable.base_id", a.base_id, b.base_id);

  for (const name of Object.keys(b.tables)) {
    const before = a.tables[name];
    const after = b.tables[name];
    if (!before) {
      continue;
    }
    push(`airtable.tables.${name}.table_id`, before.table_id, after.table_id);
    if (before.sync !== after.sync) {
      changes.push([`airtable.tables.${name}.sync`, String(after.sync)]);
    }
    push(`airtable.tables.${name}.primary_key_field`, before.primary_key_field, after.primary_key_field);
  }

  push("csv.location_data_file", a.location_data_file, b.location_data_file);
  push("csv.space_data_file", a.space_data_file, b.space_data_file);
  push("database.provider", a.db_provider, b.db_provider);
  push("database.database_path", a.db_database_path, b.db_database_path);
  push("database.schema", a.db_schema, b.db_schema);
  push("logging.level", a.log_level, b.log_level);
  push("logging.directory", a.log_directory, b.log_directory);

  return changes;
}

/** Editable settings form for `config.toml`, backed by `config set`. */
export function SettingsView() {
  const [outcome, setOutcome] = useState<ConfigShowOutcome | null>(null);
  const [original, setOriginal] = useState<ConfigForm | null>(null);
  const [form, setForm] = useState<ConfigForm | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const toast = useToast();

  const load = useCallback(() => {
    setLoading(true);
    void configShow()
      .then((result) => {
        setOutcome(result);
        if (result.kind === "result") {
          const next = toForm(result.result.config);
          setOriginal(next);
          setForm(next);
        }
      })
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const dirty = original !== null && form !== null && diffChanges(original, form).length > 0;

  const handleSave = useCallback(async () => {
    if (!original || !form) {
      return;
    }
    const changes = diffChanges(original, form);
    if (changes.length === 0) {
      toast.info("Nothing to save — no fields were changed.");
      return;
    }

    setSaving(true);
    const failures: string[] = [];
    for (const [key, value] of changes) {
      const result = await configSet(key, value);
      if (!result.success) {
        failures.push(`${key}: ${result.message}`);
      }
    }
    setSaving(false);

    if (failures.length === 0) {
      toast.success(
        `Saved ${changes.length} change${changes.length === 1 ? "" : "s"} — config.toml updated.`,
      );
    } else {
      toast.error(
        `${failures.length} of ${changes.length} change${changes.length === 1 ? "" : "s"} failed: ${failures.join(" · ")}`,
        8000,
      );
    }

    load();
  }, [original, form, load, toast]);

  const updateForm = useCallback((patch: Partial<ConfigForm>) => {
    setForm((current) => (current ? { ...current, ...patch } : current));
  }, []);

  const updateTable = useCallback((name: string, patch: Partial<TableForm>) => {
    setForm((current) => {
      if (!current) {
        return current;
      }
      const table = current.tables[name];
      if (!table) {
        return current;
      }
      return { ...current, tables: { ...current.tables, [name]: { ...table, ...patch } } };
    });
  }, []);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Settings</h1>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={load}
            disabled={loading || saving}
            className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border px-2 py-1 text-xs text-nest-muted hover:bg-nest-muted/10 disabled:opacity-40"
          >
            <Icon icon={faRotate} className={["size-3", loading ? "animate-spin" : ""].join(" ")} />
            Reload
          </button>
          <button
            type="button"
            onClick={() => void handleSave()}
            disabled={loading || saving || !dirty}
            className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-primary/40 bg-nest-primary/10 px-2 py-1 text-xs font-medium text-nest-primary hover:bg-nest-primary/15 disabled:cursor-not-allowed disabled:opacity-40"
          >
            <Icon icon={faFloppyDisk} className="size-3" />
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>

      {loading ? (
        <p className="text-sm text-nest-muted">Loading…</p>
      ) : outcome?.kind === "error" ? (
        <StatusBanner tone="error" title="Could not load configuration" detail={outcome.message} />
      ) : form ? (
        <Form form={form} updateForm={updateForm} updateTable={updateTable} />
      ) : null}
    </div>
  );
}

function Form({
  form,
  updateForm,
  updateTable,
}: {
  form: ConfigForm;
  updateForm: (patch: Partial<ConfigForm>) => void;
  updateTable: (name: string, patch: Partial<TableForm>) => void;
}) {
  const tables = Object.entries(form.tables);

  return (
    <div className="flex flex-col gap-4">
      <Section title="Airtable">
        <TextField label="api_url" value={form.api_url} onChange={(value) => updateForm({ api_url: value })} />
        <TextField
          label="token_env"
          value={form.token_env}
          onChange={(value) => updateForm({ token_env: value })}
          placeholder="(unset — leave blank)"
        />
        <TextField label="base_id" value={form.base_id} onChange={(value) => updateForm({ base_id: value })} />
      </Section>

      <Section title="Tables">
        {tables.length === 0 ? (
          <p className="px-3 py-2 text-xs italic text-nest-muted">No tables configured.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-left text-xs">
              <thead className="bg-nest-surface text-nest-foreground">
                <tr>
                  <th className="px-3 py-1.5 font-semibold">Name</th>
                  <th className="px-3 py-1.5 font-semibold">Table ID</th>
                  <th className="px-3 py-1.5 font-semibold">Primary key field</th>
                  <th className="px-3 py-1.5 font-semibold">Sync</th>
                </tr>
              </thead>
              <tbody>
                {tables.map(([name, table]) => (
                  <tr key={name} className="border-t border-nest-border/60">
                    <td className="px-3 py-1.5 font-mono text-nest-foreground">{name}</td>
                    <td className="px-3 py-1.5">
                      <CellInput
                        value={table.table_id}
                        onChange={(value) => updateTable(name, { table_id: value })}
                      />
                    </td>
                    <td className="px-3 py-1.5">
                      <CellInput
                        value={table.primary_key_field}
                        placeholder="(unset)"
                        onChange={(value) => updateTable(name, { primary_key_field: value })}
                      />
                    </td>
                    <td className="px-3 py-1.5">
                      <Chip
                        label={table.sync ? "yes" : "no"}
                        color={table.sync ? "success" : "error"}
                        size="small"
                        clickable
                        onClick={() => updateTable(name, { sync: !table.sync })}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Section>

      <Section title="CSV">
        <TextField
          label="location_data_file"
          value={form.location_data_file}
          onChange={(value) => updateForm({ location_data_file: value })}
        />
        <TextField
          label="space_data_file"
          value={form.space_data_file}
          onChange={(value) => updateForm({ space_data_file: value })}
        />
      </Section>

      <Section title="Database">
        <TextField label="provider" value={form.db_provider} onChange={(value) => updateForm({ db_provider: value })} />
        <TextField
          label="database_path"
          value={form.db_database_path}
          onChange={(value) => updateForm({ db_database_path: value })}
        />
        <TextField label="schema" value={form.db_schema} onChange={(value) => updateForm({ db_schema: value })} />
      </Section>

      <Section title="Logging">
        <TextField label="level" value={form.log_level} onChange={(value) => updateForm({ log_level: value })} />
        <TextField
          label="directory"
          value={form.log_directory}
          onChange={(value) => updateForm({ log_directory: value })}
        />
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-nest-muted">{title}</h2>
      <div className="overflow-hidden rounded-nest-md border border-nest-border bg-nest-background">
        {children}
      </div>
    </div>
  );
}

function TextField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <div className="flex items-center gap-3 border-b border-nest-border/60 px-3 py-1.5 last:border-b-0">
      <span className="w-40 shrink-0 font-mono text-[11px] text-nest-muted">{label}</span>
      <input
        type="text"
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        className="min-w-0 flex-1 rounded-nest-sm border border-transparent bg-transparent px-1.5 py-0.5 font-mono text-[11px] text-nest-foreground hover:border-nest-border focus:border-nest-primary focus:outline-none"
      />
    </div>
  );
}

function CellInput({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <input
      type="text"
      value={value}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
      className="w-full min-w-0 rounded-nest-sm border border-transparent bg-transparent px-1.5 py-0.5 font-mono text-[11px] text-nest-foreground hover:border-nest-border focus:border-nest-primary focus:outline-none"
    />
  );
}
