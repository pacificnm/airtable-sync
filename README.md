# Airtable Sync

Sync data between CSV sources and [Airtable](https://airtable.com), built on the [Nest](https://github.com/pacificnm/nest) framework.

This README is organized around the **user workflow** — from first-time setup through configuration, mapping, comparison, and synchronization. Commands are grouped by the step you are performing, not by implementation detail.

> **Status:** The CLI specification below defines the product interface. Implementation is in progress; run `./build run -- --help` to see what is available in your build.

---

## Quick start

**First run** — one command prepares everything:

```bash
cp config.example.toml config.toml   # or: airtable-sync config init
export AIRTABLE_TOKEN="pat..."

airtable-sync setup init
```

`setup init` performs:

- Validate configuration
- Create the SQLite database
- Download the Airtable schema
- Import CSV headers
- Auto-map matching fields
- Report unmapped fields

Then follow the workflow below for compare and sync.

**Build from source:**

```bash
./build build
./build run -- setup init    # passes args after --
```

---

## Typical workflow

```bash
# 1. Configure
airtable-sync config validate

# 2. Initialize local database
airtable-sync db init

# 3. Download Airtable schema
airtable-sync airtable pull-schema

# 4. Import CSV headers
airtable-sync csv import-headers

# 5. Auto-map fields
airtable-sync mapping auto

# 6. Review mappings
airtable-sync mapping list

# 7. Compare data
airtable-sync compare all

# 8. Generate change plan
airtable-sync sync dry-run

# 9. Apply updates
airtable-sync sync apply
```

Steps 1–5 are replaced by `airtable-sync setup init` on first run. Steps 6–9 are the repeat cycle before each sync.

---

## Commands

### Setup

| Command | Description |
|---------|-------------|
| `setup init` | First-run wizard: validate config, init DB, pull schema, import CSV headers, auto-map, report unmapped fields |

### Configuration

| Command | Description |
|---------|-------------|
| `config validate` | Validate `config.toml` |
| `config show` | Display the loaded configuration |
| `config init` | Create a default `config.toml` |

### Database

| Command | Description |
|---------|-------------|
| `db init` | Create the SQLite database |
| `db reset` | Recreate the database (**destructive**) |
| `db schema` | Display database schema information |
| `db migrate` | Apply database migrations |

### Airtable

| Command | Description |
|---------|-------------|
| `airtable test` | Test Airtable connectivity |
| `airtable pull-schema` | Download tables and fields into SQLite |
| `airtable list-tables` | List configured Airtable tables |
| `airtable list-fields` | List fields for a table |

### CSV

| Command | Description |
|---------|-------------|
| `csv import-headers` | Import CSV headers into SQLite |
| `csv preview` | Preview CSV records |
| `csv validate` | Validate CSV structure |

### Mapping

| Command | Description |
|---------|-------------|
| `mapping auto` | Auto-map CSV fields to Airtable fields |
| `mapping list` | Display current mappings |
| `mapping set` | Create or update a field mapping |
| `mapping remove` | Remove a mapping |
| `mapping enable` | Enable field synchronization |
| `mapping disable` | Disable field synchronization |
| `mapping report` | Generate mapping report |

### Compare

| Command | Description |
|---------|-------------|
| `compare table` | Compare one table |
| `compare all` | Compare every configured table |

### Sync

| Command | Description |
|---------|-------------|
| `sync dry-run` | Generate update plan only (no writes) |
| `sync apply` | Apply approved updates |
| `sync table` | Synchronize a single table |
| `sync all` | Synchronize all enabled tables |

### Reports

| Command | Description |
|---------|-------------|
| `report changes` | Generate change report |
| `report validation` | Validation report |
| `report summary` | Overall sync summary |

### Maintenance

| Command | Description |
|---------|-------------|
| `cache clear` | Clear cached schema |
| `logs show` | View recent logs |
| `version` | Display version information |

---

## Configuration

Copy `config.example.toml` to `config.toml` (or run `config init`). Set your Airtable base, table definitions, CSV paths, and database location.

```toml
[airtable]
api_url = "https://api.airtable.com/v0"
token_env = "AIRTABLE_TOKEN"
base_id = "appXXXXXXXXXXXXXX"

[airtable.tables.assets]
table_id = "tblXXXXXXXXXXXXXX"
primary_key_field = "Asset ID"

[logging]
level = "info"
directory = "./logs"
```

`run` and most commands load `config.toml` from the current directory automatically when present.

---

## GUI (planned)

The future desktop host will use the same setup pipeline: **First Run** invokes `setup init` under the hood, then exposes mapping, compare, and sync in the UI.

---

## Development

| Crate | Path | Role |
|-------|------|------|
| `airtable-sync-core` | `crates/core/` | Commands, sync logic, shared modules |
| `airtable-sync-cli` | `crates/cli/` | CLI binary (`airtable-sync`) |

```bash
./build build
./build test
./build release
```

- Binary: `target/release/airtable-sync`
- Logs: `./logs/`

Nest crates are pulled from [github.com/pacificnm/nest](https://github.com/pacificnm/nest). When cloned under `nest/apps/airtable-sync/`, `.cargo/config.toml` path-patches the local framework checkout.
