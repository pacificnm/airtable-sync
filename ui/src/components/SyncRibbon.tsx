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
  { id: "help", label: "Help" },
];

type Action = {
  label: string;
  icon: IconDefinition;
  args: string[];
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
        { label: "Validate", icon: faCheck, args: ["config", "validate"], tint: "primary", large: true },
        { label: "Show", icon: faFileLines, args: ["config", "show"], tint: "neutral" },
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
        { label: "Auto-map", icon: faDiagramProject, args: ["mapping", "auto"], tint: "primary", large: true },
        { label: "List", icon: faListCheck, args: ["mapping", "list"], tint: "neutral" },
        { label: "Report", icon: faFileLines, args: ["mapping", "report"], tint: "neutral" },
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
  help: [
    {
      label: "Info",
      actions: [
        { label: "Version", icon: faCircleInfo, args: ["version"], tint: "info", large: true },
        { label: "Logs", icon: faScroll, args: ["logs", "show"], tint: "neutral" },
      ],
    },
    {
      label: "Maintenance",
      actions: [{ label: "Clear cache", icon: faBroom, args: ["cache", "clear"], tint: "warning" }],
    },
  ],
};

type SyncRibbonProps = {
  activeTab: string;
  onTabChange: (tab: string) => void;
  onRun: (args: string[], label: string) => void;
  onQuit: () => void;
  busy: boolean;
};

export function SyncRibbon({
  activeTab,
  onTabChange,
  onRun,
  onQuit,
  busy,
}: SyncRibbonProps) {
  const groups = TAB_GROUPS[activeTab] ?? [];

  return (
    <Ribbon tabs={SYNC_TABS} activeTab={activeTab} onTabChange={onTabChange} fileTabId="file">
      <div className="flex h-full items-stretch">
        {groups.map((group) => (
          <RibbonGroup key={group.label} label={group.label}>
            {group.actions.map((action) => (
              <RibbonButton
                key={action.label}
                label={action.label}
                icon={action.icon}
                iconTint={action.tint}
                large={action.large}
                disabled={busy}
                onClick={() => onRun(action.args, `${action.args.join(" ")}`)}
              />
            ))}
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
            <RibbonButton label="Settings" icon={faGear} iconTint="neutral" disabled />
          </RibbonGroup>
        ) : null}
      </div>
    </Ribbon>
  );
}
