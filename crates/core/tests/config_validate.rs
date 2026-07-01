//! `config validate` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use tempfile::tempdir;

static TEST_LOCK: Mutex<()> = Mutex::new(());

const SCHEMA_SQL: &str = include_str!("../../../schema/airtable-sync.sql");

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

#[test]
fn validate_succeeds_for_valid_config() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap();
}

#[test]
fn validate_fails_when_csv_section_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
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
            "config",
            "validate",
        ])
        .unwrap_err();

    assert!(
        error.to_string().contains("csv")
            || error
                .help()
                .is_some_and(|help| help.to_lowercase().contains("csv"))
    );
}

#[test]
fn validate_fails_when_token_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let content = fs::read_to_string(&config_path).unwrap();
    let content = content.replace("token = \"pat-test\"\n", "");
    fs::write(&config_path, content).unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}

#[test]
fn validate_fails_when_csv_file_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    fs::remove_file(dir.path().join("location.csv")).unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}

#[test]
fn validate_fails_when_sync_enabled_table_missing_primary_key_field() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let content = fs::read_to_string(&config_path).unwrap();
    let content = content.replace("primary_key_field = \"Name\"\n", "");
    fs::write(&config_path, content).unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
    assert!(error.to_string().contains("primary_key_field"));
}

#[test]
fn validate_fails_when_primary_key_field_not_in_schema_cache() {
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

    let db_path = dir.path().join("data/app.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute(
        "INSERT INTO airtable_tables (name, table_id, enabled, allow_create, allow_update)
         VALUES ('assets', 'tblTEST', 1, 0, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO airtable_fields (table_id, field_id, field_name, field_type, is_computed, is_key)
         VALUES ('tblTEST', 'fldNAME', 'Name', 'singleLineText', 0, 1)",
        [],
    )
    .unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let content = content.replace(
        "primary_key_field = \"Name\"",
        "primary_key_field = \"Building ID\"",
    );
    fs::write(&config_path, content).unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
    assert!(error.to_string().contains("Building ID"));
    assert!(error.to_string().contains("not found in schema cache"));
}

#[test]
fn validate_accepts_token_env_literal() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    let content = fs::read_to_string(&config_path).unwrap();
    let content = content.replace(
        "token = \"pat-test\"",
        "token_env = \"pat-test-from-config\"",
    );
    fs::write(&config_path, content).unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap();
}
