//! `db reset` integration tests.

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
  UNIQUE(table_id, field_name)
);

CREATE TABLE csv_fields (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  normalized_name TEXT NOT NULL
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

fn table_names(db_path: &std::path::Path) -> Vec<String> {
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .unwrap();
    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect()
}

fn csv_field_count(db_path: &std::path::Path) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM csv_fields", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn reset_fails_without_yes() {
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

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "reset",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Command);
    assert!(error.message().contains("without --yes"));
}

#[test]
fn reset_recreates_existing_database() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let db_path = dir.path().join("data/app.db");

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "init",
        ])
        .unwrap();

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO csv_fields (name, normalized_name) VALUES ('test_field', 'test_field')",
            [],
        )
        .unwrap();
    }
    assert_eq!(csv_field_count(&db_path), 1);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "reset",
            "--yes",
        ])
        .unwrap();

    assert!(db_path.is_file());
    assert_eq!(
        table_names(&db_path),
        vec![
            "_nest_migrations".to_string(),
            "airtable_fields".to_string(),
            "airtable_tables".to_string(),
            "csv_fields".to_string(),
        ]
    );
    assert_eq!(csv_field_count(&db_path), 0);
}

#[test]
fn reset_creates_database_when_missing() {
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
            "reset",
            "--yes",
        ])
        .unwrap();

    assert!(db_path.is_file());
    assert_eq!(
        table_names(&db_path),
        vec![
            "_nest_migrations".to_string(),
            "airtable_fields".to_string(),
            "airtable_tables".to_string(),
            "csv_fields".to_string(),
        ]
    );
}

#[test]
fn reset_fails_when_config_invalid() {
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
            "reset",
            "--yes",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}

#[test]
fn reset_json_output() {
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
            "reset",
            "--yes",
        ])
        .unwrap();
}

#[test]
fn init_still_fails_when_db_exists() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let db_path = dir.path().join("data/app.db");
    fs::write(&db_path, "existing").unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "init",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Data);
    assert!(error.message().contains("already exists"));
}
