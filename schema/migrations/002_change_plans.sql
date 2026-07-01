CREATE TABLE change_plans (
  id INTEGER PRIMARY KEY,
  created_at TEXT NOT NULL,
  base_id TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft',
  tables_planned INTEGER NOT NULL DEFAULT 0,
  operations_total INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE change_plan_operations (
  id INTEGER PRIMARY KEY,
  plan_id INTEGER NOT NULL REFERENCES change_plans(id) ON DELETE CASCADE,
  table_name TEXT NOT NULL,
  operation TEXT NOT NULL,
  record_key TEXT NOT NULL,
  airtable_record_id TEXT,
  status TEXT NOT NULL DEFAULT 'pending'
);

CREATE TABLE change_plan_field_changes (
  id INTEGER PRIMARY KEY,
  operation_id INTEGER NOT NULL REFERENCES change_plan_operations(id) ON DELETE CASCADE,
  field_name TEXT NOT NULL,
  old_value TEXT NOT NULL DEFAULT '',
  new_value TEXT NOT NULL
);
