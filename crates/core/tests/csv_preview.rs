//! `csv preview` integration tests.

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

fn write_fixture(dir: &tempfile::TempDir) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(
        dir.path().join("location.csv"),
        "id,name,building_id\n1,Main,100\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("space.csv"),
        "ID,space_name,area\n1,Lobby,500\n",
    )
    .unwrap();
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
fn preview_location_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "csv",
            "preview",
            "location",
        ])
        .unwrap();
}

#[test]
fn preview_space_with_limit_one_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "preview",
            "space",
            "--limit",
            "1",
        ])
        .unwrap();
}

#[test]
fn preview_reports_truncation_when_more_rows_exist() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);
    fs::write(
        dir.path().join("location.csv"),
        "id,name\n1,One\n2,Two\n3,Three\n",
    )
    .unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "csv",
            "preview",
            "location",
            "--limit",
            "2",
        ])
        .unwrap();
}

#[test]
fn preview_rejects_unknown_file_role() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "preview",
            "missing",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Command);
}

#[test]
fn preview_fails_when_csv_file_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);
    fs::remove_file(dir.path().join("location.csv")).unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "preview",
            "location",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}
