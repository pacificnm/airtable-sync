import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Chip } from "../nest-components";
import { configShow, type ConfigShowOutcome, type ConfigShowResult } from "../lib/config";
import { Icon } from "../shell";
import { faRotate } from "../lib/fontawesome";
import { IssueSection, StatusBanner } from "./ConfigIssues";

/** Structured view for `config show`: colored status banner + issues + resolved settings. */
export function ShowView() {
  const [outcome, setOutcome] = useState<ConfigShowOutcome | null>(null);
  const [loading, setLoading] = useState(true);

  const run = useCallback(() => {
    setLoading(true);
    void configShow()
      .then(setOutcome)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    run();
  }, [run]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Configuration</h1>
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
        <StatusBanner tone="error" title="Could not run config show" detail={outcome.message} />
      ) : outcome?.kind === "result" ? (
        <ResultView result={outcome.result} />
      ) : null}
    </div>
  );
}

function ResultView({ result }: { result: ConfigShowResult }) {
  const { config } = result;
  const tables = Object.entries(config.airtable.tables);
  const syncCount = tables.filter(([, table]) => table.sync).length;
  const tableSummary = `${tables.length} table${tables.length === 1 ? "" : "s"} configured, ${syncCount} enabled for sync`;

  return (
    <div className="flex flex-col gap-4">
      {result.valid ? (
        <StatusBanner
          tone={result.warnings.length > 0 ? "warning" : "success"}
          title={result.warnings.length > 0 ? "Configuration valid, with warnings" : "Configuration valid"}
          detail={tableSummary}
        />
      ) : (
        <StatusBanner
          tone="error"
          title={`Configuration invalid — ${result.errors.length} error${result.errors.length === 1 ? "" : "s"}`}
          detail={tableSummary}
        />
      )}

      {result.errors.length > 0 ? <IssueSection title="Errors" tone="error" issues={result.errors} /> : null}
      {result.warnings.length > 0 ? (
        <IssueSection title="Warnings" tone="warning" issues={result.warnings} />
      ) : null}

      <ConfigSection title="Airtable">
        <Row label="config file" value={config.config_path ?? "(in memory)"} />
        <Row label="api_url" value={config.airtable.api_url} />
        <Row label="token" value={config.airtable.token} />
        {config.airtable.token_env ? <Row label="token_env" value={config.airtable.token_env} /> : null}
        <Row label="base_id" value={config.airtable.base_id} />
      </ConfigSection>

      <ConfigSection title="Tables">
        {tables.length === 0 ? (
          <p className="px-3 py-2 text-xs italic text-nest-muted">No tables configured.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-left text-xs">
              <thead className="bg-nest-surface text-nest-foreground">
                <tr>
                  <th className="px-3 py-1.5 font-semibold">Name</th>
                  <th className="px-3 py-1.5 font-semibold">Table ID</th>
                  <th className="px-3 py-1.5 font-semibold">Sync</th>
                  <th className="px-3 py-1.5 font-semibold">Primary key field</th>
                </tr>
              </thead>
              <tbody>
                {tables.map(([name, table]) => (
                  <tr key={name} className="border-t border-nest-border/60">
                    <td className="px-3 py-1.5 font-mono text-nest-foreground">{name}</td>
                    <td className="px-3 py-1.5 font-mono text-nest-foreground">{table.table_id}</td>
                    <td className="px-3 py-1.5">
                      <Chip
                        label={table.sync ? "yes" : "no"}
                        color={table.sync ? "success" : "error"}
                        size="small"
                      />
                    </td>
                    <td className="px-3 py-1.5 font-mono text-nest-foreground">
                      {table.primary_key_field ?? "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </ConfigSection>

      <ConfigSection title="Sync">
        <Row label="dry_run" value={String(config.sync.dry_run)} />
        <Row label="continue_on_error" value={String(config.sync.continue_on_error)} />
        <Row label="max_parallel_tables" value={String(config.sync.max_parallel_tables)} />
        <Row label="max_parallel_updates" value={String(config.sync.max_parallel_updates)} />
        <Row label="create_change_plan" value={String(config.sync.create_change_plan)} />
      </ConfigSection>

      <ConfigSection title="CSV">
        <Row label="location_data_file" value={config.csv.location_data_file} />
        <Row label="space_data_file" value={config.csv.space_data_file} />
      </ConfigSection>

      <ConfigSection title="Database">
        <Row label="provider" value={config.database.provider} />
        <Row label="database_path" value={config.database.database_path} />
        <Row label="schema" value={config.database.schema} />
      </ConfigSection>

      <ConfigSection title="Logging">
        <Row label="level" value={config.logging.level} />
        <Row label="directory" value={config.logging.directory} />
      </ConfigSection>
    </div>
  );
}

function ConfigSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-nest-muted">{title}</h2>
      <div className="overflow-hidden rounded-nest-md border border-nest-border bg-nest-background">
        {children}
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start gap-3 border-b border-nest-border/60 px-3 py-1.5 last:border-b-0">
      <span className="w-40 shrink-0 font-mono text-[11px] text-nest-muted">{label}</span>
      <span className="min-w-0 flex-1 break-all font-mono text-[11px] text-nest-foreground">{value}</span>
    </div>
  );
}
