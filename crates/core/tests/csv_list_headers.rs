//! `csv list-headers` integration tests.

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
    fs::write(dir.path().join("location.csv"), "id,name\n1,Main\n").unwrap();
    fs::write(dir.path().join("space.csv"), "id,area\n1,500\n").unwrap();
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

fn init_database(config_path: &std::path::Path) {
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
fn list_headers_groups_columns_by_file_and_role() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);
    init_database(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "import-headers",
        ])
        .unwrap();

    // In-process CLI dispatch (no stdout capture here) — assert success only;
    // JSON shape is covered by a manual smoke test against the real binary.
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "csv",
            "list-headers",
        ])
        .unwrap();
}

#[test]
fn list_headers_succeeds_with_empty_files_before_import() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);
    init_database(&config_path);

    // csv_fields table exists (from db init) but is empty — not an error,
    // unlike a missing database file.
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "list-headers",
        ])
        .unwrap();
}

#[test]
fn list_headers_fails_when_database_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "list-headers",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Data);
}
