# Configuration

Ribbon location: **File** tab → **Config** group.

| Command | Ribbon button | Description |
|---------|----------------|-------------|
| `config validate` | Validate | Check `config.toml` for structural and semantic problems |
| `config show` | Show | Display the loaded configuration, secrets redacted |
| `config init` | *(CLI only)* | Create a default `config.toml` from the bundled template |

## `config validate`

Checks configured paths, the Airtable token, base id, and every `[airtable.tables.<name>]`
block. When the SQLite database exists and `airtable pull-schema` has already run, it also
verifies each table's `primary_key_field` against the cached Airtable field names — a
`primary_key_field` that doesn't match any cached field is reported as an error.

Blocking problems (missing token, missing CSV file, bad `primary_key_field`, …) are
**errors**; the command fails. Non-blocking problems (an unmapped `primary_key_field` on a
table that isn't sync-enabled yet, an unverifiable value because the schema cache hasn't
been pulled, …) are **warnings**; the command still succeeds.

### CLI

```bash
airtable-sync config validate
airtable-sync --json config validate
```

If validation reports an unknown `primary_key_field`, run `airtable list-fields <table>` and
copy the exact Airtable field name from the cache.

`--json` prints a structured payload instead of plain text:

```json
{
  "valid": false,
  "table_count": 12,
  "sync_count": 5,
  "errors": [
    {
      "field": "airtable.tables.assets.primary_key_field",
      "message": "primary_key_field is required for sync-enabled table \"assets\"",
      "help": "Set primary_key_field to the Airtable field name used to match CSV rows.",
      "severity": "error"
    }
  ],
  "warnings": []
}
```

| Field | Meaning |
|-------|---------|
| `valid` | `true` when `errors` is empty |
| `table_count` | Configured `[airtable.tables.<name>]` blocks |
| `sync_count` | Of those, how many have `sync = true` |
| `errors` / `warnings` | Issue lists; each has `field` (dotted config path, or `null`), `message`, optional `help`, and `severity` (`"error"` or `"warning"`) |

### Desktop app

**File → Validate** runs `config validate --json` and renders the result as a **Configuration
Validation** page instead of dumping raw text:

- A colored status banner at the top:
  - **green** ("Configuration valid") when there are no errors or warnings
  - **amber** ("Configuration valid, with warnings") when valid but warnings remain
  - **red** ("Configuration invalid — N errors") when blocking errors were found
- An **Errors** section (red) and a **Warnings** section (amber), each listing the affected
  config field, the message, and the remediation hint (`help`) when one is available
- A **Re-validate** button to re-run the check after editing `config.toml`

This view is available whenever the command can be dispatched, whether or not Airtable
credentials are valid — it's the fastest way to check `config.toml` before running
`setup init` or `sync dry-run`.

## `config show`

Displays the loaded configuration with secrets redacted (`token`/`token_env` show as
`(set)` or `(not set)`, never the raw value). Runs the same validation as `config validate`
first, so a broken config fails here too.

```bash
airtable-sync config show
airtable-sync --json config show
```

**File → Show** runs this in the desktop app's command-output log (not a dedicated view).

## `config init`

Writes `config.example.toml`'s template to `config.toml` (or `--output <path>`). Fails if the
target file already exists unless `--force` is passed. Not exposed in the ribbon — it's a
one-time setup step, normally run from the CLI before the desktop app is used, or implicitly
via `setup init`.

```bash
airtable-sync config init
airtable-sync config init --output ./config.local.toml
airtable-sync config init --force
```
