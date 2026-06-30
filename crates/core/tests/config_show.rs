//! `config show` integration tests.

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

fn write_valid_fixture(dir: &tempfile::TempDir) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("location.csv"), "id\n1\n").unwrap();
    fs::write(dir.path().join("space.csv"), "id\n1\n").unwrap();
    fs::write(dir.path().join("schema.sql"), "-- schema\n").unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[airtable]
token = "pat-test-secret"
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

#[test]
fn show_succeeds_for_valid_config() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "show",
        ])
        .unwrap();
}

#[test]
fn show_fails_when_validation_fails() {
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
            "show",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
    assert!(error.message().contains("csv.location_data_file"));
}

#[test]
fn show_json_succeeds_for_valid_config() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "config",
            "show",
        ])
        .unwrap();
}

#[test]
fn show_respects_quiet() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--quiet",
            "config",
            "show",
        ])
        .unwrap();
}

#[test]
fn show_quiet_with_json_still_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--quiet",
            "--json",
            "config",
            "show",
        ])
        .unwrap();
}

#[test]
fn show_fails_same_as_validate_for_invalid_config() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir);
    fs::remove_file(dir.path().join("space.csv")).unwrap();

    let validate_error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "validate",
        ])
        .unwrap_err();

    let show_error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "show",
        ])
        .unwrap_err();

    assert_eq!(validate_error.message(), show_error.message());
}
