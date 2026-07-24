# Setup

Ribbon location: **Setup** tab → **First run** group → **Init all**.

| Command | Ribbon button | Description |
|---------|----------------|-------------|
| `setup init` | Init all | First-run wizard: validate config, init the database, pull schema, import CSV headers, auto-map fields, report unmapped |

## `setup init`

Runs the whole first-time setup sequence in one command, in order:

1. **Validate `config.toml`** — same checks as `config validate`. Stops here if anything is blocking (missing token, bad `base_id`, unknown paths, …).
2. **Create/migrate the database** — same effect as `db init`, but safe to re-run: if the database already exists, it just applies any pending migrations instead of failing.
3. **Pull the Airtable schema** — same as `airtable pull-schema`. Needs network access and a working token.
4. **Import CSV headers** — same as `csv import-headers`. Needs the configured CSV files to exist on disk.
5. **Auto-map fields** — see below.
6. **Report unmapped fields** — same rollup `report validation`'s mapping section uses: how many fields are still unmapped across sync-enabled tables.

Each step's outcome is recorded independently. **Steps run in order and stop at the first failure** — a later step's prerequisites come from the step before it (there's no cached schema to auto-map against if the pull failed), so continuing would just fail again for a less useful reason.

```bash
airtable-sync setup init
airtable-sync --json setup init
```

`--json` returns every step's pass/fail plus each step's own structured result (schema pull counts, import counts, mapping counts):

```json
{
  "ok": true,
  "steps": [
    { "name": "validate config", "ok": true, "message": "config.toml is valid" },
    { "name": "database", "ok": true, "message": "created data/app.db and applied 2 migration(s)" },
    { "name": "pull schema", "ok": true, "message": "12 table(s), 84 field(s) upserted" },
    { "name": "import csv headers", "ok": true, "message": "37 field(s) imported" },
    { "name": "auto-map fields", "ok": true, "message": "22 field(s) mapped, 4 unresolved" },
    { "name": "unmapped fields", "ok": true, "message": "4 unmapped field(s) across sync-enabled tables" }
  ],
  "database_created": true,
  "schema": { "...": "full airtable pull-schema result" },
  "csv": { "...": "full csv import-headers result" },
  "mapping": { "...": "full mapping auto result" },
  "unmapped_summary": { "fields_total": 26, "mapped": 22, "unmapped": 4, "sync_enabled": 22, "mapped_sync_disabled": 0 }
}
```

**Idempotent.** Re-running `setup init` after a successful run is safe: the database step no-ops if there are no pending migrations, the schema/CSV steps just re-cache the current state, and auto-map never touches a field that's already mapped (see below) — it only fills in what's still missing.

### Auto-mapping (`mapping auto`)

Step 5 matches CSV columns to Airtable fields **by name**, case- and whitespace-insensitive (the same normalization used everywhere in the mapping layer). For each sync-enabled table:

- The table's CSV file is resolved from its `primary_key_field` — either an existing mapping, or (if unmapped) a name match that's unambiguous across both configured CSV files. If neither works, the table's other fields can only be auto-mapped from an unambiguous match too, since which file they belong to isn't known yet.
- Every other non-computed, not-yet-mapped field is matched against that file's columns by normalized name. A match maps the field **and enables sync** on it. No match leaves it unmapped — check `mapping report` (or the `unmapped_summary` above) afterward and map the rest by hand with `mapping set`.
- **Never overwrites an existing mapping.** If you've already run `mapping set` on a field, auto-map leaves it alone, whether run standalone (`mapping auto`) or as part of `setup init`.

```bash
airtable-sync mapping auto
airtable-sync --json mapping auto
```

### Desktop app

**Setup → Init all** runs `setup init --json` and shows the result in the command-output log, same as any other ribbon action — not a dedicated structured view. The JSON step list above is what appears there.
