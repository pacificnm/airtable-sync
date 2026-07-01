//! `db migrate` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use rusqlite::Connection;
use tempfile::tempdir;

static TEST_LOCK: Mutex<()> = Mutex::new(());

const SCHEMA_SQL: &str = r#"
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
"#;

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_valid_fixture(dir: &tempfile::TempDir) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("location.csv"), "id\n1\n").unwrap();
    fs::write(dir.path().join("space.csv"), "id\n1\n").unwrap();
    fs::write(dir.path().join("schema.sql"), SCHEMA_SQL).unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[airtable]
token = "pat-test"
base_id = "appTEST"

[airtable.tables.assets]
table_id = "tblTEST"
sync = true
primary_key_field = "Name"

[sync]
dry_run = true
continue_on_error = true
max_parallel_tables = 2
max_parallel_updates = 5
create_change_plan = true

[csv]
location_data_file = "location.csv"
space_data_file = "space.csv"

[database]
provider = "sqlite"
database_path = "data/app.db"
schema = "schema.sql"

[logging]
level = "info"
directory = "logs"
"#,
    )
    .unwrap();

    config_path
}

fn migration_ids(db_path: &std::path::Path) -> Vec<String> {
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT id FROM _nest_migrations ORDER BY rowid ASC")
        .unwrap();
    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect()
}

#[test]
fn migrate_creates_database_when_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let db_path = dir.path().join("data/app.db");

    assert!(!db_path.exists());

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "migrate",
        ])
        .unwrap();

    assert!(db_path.is_file());
    assert_eq!(migration_ids(&db_path), vec!["001_initial_schema".to_string()]);
}

#[test]
fn migrate_is_noop_when_up_to_date() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "init",
        ])
        .unwrap();

    let result = airtable_sync_core::db::apply_pending_migrations(
        &dir.path().join("data/app.db"),
        &dir.path().join("schema.sql"),
        false,
    )
    .unwrap();

    assert!(result.applied.is_empty());
    assert_eq!(result.all_applied, vec!["001_initial_schema".to_string()]);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "migrate",
        ])
        .unwrap();
}

#[test]
fn migrate_fails_when_config_invalid() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[airtable]
base_id = "appTEST"

[sync]
dry_run = true
continue_on_error = true
max_parallel_tables = 2
max_parallel_updates = 5
create_change_plan = true

[csv]
location_data_file = "missing-location.csv"
space_data_file = "missing-space.csv"

[database]
provider = "sqlite"
database_path = "data/app.db"
schema = "schema.sql"

[logging]
level = "info"
directory = "logs"
"#,
    )
    .unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "migrate",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}

#[test]
fn migrate_json_output() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "db",
            "migrate",
        ])
        .unwrap();
}
