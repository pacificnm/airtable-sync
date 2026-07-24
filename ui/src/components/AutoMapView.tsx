import { useCallback, useEffect, useState } from "react";
import { mappingAuto, type AutoMapOutcome, type AutoMapResult, type AutoMapTable } from "../lib/mapping";
import { Icon } from "../shell";
import { faCircleCheck, faDiagramProject, faTriangleExclamation } from "../lib/fontawesome";
import { StatusBanner } from "./ConfigIssues";

/** Structured view for `mapping auto`: colored status banner + per-table mapped/unresolved breakdown. */
export function AutoMapView() {
  const [outcome, setOutcome] = useState<AutoMapOutcome | null>(null);
  const [loading, setLoading] = useState(true);

  const run = useCallback(() => {
    setLoading(true);
    void mappingAuto()
      .then(setOutcome)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    run();
  }, [run]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Auto-Map Fields</h1>
        <button
          type="button"
          onClick={run}
          disabled={loading}
          className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border px-2 py-1 text-xs text-nest-muted hover:bg-nest-muted/10 disabled:opacity-40"
        >
          <Icon icon={faDiagramProject} className={["size-3", loading ? "animate-spin" : ""].join(" ")} />
          {loading ? "Mapping…" : "Run again"}
        </button>
      </div>

      {loading ? (
        <p className="text-sm text-nest-muted">Mapping…</p>
      ) : outcome?.kind === "error" ? (
        <StatusBanner tone="error" title="Could not run mapping auto" detail={outcome.message} />
      ) : outcome?.kind === "result" ? (
        <ResultView result={outcome.result} />
      ) : null}
    </div>
  );
}

function ResultView({ result }: { result: AutoMapResult }) {
  const detail = `${result.mapped_total} field${result.mapped_total === 1 ? "" : "s"} mapped, ${result.unresolved_total} unresolved across ${result.tables.length} table${result.tables.length === 1 ? "" : "s"}`;

  if (result.tables.length === 0) {
    return (
      <StatusBanner
        tone="info"
        title="No sync-enabled tables with a cached schema"
        detail="Run Setup → Init all (or Airtable → Pull schema) first."
      />
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {result.unresolved_total > 0 ? (
        <StatusBanner tone="warning" title="Mapped, with unresolved fields" detail={detail} />
      ) : result.mapped_total > 0 ? (
        <StatusBanner tone="success" title="Auto-map complete" detail={detail} />
      ) : (
        <StatusBanner tone="info" title="Nothing new to map" detail={detail} />
      )}

      {result.tables.map((table) => (
        <TableSection key={table.table_id} table={table} />
      ))}
    </div>
  );
}

function TableSection({ table }: { table: AutoMapTable }) {
  return (
    <div className="flex flex-col gap-1.5">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-nest-muted">
        {table.name} <span className="normal-case text-nest-muted/70">({table.table_id})</span>
      </h2>
      <p className="text-[11px] text-nest-muted">
        {table.mapped.length} mapped now · {table.already_mapped} already mapped · {table.unresolved.length} unresolved
      </p>

      {table.mapped.length === 0 && table.unresolved.length === 0 ? (
        <p className="rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2 text-xs italic text-nest-muted">
          Every field is already mapped.
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {table.mapped.map((field) => (
            <li
              key={field.field_name}
              className="flex items-start gap-2.5 rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2"
            >
              <Icon icon={faCircleCheck} className="mt-0.5 size-3.5 shrink-0 text-nest-success" />
              <p className="text-xs text-nest-foreground">
                <span className="font-mono">{field.field_name}</span>
                {" → "}
                <span className="font-mono">{field.csv_field}</span>
                <span className="text-nest-muted"> ({field.csv_file})</span>
              </p>
            </li>
          ))}
          {table.unresolved.map((fieldName) => (
            <li
              key={fieldName}
              className="flex items-start gap-2.5 rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2"
            >
              <Icon icon={faTriangleExclamation} className="mt-0.5 size-3.5 shrink-0 text-nest-warning" />
              <p className="text-xs text-nest-foreground">
                <span className="font-mono">{fieldName}</span>
                <span className="text-nest-muted"> — no matching CSV column</span>
              </p>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
