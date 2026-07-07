import { useCallback, useEffect, useRef, useState } from "react";
import { AppShell, Icon, useStatusBar, useToast } from "./shell";
import { SyncRibbon } from "./components/SyncRibbon";
import { faBroom, faTrash } from "./lib/fontawesome";
import { runCommand } from "./lib/airtable";
import { quitApp } from "./lib/tauri";
import { applyThemeRootBlock, fetchAppMetadata, fetchThemeCss } from "./lib/nest";

type LogEntry = {
  id: string;
  command: string;
  status: "running" | "ok" | "failed";
  text: string;
  at: string;
};

export function App() {
  const [activeTab, setActiveTab] = useState("sync");
  const [log, setLog] = useState<LogEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const consoleRef = useRef<HTMLDivElement>(null);
  const toast = useToast();
  const { setStatus } = useStatusBar();

  useEffect(() => {
    void (async () => {
      try {
        const [, theme] = await Promise.all([fetchAppMetadata(), fetchThemeCss()]);
        applyThemeRootBlock(theme.root_block);
      } catch {
        // Vite-only dev (no Tauri host): keep CSS-variable fallbacks.
      }
    })();
  }, []);

  useEffect(() => {
    const node = consoleRef.current;
    if (node) {
      node.scrollTop = node.scrollHeight;
    }
  }, [log]);

  const handleRun = useCallback(
    async (args: string[], label: string) => {
      if (busy) {
        return;
      }
      setBusy(true);
      const id = crypto.randomUUID();
      const at = new Date().toLocaleTimeString();
      setStatus(`Running ${label}…`, { variant: "info" });
      setLog((prev) => [
        ...prev,
        { id, command: label, status: "running", text: "Running…", at },
      ]);

      const result = await runCommand(args, true);
      const text =
        (result.stdout?.trim() ? result.stdout.trim() : "") +
        (result.error ? `\n${result.error}` : "");

      setLog((prev) =>
        prev.map((entry) =>
          entry.id === id
            ? {
                ...entry,
                status: result.success ? "ok" : "failed",
                text: text.trim() || (result.success ? "(no output)" : "(failed)"),
              }
            : entry,
        ),
      );

      if (result.success) {
        toast.success(`${label} completed`);
        setStatus(`${label} completed`, { variant: "success", timeoutMs: 4000 });
      } else {
        toast.error(`${label} failed`);
        setStatus(`${label} failed`, { variant: "error", timeoutMs: 6000 });
      }
      setBusy(false);
    },
    [busy, setStatus, toast],
  );

  const ribbon = (
    <SyncRibbon
      activeTab={activeTab}
      onTabChange={setActiveTab}
      onRun={handleRun}
      onQuit={() => void quitApp()}
      busy={busy}
    />
  );

  return (
    <AppShell
      ribbon={ribbon}
      statusLeft={<span>{busy ? "Running" : "Ready"}</span>}
      statusRight={<span>Airtable Sync</span>}
    >
      <div className="flex h-full min-h-0 flex-col gap-3 p-4">
        <div className="flex items-center justify-between">
          <h1 className="text-sm font-semibold text-nest-foreground">Command output</h1>
          <button
            type="button"
            onClick={() => setLog([])}
            disabled={log.length === 0}
            className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border px-2 py-1 text-xs text-nest-muted hover:bg-nest-muted/10 disabled:opacity-40"
          >
            <Icon icon={faTrash} className="size-3" />
            Clear
          </button>
        </div>

        <div
          ref={consoleRef}
          className="min-h-0 flex-1 overflow-auto rounded-nest-md border border-nest-border bg-nest-surface p-3"
        >
          {log.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-center text-nest-muted">
              <Icon icon={faBroom} className="size-6 opacity-50" />
              <p className="text-sm">No commands run yet.</p>
              <p className="text-xs">Pick an action from the ribbon above to get started.</p>
            </div>
          ) : (
            <ul className="flex flex-col gap-2">
              {log.map((entry) => (
                <li
                  key={entry.id}
                  className="rounded-nest-sm border border-nest-border bg-nest-background"
                >
                  <div className="flex items-center justify-between border-b border-nest-border px-3 py-1.5">
                    <span className="font-mono text-xs text-nest-foreground">
                      airtable-sync {entry.command}
                    </span>
                    <span
                      className={[
                        "rounded-nest-sm px-1.5 py-0.5 text-[10px] font-medium uppercase",
                        entry.status === "ok"
                          ? "bg-nest-success/15 text-nest-success"
                          : entry.status === "failed"
                            ? "bg-nest-error/15 text-nest-error"
                            : "bg-nest-info/15 text-nest-info",
                      ].join(" ")}
                    >
                      {entry.status === "running" ? "running" : entry.status}
                      <span className="ml-2 text-nest-muted">{entry.at}</span>
                    </span>
                  </div>
                  <pre className="max-h-64 overflow-auto whitespace-pre-wrap px-3 py-2 font-mono text-[11px] leading-relaxed text-nest-foreground">
                    {entry.text}
                  </pre>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </AppShell>
  );
}
