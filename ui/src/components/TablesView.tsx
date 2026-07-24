import { useCallback, useEffect, useState } from "react";
import { Chip } from "../nest-components";
import { listTables, type AirtableTableSummary, type ListTablesOutcome } from "../lib/tables";
import { Icon } from "../shell";
import { faRotate } from "../lib/fontawesome";
import { StatusBanner } from "./ConfigIssues";

/** Structured, read-only view for `airtable list-tables`: cached Airtable tables at a glance. */
export function TablesView() {
  const [outcome, setOutcome] = useState<ListTablesOutcome | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(() => {
    setLoading(true);
    void listTables()
      .then(setOutcome)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Airtable Tables</h1>
        <button
          type="button"
          onClick={load}
          disabled={loading}
          className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border px-2 py-1 text-xs text-nest-muted hover:bg-nest-muted/10 disabled:opacity-40"
        >
          <Icon icon={faRotate} className={["size-3", loading ? "animate-spin" : ""].join(" ")} />
          Reload
        </button>
      </div>

      {loading ? (
        <p className="text-sm text-nest-muted">Loading…</p>
      ) : outcome?.kind === "error" ? (
        <StatusBanner tone="error" title="Could not run airtable list-tables" detail={outcome.message} />
      ) : outcome?.kind === "result" ? (
        <ResultView tables={outcome.result.tables} baseId={outcome.result.base_id} />
      ) : null}
    </div>
  );
}

function ResultView({ tables, baseId }: { tables: AirtableTableSummary[]; baseId: string }) {
  if (tables.length === 0) {
    return (
      <StatusBanner
        tone="info"
        title="No tables in cache"
        detail="Run Setup → Init all (or Airtable → Pull schema) first."
      />
    );
  }

  const enabledCount = tables.filter((table) => table.enabled).length;
  const detail = `${tables.length} table${tables.length === 1 ? "" : "s"} in base ${baseId} · ${enabledCount} sync enabled`;

  return (
    <div className="flex flex-col gap-4">
      <StatusBanner tone="success" title="Tables loaded from cache" detail={detail} />

      <div className="overflow-x-auto rounded-nest-md border border-nest-border">
        <table className="w-full border-collapse text-left text-xs">
          <thead className="bg-nest-surface text-nest-foreground">
            <tr>
              <th className="px-3 py-1.5 font-semibold">Name</th>
              <th className="px-3 py-1.5 font-semibold">Table ID</th>
              <th className="px-3 py-1.5 font-semibold">Sync</th>
              <th className="px-3 py-1.5 font-semibold">Create</th>
              <th className="px-3 py-1.5 font-semibold">Update</th>
              <th className="px-3 py-1.5 font-semibold">Fields</th>
            </tr>
          </thead>
          <tbody>
            {tables.map((table) => (
              <tr key={table.table_id} className="border-t border-nest-border/60">
                <td className="px-3 py-1.5 font-mono text-nest-foreground">{table.name}</td>
                <td className="px-3 py-1.5 font-mono text-nest-foreground">{table.table_id}</td>
                <td className="px-3 py-1.5">
                  <Chip
                    label={table.enabled ? "yes" : "no"}
                    color={table.enabled ? "success" : "error"}
                    size="small"
                  />
                </td>
                <td className="px-3 py-1.5 text-nest-foreground">{table.allow_create ? "yes" : "no"}</td>
                <td className="px-3 py-1.5 text-nest-foreground">{table.allow_update ? "yes" : "no"}</td>
                <td className="px-3 py-1.5 text-nest-foreground">{table.field_count}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
