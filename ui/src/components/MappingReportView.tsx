import { useCallback, useEffect, useState } from "react";
import { Chip } from "../nest-components";
import { mappingReport, type MappingReportOutcome, type MappingReportResult, type MappingReportTable } from "../lib/mapping";
import { Icon } from "../shell";
import { faCircleInfo, faRotate, faTriangleExclamation } from "../lib/fontawesome";
import { StatusBanner } from "./ConfigIssues";

/** Structured view for `mapping report`: colored status banner + per-table unmapped/sync-disabled breakdown. */
export function MappingReportView() {
  const [outcome, setOutcome] = useState<MappingReportOutcome | null>(null);
  const [loading, setLoading] = useState(true);

  const run = useCallback(() => {
    setLoading(true);
    void mappingReport()
      .then(setOutcome)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    run();
  }, [run]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Mapping Report</h1>
        <button
          type="button"
          onClick={run}
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
        <StatusBanner tone="error" title="Could not run mapping report" detail={outcome.message} />
      ) : outcome?.kind === "result" ? (
        <ResultView result={outcome.result} />
      ) : null}
    </div>
  );
}

function ResultView({ result }: { result: MappingReportResult }) {
  if (result.tables.length === 0) {
    return (
      <StatusBanner
        tone="info"
        title="No tables in cache"
        detail="Run Setup → Init all (or Airtable → Pull schema) first."
      />
    );
  }

  const { summary } = result;
  const detail = `${result.tables.length} table${result.tables.length === 1 ? "" : "s"} · ${summary.mapped} of ${summary.fields_total} field(s) mapped · ${summary.sync_enabled} sync enabled`;

  return (
    <div className="flex flex-col gap-4">
      {summary.unmapped > 0 ? (
        <StatusBanner tone="warning" title={`${summary.unmapped} field(s) unmapped`} detail={detail} />
      ) : (
        <StatusBanner tone="success" title="Every field is mapped" detail={detail} />
      )}

      {result.tables.map((table) => (
        <TableSection key={table.table_id} table={table} />
      ))}
    </div>
  );
}

function TableSection({ table }: { table: MappingReportTable }) {
  const { summary } = table;

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center gap-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-nest-muted">
          {table.name} <span className="normal-case text-nest-muted/70">({table.table_id})</span>
        </h2>
        <Chip label={table.enabled ? "sync" : "not synced"} color={table.enabled ? "success" : "default"} size="small" />
      </div>
      <p className="text-[11px] text-nest-muted">
        {summary.mapped} of {summary.fields_total} mapped · {summary.sync_enabled} sync enabled · {summary.mapped_sync_disabled} mapped but sync disabled
      </p>

      {table.unmapped_fields.length === 0 && table.mapped_sync_disabled.length === 0 ? (
        <p className="rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2 text-xs italic text-nest-muted">
          Every field is mapped and synced.
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {table.unmapped_fields.map((fieldName) => (
            <li
              key={`unmapped-${fieldName}`}
              className="flex items-start gap-2.5 rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2"
            >
              <Icon icon={faTriangleExclamation} className="mt-0.5 size-3.5 shrink-0 text-nest-warning" />
              <p className="text-xs text-nest-foreground">
                <span className="font-mono">{fieldName}</span>
                <span className="text-nest-muted"> — unmapped</span>
              </p>
            </li>
          ))}
          {table.mapped_sync_disabled.map((field) => (
            <li
              key={`sync-disabled-${field.field_name}`}
              className="flex items-start gap-2.5 rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2"
            >
              <Icon icon={faCircleInfo} className="mt-0.5 size-3.5 shrink-0 text-nest-info" />
              <p className="text-xs text-nest-foreground">
                <span className="font-mono">{field.field_name}</span>
                {" → "}
                <span className="font-mono">{field.csv_field}</span>
                <span className="text-nest-muted"> ({field.csv_file ?? "—"}) — mapped, sync disabled</span>
              </p>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
