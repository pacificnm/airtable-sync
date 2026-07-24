# Airtable Sync

Airtable Sync keeps a CSV data source and an [Airtable](https://airtable.com) base in agreement. It maps CSV
columns to Airtable fields, compares the two sides record by record, and produces a reviewable change plan
before pushing any updates — nothing is written to Airtable without an explicit approve/apply step.

It ships as a Rust command tree (`airtable-sync-core`) with two hosts built on the [Nest](https://github.com/pacificnm/nest)
framework: a CLI (`airtable-sync-cli`) and a Tauri + React desktop app. Both hosts call the same command
handlers, so behavior is identical whether you drive Airtable Sync from a terminal or the desktop UI. See
[architecture.md](architecture.md) for how the pieces fit together.

## Workflow

1. **Configure** — point Airtable Sync at a CSV file, an Airtable base, and a local SQLite cache.
2. **Initialize** — create the database and pull the Airtable schema.
3. **Map** — match CSV columns to Airtable fields.
4. **Compare** — diff CSV rows against live Airtable records.
5. **Plan** — generate an update-only change plan (dry run).
6. **Review** — approve or deny individual changes.
7. **Apply** — push approved changes to Airtable.

`setup init` runs steps 2–3 in one command on first use (it needs a `config.toml` from step 1 already in place — run `config init` first if you don't have one yet); the rest is the repeat cycle before each sync. See [setup.md](setup.md) for exactly what it does and how it handles fields it can't auto-map.

## Commands

Commands are grouped the same way the CLI groups them (`airtable-sync <group> <command>`).

| Group | Covers | Docs |
|-------|--------|------|
| Setup | First-run wizard: validate, init DB, pull schema, import CSV, auto-map, report unmapped | [setup.md](setup.md) |
| Configuration | `config validate`, `config show`, `config init` | [config.md](config.md) |
| Database | `db init`, `db reset`, `db schema`, `db migrate` | *not yet written* |
| Airtable | `airtable test`, `airtable pull-schema`, `airtable list-tables`, `airtable list-fields` | *not yet written* |
| CSV | `csv import-headers`, `csv preview`, `csv validate` | *not yet written* |
| Mapping | `mapping auto`, `mapping list`, `mapping set`, `mapping remove`, `mapping enable`, `mapping disable`, `mapping report` | *not yet written* |
| Compare | `compare table`, `compare all` | *not yet written* |
| Sync | `sync dry-run`, `sync review`, `sync approve`/`deny`, `sync approve-all`/`deny-all`, `sync apply` | *not yet written* |
| Reports | `report changes`, `report validation`, `report summary` | *not yet written* |
| Maintenance | `cache clear`, `logs show`, `version` | *not yet written* |

Pages are added as each command group is documented. A group marked *not yet written* has no
page here yet — see the [README](../README.md#commands) for full command syntax in the
meantime.

## Other docs

- [architecture.md](architecture.md) — host model, crate layout, why the CLI owns execution
- [plan/](plan/) — implementation milestones
