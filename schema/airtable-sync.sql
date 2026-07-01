CREATE TABLE airtable_tables (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  table_id TEXT NOT NULL UNIQUE,
  enabled BOOLEAN NOT NULL DEFAULT 1,
  allow_create BOOLEAN NOT NULL DEFAULT 0,
  allow_update BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE airtable_fields (
  id INTEGER PRIMARY KEY,
  table_id TEXT NOT NULL,
  field_id TEXT,
  field_name TEXT NOT NULL,
  field_type TEXT,
  is_computed BOOLEAN NOT NULL DEFAULT 0,
  is_key BOOLEAN NOT NULL DEFAULT 0,
  sync_enabled BOOLEAN NOT NULL DEFAULT 0,
  csv_field TEXT,
  csv_filename TEXT,
  UNIQUE(table_id, field_name)
);

CREATE TABLE csv_fields (
  id INTEGER PRIMARY KEY,
  filename TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  UNIQUE(filename, normalized_name)
);