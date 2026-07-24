import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  Ribbon,
  RibbonButton,
  RibbonGroup,
  type RibbonIconTint,
  type RibbonTabDef,
} from "../shell";
import {
  faBan,
  faBook,
  faBroom,
  faCheck,
  faCircleInfo,
  faCloudArrowUp,
  faCodeCompare,
  faDatabase,
  faDiagramProject,
  faDoorOpen,
  faDownload,
  faFileCsv,
  faFileImport,
  faFileLines,
  faGear,
  faListCheck,
  faPlay,
  faPlug,
  faRotate,
  faScrewdriverWrench,
  faScroll,
  faTable,
} from "../lib/fontawesome";

export const SYNC_TABS: RibbonTabDef[] = [
  { id: "file", label: "File" },
  { id: "setup", label: "Setup" },
  { id: "mapping", label: "Mapping" },
  { id: "compare", label: "Compare" },
  { id: "sync", label: "Sync" },
  { id: "report", label: "Report" },
  { id: "data", label: "Data" },
  { id: "help", label: "Help" },
];

type Action = {
  label: string;
  icon: IconDefinition;
  /** CLI subcommand path, e.g. `["sync", "dry-run"]`. Omit for non-`"run"` kinds. */
  args?: string[];
  /**
   * `"docs"` opens the Help viewer; `"validate"`/`"show"`/`"automap"`/`"mappinglist"`/
   * `"mappingreport"`/`"tables"` open their structured views. All bypass `args`/CLI dispatch.
   */
  kind?: "run" | "docs" | "validate" | "show" | "automap" | "mappinglist" | "mappingreport" | "tables";
  tint?: RibbonIconTint;
  large?: boolean;
};

type Group = {
  label: string;
  actions: Action[];
};

const TAB_GROUPS: Record<string, Group[]> = {
  file: [
    {
      label: "Config",
      actions: [
        { label: "Validate", icon: faCheck, kind: "validate", tint: "primary", large: true },
        { label: "Show", icon: faFileLines, kind: "show", tint: "neutral" },
      ],
    },
  ],
  setup: [
    {
      label: "First run",
      actions: [
        { label: "Init all", icon: faScrewdriverWrench, args: ["setup", "init"], tint: "primary", large: true },
      ],
    },
    {
      label: "Database",
      actions: [
        { label: "DB init", icon: faDatabase, args: ["db", "init"], tint: "secondary" },
        { label: "Migrate", icon: faRotate, args: ["db", "migrate"], tint: "neutral" },
        { label: "Schema", icon: faTable, args: ["db", "schema"], tint: "neutral" },
      ],
    },
    {
      label: "Airtable",
      actions: [
        { label: "Test", icon: faPlug, args: ["airtable", "test"], tint: "info" },
        { label: "Pull schema", icon: faDownload, args: ["airtable", "pull-schema"], tint: "secondary" },
      ],
    },
    {
      label: "CSV",
      actions: [
        { label: "Import headers", icon: faFileImport, args: ["csv", "import-headers"], tint: "neutral" },
        { label: "Validate CSV", icon: faFileCsv, args: ["csv", "validate"], tint: "neutral" },
      ],
    },
  ],
  mapping: [
    {
      label: "Fields",
      actions: [
        { label: "Auto-map", icon: faDiagramProject, kind: "automap", tint: "primary", large: true },
        { label: "List", icon: faListCheck, kind: "mappinglist", tint: "neutral" },
        { label: "Report", icon: faFileLines, kind: "mappingreport", tint: "neutral" },
      ],
    },
  ],
  compare: [
    {
      label: "Compare",
      actions: [
        { label: "Compare all", icon: faCodeCompare, args: ["compare", "all"], tint: "primary", large: true },
      ],
    },
  ],
  sync: [
    {
      label: "Plan",
      actions: [
        { label: "Dry run", icon: faPlay, args: ["sync", "dry-run"], tint: "primary", large: true },
        { label: "Review", icon: faListCheck, args: ["sync", "review"], tint: "neutral" },
      ],
    },
    {
      label: "Decide",
      actions: [
        { label: "Approve all", icon: faCheck, args: ["sync", "approve-all"], tint: "secondary" },
        { label: "Deny all", icon: faBan, args: ["sync", "deny-all"], tint: "warning" },
      ],
    },
    {
      label: "Apply",
      actions: [
        { label: "Apply", icon: faCloudArrowUp, args: ["sync", "apply"], tint: "primary", large: true },
        { label: "Sync all", icon: faRotate, args: ["sync", "all"], tint: "secondary" },
      ],
    },
  ],
  report: [
    {
      label: "Reports",
      actions: [
        { label: "Summary", icon: faFileLines, args: ["report", "summary"], tint: "primary", large: true },
        { label: "Changes", icon: faCodeCompare, args: ["report", "changes"], tint: "neutral" },
        { label: "Validation", icon: faListCheck, args: ["report", "validation"], tint: "neutral" },
      ],
    },
  ],
  data: [
    {
      label: "Data",
      actions: [
        { label: "Tables", icon: faTable, kind: "tables", tint: "primary", large: true },
      ],
    },
  ],
  help: [
    {
      label: "Info",
      actions: [
        { label: "Version", icon: faCircleInfo, args: ["version"], tint: "info", large: true },
        { label: "Docs", icon: faBook, kind: "docs", tint: "secondary", large: true },
        { label: "Logs", icon: faScroll, args: ["logs", "show"], tint: "neutral" },
      ],
    },
    {
      label: "Maintenance",
      actions: [{ label: "Clear cache", icon: faBroom, args: ["cache", "clear"], tint: "warning" }],
    },
  ],
};

/** Which non-log view (if any) is currently displayed in the main content area. */
export type RibbonActiveView =
  | "log"
  | "docs"
  | "validate"
  | "show"
  | "settings"
  | "automap"
  | "mappinglist"
  | "mappingreport"
  | "tables";

type SyncRibbonProps = {
  activeTab: string;
  onTabChange: (tab: string) => void;
  onRun: (args: string[], label: string) => void;
  onOpenDocs: () => void;
  onOpenValidate: () => void;
  onOpenShow: () => void;
  onOpenSettings: () => void;
  onOpenAutoMap: () => void;
  onOpenMappingList: () => void;
  onOpenMappingReport: () => void;
  onOpenTables: () => void;
  onQuit: () => void;
  busy: boolean;
  /** Highlights the Docs/Validate/Show/Settings/Auto-map/List/Report/Tables button matching the view currently on screen. */
  activeView: RibbonActiveView;
  /** Args-key (`action.args.join(" ")`) of the last dispatched CLI command, for highlighting it while its output is showing in the log view. */
  activeRunKey: string | null;
};

export function SyncRibbon({
  activeTab,
  onTabChange,
  onRun,
  onOpenDocs,
  onOpenValidate,
  onOpenShow,
  onOpenSettings,
  onOpenAutoMap,
  onOpenMappingList,
  onOpenMappingReport,
  onOpenTables,
  onQuit,
  busy,
  activeView,
  activeRunKey,
}: SyncRibbonProps) {
  const groups = TAB_GROUPS[activeTab] ?? [];

  return (
    <Ribbon tabs={SYNC_TABS} activeTab={activeTab} onTabChange={onTabChange} fileTabId="file">
      <div className="flex h-full items-stretch">
        {groups.map((group) => (
          <RibbonGroup key={group.label} label={group.label}>
            {group.actions.map((action) => {
              const isRun = action.kind === "run" || action.kind === undefined;
              const runKey = (action.args ?? []).join(" ");
              const active = isRun
                ? activeView === "log" && activeRunKey === runKey
                : action.kind === activeView;

              return (
                <RibbonButton
                  key={action.label}
                  label={action.label}
                  icon={action.icon}
                  iconTint={action.tint}
                  large={action.large}
                  disabled={isRun ? busy : false}
                  active={active}
                  onClick={() => {
                    if (action.kind === "docs") {
                      onOpenDocs();
                    } else if (action.kind === "validate") {
                      onOpenValidate();
                    } else if (action.kind === "show") {
                      onOpenShow();
                    } else if (action.kind === "automap") {
                      onOpenAutoMap();
                    } else if (action.kind === "mappinglist") {
                      onOpenMappingList();
                    } else if (action.kind === "mappingreport") {
                      onOpenMappingReport();
                    } else if (action.kind === "tables") {
                      onOpenTables();
                    } else {
                      onRun(action.args ?? [], runKey);
                    }
                  }}
                />
              );
            })}
          </RibbonGroup>
        ))}

        {activeTab === "file" ? (
          <RibbonGroup label="App">
            <RibbonButton
              label="Quit"
              icon={faDoorOpen}
              iconTint="neutral"
              onClick={onQuit}
            />
            <RibbonButton
              label="Settings"
              icon={faGear}
              iconTint="neutral"
              active={activeView === "settings"}
              onClick={onOpenSettings}
            />
          </RibbonGroup>
        ) : null}
      </div>
    </Ribbon>
  );
}
