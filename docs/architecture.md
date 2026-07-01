# Airtable Sync architecture

## Host model

Airtable Sync uses the Nest **multi-host** pattern: one shared core, multiple presentation hosts.

```text
┌─────────────────────┐     ┌─────────────────────┐
│  airtable-sync-cli  │     │ airtable-sync-gui   │
│  (nest-cli)         │     │ (nest-gui)          │
└──────────┬──────────┘     └──────────┬──────────┘
           │                           │
           │    invokes same commands  │
           └─────────────┬─────────────┘
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
| `airtable-sync-gui` | `nest-gui` host; dispatches to core commands (v1: command grid + output panel) |

## Related

- [README](../README.md) — command workflow
- [plan/milestone-1-cli-help.md](plan/milestone-1-cli-help.md) — CLI host proof
