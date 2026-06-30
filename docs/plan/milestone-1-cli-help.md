# Milestone 1: CLI Help & Nest Host Proof

## Goal

Prove the **Nest architecture** works for airtable-sync before any Airtable, CSV, or sync logic:

- `airtable-sync --help` shows a branded header and nine top-level command groups
- Placeholder commands register via `nest-cli` but do not implement behavior yet
- Minimal Nest wiring: **nest-core**, **nest-cli**, **nest-config**, **nest-error** (logging via nest-cli host)

## Success criteria

- [x] `./build build` succeeds
- [x] `./build run -- --help` shows `Airtable Sync` tagline and all command groups
- [x] `./build run -- version` exits 0 (stub)
- [x] `./build test` passes
- [x] `cargo test -p nest-cli` passes (help/version handling)

## M1 command groups (placeholders)

| Command | Help description | M1 behavior |
|---------|------------------|-------------|
| `config` | Configuration management | Stub (`Ok(())`) |
| `db` | SQLite database management | Stub |
| `airtable` | Airtable schema operations | Stub |
| `csv` | CSV import operations | Stub |
| `mapping` | Field mapping management | Stub |
| `compare` | Compare CSV to Airtable | Stub |
| `sync` | Synchronize Airtable | Stub |
| `report` | Generate reports | Stub |
| `version` | Display version information | Stub |

Future milestones add **nested subcommands** under each group (e.g. `config validate`, `sync dry-run`, `setup init`).

## Nest crates exercised

| Crate | Role in M1 |
|-------|------------|
| **nest-cli** | `CliApp`, command registry, clap help, bootstrap pipeline |
| **nest-config** | `ConfigLoader`, `ConfigService` registration |
| **nest-core** | `AppBuilder`, `AppContext`, module lifecycle |
| **nest-error** | Structured errors, exit codes |
| **nest-logging** | Host logging init (via nest-cli) |

## Explicit non-goals (M1)

- Airtable API calls (`nest-airtable`)
- SQLite / schema (`nest-data-sqlite`, `rusqlite`)
- CSV import (`nest-file-csv`)
- Config validation rules, mapping, compare, sync
- `setup init` wizard

## Framework changes

Small **nest-cli** enhancements for all Nest CLI apps:

- `CliApp::with_about` / `with_long_about`
- `--help` and `--version` print and return success (no `NestError`)

## Verification

```bash
cd apps/airtable-sync
./build build
./build run -- --help
./build run -- version
./build test
```

From nest repo root (after framework changes):

```bash
cargo test -p nest-cli
```

## Next: Milestone 2

- Implement command handlers starting with `config validate` / `config show` and `setup init`
- Wire `nest-data-sqlite` and schema from `[database]` in config
- Keep handlers in `airtable-sync-core` so the future GUI can invoke the same commands (see [architecture.md](../architecture.md))
