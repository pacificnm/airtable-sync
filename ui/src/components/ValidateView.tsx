import { useCallback, useEffect, useState } from "react";
import {
  configValidate,
  type ConfigValidateOutcome,
  type ConfigValidateResult,
} from "../lib/config";
import { Icon } from "../shell";
import { faRotate } from "../lib/fontawesome";
import { IssueSection, StatusBanner } from "./ConfigIssues";

/** Structured view for `config validate`: colored status banner + issue list. */
export function ValidateView() {
  const [outcome, setOutcome] = useState<ConfigValidateOutcome | null>(null);
  const [loading, setLoading] = useState(true);

  const run = useCallback(() => {
    setLoading(true);
    void configValidate()
      .then(setOutcome)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    run();
  }, [run]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-sm font-semibold text-nest-foreground">Configuration Validation</h1>
        <button
          type="button"
          onClick={run}
          disabled={loading}
          className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border px-2 py-1 text-xs text-nest-muted hover:bg-nest-muted/10 disabled:opacity-40"
        >
          <Icon icon={faRotate} className={["size-3", loading ? "animate-spin" : ""].join(" ")} />
          Re-validate
        </button>
      </div>

      {loading ? (
        <p className="text-sm text-nest-muted">Validating…</p>
      ) : outcome?.kind === "error" ? (
        <StatusBanner tone="error" title="Could not run config validate" detail={outcome.message} />
      ) : outcome?.kind === "result" ? (
        <ResultView result={outcome.result} />
      ) : null}
    </div>
  );
}

function ResultView({ result }: { result: ConfigValidateResult }) {
  const tableSummary = `${result.table_count} table${result.table_count === 1 ? "" : "s"} configured, ${result.sync_count} enabled for sync`;

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

      {result.errors.length > 0 ? (
        <IssueSection title="Errors" tone="error" issues={result.errors} />
      ) : null}

      {result.warnings.length > 0 ? (
        <IssueSection title="Warnings" tone="warning" issues={result.warnings} />
      ) : null}

      {result.valid && result.warnings.length === 0 ? (
        <StatusBanner tone="info" title="No issues found" detail="Every configured table passed validation." />
      ) : null}
    </div>
  );
}
