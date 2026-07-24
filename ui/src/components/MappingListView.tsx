import { useCallback, useEffect, useMemo, useState } from "react";
import { Chip } from "../nest-components";
import {
  csvListHeaders,
  mappingList,
  mappingSet,
  mappingSetSyncEnabled,
  type CsvFileHeaders,
  type MappingListField,
  type MappingListOutcome,
  type MappingListTable,
} from "../lib/mapping";
import { Icon, useToast } from "../shell";
import { faRotate } from "../lib/fontawesome";
import { StatusBanner } from "./ConfigIssues";

/** Structured, editable view for `mapping list`: CSV file/column dropdowns save via `mapping set`. */
export function MappingListView() {
  const [outcome, setOutcome] = useState<MappingListOutcome | null>(null);
  const [headerFiles, setHeaderFiles] = useState<CsvFileHeaders[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [pendingFile, setPendingFile] = useState<Record<string, string>>({});
  const toast = useToast();

  const load = useCallback(() => {
    setLoading(true);
    setPendingFile({});
    void (async () => {
      // Sequential, not Promise.all: two concurrent `airtable_sync_run`
      // invocations race on the Rust dispatcher's stdout capture (a raw fd
      // swap that isn't safe across threads), which can hand back one
      // command's stdout mixed with the other's — malformed JSON for both.
      const listOutcome = await mappingList();
      const headersOutcome = await csvListHeaders();
      setOutcome(listOutcome);
      setHeaderFiles(headersOutcome.kind === "result" ? headersOutcome.result.files : []);
      setLoading(false);
    })();
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const roleForFilename = useCallback(
    (filename: string): "location" | "space" | null => {
      const file = headerFiles.find((candidate) => candidate.filename === filename);
      return file && file.role !== "unknown" ? file.role : null;
    },
    [headerFiles],
  );

  const handleFileChange = useCallback(
    async (table: string, field: MappingListField, newFilename: string) => {
      const key = `${table}:${field.field_name}`;
      const targetFile = headerFiles.find((candidate) => candidate.filename === newFilename);
      const role = targetFile && targetFile.role !== "unknown" ? targetFile.role : null;

      const keptColumn =
        field.csv_field && targetFile?.columns.some((column) => column.normalized_name === field.csv_field)
          ? field.csv_field
          : null;

      if (keptColumn && role) {
        setSaving(true);
        const result = await mappingSet(table, field.field_name, keptColumn, role);
        setSaving(false);
        if (result.success) {
          toast.success(`${table}.${field.field_name} → ${newFilename}`);
          load();
        } else {
          toast.error(result.message, 8000);
        }
        return;
      }

      // No matching column in the new file yet — just switch which file the
      // CSV column dropdown is scoped to; nothing to save until a column is chosen.
      setPendingFile((current) => ({ ...current, [key]: newFilename }));
    },
    [headerFiles, load, toast],
  );

  const handleColumnChange = useCallback(
    async (table: string, field: MappingListField, effectiveFile: string, newColumn: string) => {
      const role = roleForFilename(effectiveFile);
      if (!role) {
        toast.error(`"${effectiveFile}" is not a configured location/space CSV file`);
        return;
      }
      setSaving(true);
      const result = await mappingSet(table, field.field_name, newColumn, role);
      setSaving(false);
      if (result.success) {
        toast.success(`${table}.${field.field_name} → ${newColumn} (${effectiveFile})`);
        load();
      } else {
        toast.error(result.message, 8000);
      }
    },
    [roleForFilename, load, toast],
  );

  const handleSyncToggle = useCallback(
    async (table: string, field: MappingListField) => {
      if (!field.csv_field) {
        toast.error(`${table}.${field.field_name} has no CSV mapping yet — pick a CSV column first.`);
        return;
      }
      const nextEnabled = !field.sync_enabled;
      setSaving(true);
      const result = await mappingSetSyncEnabled(table, field.field_name, nextEnabled);
      setSaving(false);
      if (result.success) {
        toast.success(`${table}.${field.field_name} sync ${nextEnabled ? "enabled" : "disabled"}`);
        load();
      } else {
        toast.error(result.message, 8000);
      }
    },
    [load, toast],
  );

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Field Mappings</h1>
        <button
          type="button"
          onClick={load}
          disabled={loading || saving}
          className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border px-2 py-1 text-xs text-nest-muted hover:bg-nest-muted/10 disabled:opacity-40"
        >
          <Icon icon={faRotate} className={["size-3", loading ? "animate-spin" : ""].join(" ")} />
          Reload
        </button>
      </div>

      {loading ? (
        <p className="text-sm text-nest-muted">Loading…</p>
      ) : outcome?.kind === "error" ? (
        <StatusBanner tone="error" title="Could not run mapping list" detail={outcome.message} />
      ) : outcome?.kind === "result" ? (
        <div className="flex flex-col gap-4">
          <Summary tables={outcome.result.tables} />
          {headerFiles.length === 0 ? (
            <StatusBanner
              tone="info"
              title="CSV file/column editing unavailable"
              detail="Run CSV → Import headers to enable the dropdowns below."
            />
          ) : null}
          {outcome.result.tables.map((table) => (
            <TableSection
              key={table.table.table_id}
              table={table}
              headerFiles={headerFiles}
              pendingFile={pendingFile}
              disabled={saving}
              onFileChange={handleFileChange}
              onColumnChange={handleColumnChange}
              onSyncToggle={handleSyncToggle}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function Summary({ tables }: { tables: MappingListTable[] }) {
  if (tables.length === 0) {
    return (
      <StatusBanner
        tone="info"
        title="No tables in cache"
        detail="Run Setup → Init all (or Airtable → Pull schema) first."
      />
    );
  }

  const fieldsTotal = tables.reduce((sum, table) => sum + table.summary.fields_total, 0);
  const mappedTotal = tables.reduce((sum, table) => sum + table.summary.mapped, 0);
  const unmappedTotal = fieldsTotal - mappedTotal;
  const detail = `${tables.length} table${tables.length === 1 ? "" : "s"} · ${mappedTotal} of ${fieldsTotal} field(s) mapped`;

  return unmappedTotal > 0 ? (
    <StatusBanner tone="warning" title={`${unmappedTotal} field(s) unmapped`} detail={detail} />
  ) : (
    <StatusBanner tone="success" title="Every field is mapped" detail={detail} />
  );
}

type TableSectionProps = {
  table: MappingListTable;
  headerFiles: CsvFileHeaders[];
  pendingFile: Record<string, string>;
  disabled: boolean;
  onFileChange: (table: string, field: MappingListField, newFilename: string) => void;
  onColumnChange: (table: string, field: MappingListField, effectiveFile: string, newColumn: string) => void;
  onSyncToggle: (table: string, field: MappingListField) => void;
};

function TableSection({
  table,
  headerFiles,
  pendingFile,
  disabled,
  onFileChange,
  onColumnChange,
  onSyncToggle,
}: TableSectionProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center gap-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-nest-muted">
          {table.table.name} <span className="normal-case text-nest-muted/70">({table.table.table_id})</span>
        </h2>
        <Chip label={table.table.enabled ? "sync" : "not synced"} color={table.table.enabled ? "success" : "default"} size="small" />
      </div>
      <p className="text-[11px] text-nest-muted">
        {table.summary.fields_total} mappable field(s) · {table.summary.mapped} mapped · {table.summary.sync_enabled} sync enabled
      </p>

      {table.fields.length === 0 ? (
        <p className="rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2 text-xs italic text-nest-muted">
          No mappable fields in cache.
        </p>
      ) : (
        <div className="overflow-x-auto rounded-nest-md border border-nest-border">
          <table className="w-full border-collapse text-left text-xs">
            <thead className="bg-nest-surface text-nest-foreground">
              <tr>
                <th className="px-3 py-1.5 font-semibold">Field</th>
                <th className="px-3 py-1.5 font-semibold">CSV file</th>
                <th className="px-3 py-1.5 font-semibold">CSV column</th>
                <th className="px-3 py-1.5 font-semibold">Sync</th>
                <th className="px-3 py-1.5 font-semibold">Key</th>
              </tr>
            </thead>
            <tbody>
              {table.fields.map((field) => (
                <FieldRow
                  key={field.field_name}
                  tableName={table.table.name}
                  field={field}
                  headerFiles={headerFiles}
                  pendingFile={pendingFile}
                  disabled={disabled}
                  onFileChange={onFileChange}
                  onColumnChange={onColumnChange}
                  onSyncToggle={onSyncToggle}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

type FieldRowProps = {
  tableName: string;
  field: MappingListField;
  headerFiles: CsvFileHeaders[];
  pendingFile: Record<string, string>;
  disabled: boolean;
  onFileChange: (table: string, field: MappingListField, newFilename: string) => void;
  onColumnChange: (table: string, field: MappingListField, effectiveFile: string, newColumn: string) => void;
  onSyncToggle: (table: string, field: MappingListField) => void;
};

function FieldRow({
  tableName,
  field,
  headerFiles,
  pendingFile,
  disabled,
  onFileChange,
  onColumnChange,
  onSyncToggle,
}: FieldRowProps) {
  const key = `${tableName}:${field.field_name}`;
  const editingFile = pendingFile[key];
  const effectiveFile = editingFile ?? field.csv_file ?? "";
  const columnValue = editingFile ? "" : field.csv_field ?? "";
  const canEdit = headerFiles.length > 0;

  const columnsForFile = useMemo(
    () => headerFiles.find((file) => file.filename === effectiveFile)?.columns ?? [],
    [headerFiles, effectiveFile],
  );

  return (
    <tr className="border-t border-nest-border/60">
      <td className="px-3 py-1.5 font-mono text-nest-foreground">{field.field_name}</td>
      <td className="px-3 py-1.5">
        {canEdit ? (
          <select
            value={effectiveFile}
            disabled={disabled}
            onChange={(event) => onFileChange(tableName, field, event.target.value)}
            className="w-full min-w-0 rounded-nest-sm border border-nest-border bg-nest-background px-1.5 py-1 font-mono text-[11px] text-nest-foreground focus:border-nest-primary focus:outline-none disabled:opacity-50"
          >
            <option value="">— select —</option>
            {headerFiles.map((file) => (
              <option key={file.filename} value={file.filename}>
                {file.filename}
              </option>
            ))}
          </select>
        ) : (
          <span className="font-mono text-nest-foreground">{field.csv_file ?? "—"}</span>
        )}
      </td>
      <td className="px-3 py-1.5">
        {canEdit ? (
          <select
            value={columnValue}
            disabled={disabled || !effectiveFile}
            onChange={(event) => onColumnChange(tableName, field, effectiveFile, event.target.value)}
            className="w-full min-w-0 rounded-nest-sm border border-nest-border bg-nest-background px-1.5 py-1 font-mono text-[11px] text-nest-foreground focus:border-nest-primary focus:outline-none disabled:opacity-50"
          >
            <option value="">— select —</option>
            {columnsForFile.map((column) => (
              <option key={column.normalized_name} value={column.normalized_name}>
                {column.name}
              </option>
            ))}
          </select>
        ) : (
          <span className="font-mono text-nest-foreground">{field.csv_field ?? "—"}</span>
        )}
      </td>
      <td className="px-3 py-1.5">
        <span title={field.csv_field ? "Click to toggle sync" : "Pick a CSV column first"}>
          <Chip
            label={field.sync_enabled ? "yes" : "no"}
            color={field.sync_enabled ? "success" : "error"}
            size="small"
            clickable
            disabled={disabled || !field.csv_field}
            onClick={() => onSyncToggle(tableName, field)}
          />
        </span>
      </td>
      <td className="px-3 py-1.5 text-nest-foreground">{field.is_key ? "yes" : "no"}</td>
    </tr>
  );
}
