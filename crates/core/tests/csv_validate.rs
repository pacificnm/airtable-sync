//! `csv validate` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use tempfile::tempdir;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_fixture(dir: &tempfile::TempDir, location_csv: &str, space_csv: &str) -> std::path::PathBuf {
    const SCHEMA_SQL: &str = include_str!("../../../schema/airtable-sync.sql");

    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("location.csv"), location_csv).unwrap();
    fs::write(dir.path().join("space.csv"), space_csv).unwrap();
    fs::write(dir.path().join("schema.sql"), SCHEMA_SQL).unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[airtable]
api_url = "https://api.airtable.com/v0"
meta_api_url = "https://example.invalid/meta"
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
fn validate_both_files_succeeds_by_default() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(
        &dir,
        "id,name\n1,Main\n",
        "id,space_name\n1,Lobby\n",
    );

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "validate",
        ])
        .unwrap();
}

#[test]
fn validate_single_file_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(
        &dir,
        "id,name\n1,Main\n",
        "id\n1\n",
    );

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "validate",
            "location",
        ])
        .unwrap();
}

#[test]
fn validate_warns_on_duplicate_headers_but_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir, "id,ID\n1,2\n", "id\n1\n");

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "validate",
            "location",
        ])
        .unwrap();
}

#[test]
fn validate_fails_on_ragged_rows() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir, "id,name\n1\n", "id\n1\n");

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "validate",
            "location",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}

#[test]
fn validate_json_reports_invalid_file() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir, "id,name\n1\n", "id\n1\n");

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "csv",
            "validate",
            "location",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}
