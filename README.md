# Airtable Sync

Sync data between CSV sources and [Airtable](https://airtable.com), built on the [Nest](https://github.com/pacificnm/nest) framework.

This README is organized around the **user workflow** — from first-time setup through configuration, mapping, comparison, and synchronization. Commands are grouped by the step you are performing, not by implementation detail.

> **Status:** Milestone 1 complete — full command tree and branded `--help`. **`config validate`**, **`config show`**, **`config init`**, **`db init`**, **`db reset`**, **`db schema`**, and **`db migrate`** are implemented; other commands are stubs.

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

# 2. Initialize local database (first run only; use db reset --yes to recreate)
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
| `db init` | Create the SQLite database (first time only) |
| `db reset --yes` | Recreate the database (**destructive**; deletes all local sync data) |
| `db schema` | Introspect the live SQLite database (tables, columns, migrations); requires an existing DB |
| `db migrate` | Apply pending database migrations; creates the DB if missing |

#### `db schema`

Read-only inspection of the database at `[database].database_path`. Does not print the schema SQL file — it queries the live database (tables, columns, applied migrations from `_nest_migrations`).

Requires the database file to exist (run `db init` or `db reset --yes` first):

```bash
airtable-sync db schema
airtable-sync db schema --json
```

#### `db migrate`

Applies pending migrations from the product registry (currently `001_initial_schema` from `[database].schema`). Safe to re-run — no-op when already up to date. Does **not** delete data (unlike `db reset`).

If the database file does not exist, `db migrate` creates it and applies all migrations (similar to `db init`, but `db init` fails when the file already exists).

```bash
airtable-sync db migrate
airtable-sync db migrate --json
```

Use `db init` for an explicit first-time create, or `db migrate` for upgrades and catch-up.

#### `db init` vs `db reset`

Both commands validate config, apply the schema from `[database].schema`, and record migration `001_initial_schema`. Use one or the other — not both in sequence.

| Situation | Command |
|-----------|---------|
| No database file yet (first run) | `db init` |
| Database already exists and you want a clean slate | `db reset --yes` |
| Just ran `db reset --yes` successfully | **Do nothing** — the database is already created |

`db init` **fails** if the database file already exists (even an empty file). That is intentional — use `db reset --yes` to recreate instead.

`db reset` **requires `--yes`** before it deletes anything. Without it, the command refuses to run:

```bash
# First time — no database file yet
airtable-sync db init

# Wipe and recreate (destructive)
airtable-sync db reset --yes

# After a successful reset, skip db init — reset already recreated the database
```

When developing locally with `./build run`, rebuild after code changes so you are not running a stale binary:

```bash
./build build
./build run -- db init
./build run -- db reset --yes
./build run -- db migrate
```

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

Copy `config.example.toml` to `config.toml` (or run `config init`). `config.toml` is gitignored — it holds environment-specific paths, table IDs, and secrets.

Most commands load `config.toml` from the current directory automatically when present.

### Sections overview

| Section | Purpose |
|---------|---------|
| `[airtable]` | Airtable API connection (base URL, token, base ID) |
| `[airtable.tables.<name>]` | One block per logical table — Airtable table ID and sync enablement |
| `[sync]` | Global synchronization behavior (dry-run, parallelism, change plans) |
| `[csv]` | Input CSV file paths for location and space data |
| `[database]` | Local SQLite database path and schema script |
| `[logging]` | Log level and output directory |

### `[airtable]`

| Key | Description |
|-----|-------------|
| `api_url` | Airtable REST API base URL (default `https://api.airtable.com/v0`) |
| `token` | Personal access token stored directly in gitignored `config.toml` (preferred) |
| `token_env` | Environment variable name for the token, or legacy direct token value in config |
| `base_id` | Airtable base ID (`app…`) |

### `[airtable.tables.<name>]`

Define one section per table using a stable **logical name** (snake_case). Commands and mappings refer to this name, not the Airtable table title.

| Key | Description |
|-----|-------------|
| `table_id` | Airtable table ID (`tbl…`) |
| `sync` | When `true`, the table is included in `sync all` and bulk compare operations (default `false` if omitted) |
| `primary_key_field` | Optional. Airtable field name used as the record key for compare/sync |

Example — enable sync for a subset of tables:

```toml
[airtable.tables.building]
table_id = "tblXXXXXXXXXXXXXX"
sync = true

[airtable.tables.city]
table_id = "tblXXXXXXXXXXXXXX"
sync = true

[airtable.tables.department]
table_id = "tblXXXXXXXXXXXXXX"
sync = false
```

### `[sync]`

| Key | Description |
|-----|-------------|
| `dry_run` | When `true`, sync commands plan changes without writing to Airtable |
| `continue_on_error` | When `true`, keep processing other tables/records after a non-fatal error |
| `max_parallel_tables` | Maximum tables processed concurrently |
| `max_parallel_updates` | Maximum concurrent update operations per table |
| `create_change_plan` | When `true`, persist a change plan to SQLite before apply |

### `[csv]`

| Key | Description |
|-----|-------------|
| `location_data_file` | Path to the location CSV used for header import, mapping, and compare |
| `space_data_file` | Path to the space CSV used for header import, mapping, and compare |

### `[database]`

| Key | Description |
|-----|-------------|
| `provider` | Database backend (`sqlite`) |
| `database_path` | Path to the SQLite file (schema, mappings, sync history, change plans) |
| `schema` | Path to the SQL schema file applied by `db init`, `db reset`, and `db migrate` |

### `[logging]`

| Key | Description |
|-----|-------------|
| `level` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `directory` | Directory for log files (e.g. `./logs`) |

### Full example

```toml
[airtable]
api_url = "https://api.airtable.com/v0"
token = "pat..."
base_id = "appXXXXXXXXXXXXXX"

[sync]
dry_run = true
continue_on_error = true
max_parallel_tables = 2
max_parallel_updates = 5
create_change_plan = true

[airtable.tables.building]
table_id = "tblXXXXXXXXXXXXXX"
sync = true

[airtable.tables.city]
table_id = "tblXXXXXXXXXXXXXX"
sync = true

[airtable.tables.people]
table_id = "tblXXXXXXXXXXXXXX"
# sync omitted — treated as disabled

[csv]
location_data_file = "/path/to/location.csv"
space_data_file = "/path/to/space.csv"

[database]
provider = "sqlite"
database_path = "/path/to/airtable-sync.db"
schema = "./schema/airtable-sync.sql"

[logging]
level = "info"
directory = "./logs"
```

Set credentials in gitignored `config.toml` (`token` preferred) or export the env var named by `token_env`:

```bash
export AIRTABLE_TOKEN="pat..."   # when using token_env = "AIRTABLE_TOKEN"
```

---

## Dependencies

Airtable Sync is built on the Nest framework and uses the following core crates:

| Crate | Purpose |
|--------|---------|
| **nest-core** | Core module system, dependency injection, application context, and shared services. |
| **nest-cli** | Command-line host, command registration, argument parsing, and application startup. |
| **nest-config** | Loads and validates `config.toml` application configuration. |
| **nest-error** | Structured error handling and application-wide error reporting. |
| **nest-logging** | Logging initialization, log configuration, and tracing integration. |
| **nest-file** | Safe file operations including reading, writing, copying, moving, and path validation. |
| **nest-file-csv** | CSV parsing, serialization, column mapping, and import/export utilities. |
| **nest-http** | Shared HTTP contracts, request/response models, and common HTTP types. |
| **nest-http-client** | HTTP client for communicating with external services, including Airtable. |
| **nest-airtable** | Airtable API client, schema discovery, record operations, batching, and synchronization helpers. |
| **nest-task** | Background task execution, progress reporting, cancellation, and async task management. |
| **nest-validation** | Validation framework for configuration, CSV data, mappings, and synchronization rules. |

### Additional libraries

| Library | Purpose |
|---------|---------|
| **rusqlite** | Local SQLite database used to store Airtable schema, field mappings, application settings, sync history, and change plans. |
| **serde** | Serialization and deserialization of configuration and application models. |
| **toml** | Parsing and writing application configuration files. |
| **tracing** | Structured instrumentation used throughout the application. |

---

## GUI (planned)

The desktop host (`airtable-sync-gui`, via `nest-gui`) is a **front end only**. It does not implement sync, mapping, or database logic itself.

**Rule:** the GUI runs the same commands as the CLI. Buttons, wizards, and progress views invoke the existing CLI command pipeline (e.g. `setup init`, `mapping auto`, `sync dry-run`) — typically through `airtable-sync-core` and `CliApp::try_run_with` with explicit args, not duplicate business code in the GUI crate.

| Host | Role |
|------|------|
| **CLI** (`airtable-sync-cli`) | Primary execution surface — all behavior lives in command handlers in `airtable-sync-core`. |
| **GUI** (planned) | Presents workflow UI; dispatches to the same commands the CLI exposes. |

First Run in the GUI maps to `setup init`. Compare, mapping review, and sync approval map to the corresponding CLI groups documented above.

See [docs/architecture.md](docs/architecture.md) for the host model.

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
