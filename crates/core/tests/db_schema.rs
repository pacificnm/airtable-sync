//! `db schema` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
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

fn init_db(config_path: &std::path::Path) {
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "init",
        ])
        .unwrap();
}

#[test]
fn schema_succeeds_after_init() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    init_db(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "schema",
        ])
        .unwrap();
}

#[test]
fn schema_fails_when_database_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "schema",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Data);
    assert!(error.message().contains("not found"));
    assert!(
        error
            .help()
            .is_some_and(|help| help.contains("db init"))
    );
}

#[test]
fn schema_fails_when_config_invalid() {
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
            "schema",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}

#[test]
fn schema_json_output() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    init_db(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "db",
            "schema",
        ])
        .unwrap();
}

#[test]
fn schema_lists_expected_columns() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let db_path = dir.path().join("data/app.db");
    init_db(&config_path);

    let view =
        airtable_sync_core::db::introspect_database(&db_path, &dir.path().join("schema.sql"))
            .unwrap();

    let airtable_tables = view
        .tables
        .iter()
        .find(|table| table.name == "airtable_tables")
        .expect("airtable_tables should exist");
    assert!(
        airtable_tables
            .columns
            .iter()
            .any(|column| column.name == "table_id" && column.type_name == "TEXT")
    );
    assert!(view.migrations.contains(&"001_initial_schema".to_string()));
}
