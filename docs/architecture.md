# Airtable Sync architecture

## Host model

Airtable Sync uses the Nest **multi-host** pattern: one shared core, multiple presentation hosts.

```text
┌────────────────────┐ ┌────────────────────┐ ┌──────────────────────┐
│ airtable-sync-cli  │ │ airtable-sync-gui  │ │ airtable-sync-tauri  │
│ (nest-cli)         │ │ (nest-gui, egui)   │ │ (nest-tauri + React) │
└─────────┬──────────┘ └─────────┬──────────┘ └──────────┬───────────┘
          │                      │                        │
          │         invokes the same core commands        │
          └──────────────────────┬────────────────────────┘
                                 ▼
                      ┌─────────────────────┐
                      │  airtable-sync-core │
                      │  commands + logic   │
                      └──────────┬──────────┘
                                 │
                                 ▼
                      Nest framework (nest-core, nest-config, modules, …)
```

## CLI owns execution

All product behavior is built **once** as CLI commands in `airtable-sync-core`. The GUI is a pretty wrapper and display layer — it shows progress, forms, and results while dispatching to those same commands.

The GUI does **not**:

- Reimplement sync, compare, mapping, or schema logic in UI code
- Talk to Airtable or SQLite directly (except through command handlers shared with the CLI)

The GUI **does**:

- Render workflow screens (setup, mapping review, dry-run approval, …)
- Call the same command entry points the terminal uses (e.g. `setup init`, `sync dry-run`, `sync apply`)
- Surface command output, logs, and progress to the user

## Why

- **One implementation** — no CLI/GUI drift
- **Scriptable** — automation and CI use the same commands as the desktop app
- **Testable** — integration tests target command handlers; GUI tests mock dispatch

## Crates

| Crate | Responsibility |
|-------|----------------|
| `airtable-sync-core` | Command tree, handlers, shared services |
| `airtable-sync-cli` | `main` → `cli_app().run()` |
| `airtable-sync-gui` | `nest-gui` (egui) host; dispatches to core commands (command grid + output panel) |
| `src-tauri/` (`airtable-sync-tauri`) | `nest-tauri` host + React `ui/`; dispatches to core via `CommandDispatch` |

## Desktop host (Tauri + React)

The Tauri host is the modern desktop front end (the egui `-gui` crate remains as
the legacy host). It follows the Nest desktop platform: `ui/` (React + Tailwind)
+ `src-tauri/` (`nest-tauri`), themed with the default **`cbre-light`** theme and
the shared shell components promoted to [`templates/desktop`](../../../templates/desktop).

- **No new business logic.** `src-tauri/src/commands.rs` exposes two IPC commands:
  - `airtable_sync_run(args, json)` → runs a subcommand through
    `airtable_sync_core::CommandDispatch` on a blocking thread and returns
    `{ success, stdout, error }`.
  - `airtable_sync_command_groups()` → returns the core `COMMAND_GROUPS` tree.
- **UI.** A `SyncRibbon` (File · Setup · Mapping · Compare · Sync · Report · Help)
  maps ribbon buttons to core subcommands; results stream into a command console.
- **Run it:** `./build desktop` (dev) or `./build desktop-build` (bundle).

```text
src-tauri/  → Tauri host (own cargo workspace; path deps to nest crates)
ui/         → React app (Vite); shared shell copied from templates/desktop
```

## Related

- [README](../README.md) — command workflow
- [plan/milestone-1-cli-help.md](plan/milestone-1-cli-help.md) — CLI host proof
- [nest-tauri](../../../docs/nest-tauri/README.md) — desktop host crate
- [templates/desktop](../../../templates/desktop/README.md) — shared shell + theme
