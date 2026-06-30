# Airtable Sync

Sync tooling for [Airtable](https://airtable.com), built on the [Nest](https://github.com/pacificnm/nest) framework.

This is the **product repository**. Application code lives here — not in the Nest framework monorepo.

## Crates

| Crate | Path | Role |
|-------|------|------|
| `airtable-sync-core` | `crates/core/` | Shared modules, commands, and sync logic |
| `airtable-sync-cli` | `crates/cli/` | CLI binary (`airtable-sync`) |

Planned: `crates/gui/` (`airtable-sync-gui`).

## Build

```bash
cp config.example.toml config.toml
export AIRTABLE_TOKEN="pat..."

./build build
./build release
./build run -- tables
./build run -- list assets --json
```

- Binary: `target/release/airtable-sync`
- Config: `config.toml` (auto-loaded on `run`)
- Logs: `./logs/` (see `config.example.toml`)

## Nest dependencies

Nest crates are pulled from [github.com/pacificnm/nest](https://github.com/pacificnm/nest) via `git` in the workspace `Cargo.toml`.

For local framework development, override with a `.cargo/config.toml` `patch` section pointing at a sibling `nest` checkout.

## Configuration

See `config.example.toml`. Copy to `config.toml` and set `base_id`, table sections, and `AIRTABLE_TOKEN`.

## Commands

| Command | Description |
|---------|-------------|
| `tables` | List configured logical table names |
| `list <table>` | Fetch all records from a table (`--json` for output) |
