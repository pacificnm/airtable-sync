import type { ConfigIssue } from "../lib/config";
import { Icon } from "../shell";
import {
  faCircleCheck,
  faCircleExclamation,
  faCircleInfo,
  faTriangleExclamation,
} from "../lib/fontawesome";

const TONE_ICON = {
  success: faCircleCheck,
  error: faCircleExclamation,
  warning: faTriangleExclamation,
  info: faCircleInfo,
} as const;

const TONE_CLASS = {
  success: "border-nest-success/30 bg-nest-success/10 text-nest-success",
  error: "border-nest-error/30 bg-nest-error/10 text-nest-error",
  warning: "border-nest-warning/30 bg-nest-warning/10 text-nest-warning",
  info: "border-nest-info/30 bg-nest-info/10 text-nest-info",
} as const;

export type Tone = keyof typeof TONE_ICON;

/** Colored status banner: an icon + title + detail line, tinted by severity. */
export function StatusBanner({ tone, title, detail }: { tone: Tone; title: string; detail: string }) {
  return (
    <div className={["flex items-start gap-3 rounded-nest-md border px-4 py-3", TONE_CLASS[tone]].join(" ")}>
      <Icon icon={TONE_ICON[tone]} className="mt-0.5 size-4 shrink-0" />
      <div className="min-w-0">
        <p className="text-sm font-semibold">{title}</p>
        <p className="text-xs text-nest-foreground/80">{detail}</p>
      </div>
    </div>
  );
}

/** A titled list of config issues (errors or warnings), each with field/message/help. */
export function IssueSection({
  title,
  tone,
  issues,
}: {
  title: string;
  tone: "error" | "warning";
  issues: ConfigIssue[];
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-nest-muted">{title}</h2>
      <ul className="flex flex-col gap-1.5">
        {issues.map((issue, index) => (
          <li
            key={`${issue.field ?? "config"}-${index}`}
            className="flex items-start gap-2.5 rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-2"
          >
            <Icon
              icon={TONE_ICON[tone]}
              className={["mt-0.5 size-3.5 shrink-0", tone === "error" ? "text-nest-error" : "text-nest-warning"].join(" ")}
            />
            <div className="min-w-0 flex-1">
              {issue.field ? (
                <p className="font-mono text-[11px] text-nest-muted">{issue.field}</p>
              ) : null}
              <p className="text-xs text-nest-foreground">{issue.message}</p>
              {issue.help ? <p className="mt-0.5 text-[11px] text-nest-muted">{issue.help}</p> : null}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
