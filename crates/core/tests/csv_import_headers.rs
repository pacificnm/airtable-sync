//! `csv import-headers` integration tests.

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

fn csv_field_count(db_path: &std::path::Path) -> i64 {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM csv_fields", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn import_headers_keeps_shared_column_names_per_file() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);
    init_database(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "csv",
            "import-headers",
        ])
        .unwrap();

    let db_path = dir.path().join("data/app.db");
    // location: id, name, building_id (3) + space: ID, space_name, area (3) = 6
    assert_eq!(csv_field_count(&db_path), 6);

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let id_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM csv_fields WHERE normalized_name = 'id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(id_rows, 2);
}

#[test]
fn import_headers_replaces_previous_rows_on_rerun() {
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

    fs::write(
        dir.path().join("location.csv"),
        "only_column\n1\n",
    )
    .unwrap();
    fs::write(dir.path().join("space.csv"), "another\n2\n").unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "import-headers",
        ])
        .unwrap();

    let db_path = dir.path().join("data/app.db");
    assert_eq!(csv_field_count(&db_path), 2);

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let rows: Vec<(String, String)> = conn
        .prepare(
            "SELECT filename, normalized_name FROM csv_fields ORDER BY filename, normalized_name",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    assert_eq!(
        rows,
        vec![
            ("location.csv".to_string(), "only_column".to_string()),
            ("space.csv".to_string(), "another".to_string()),
        ]
    );
}

#[test]
fn import_headers_fails_when_database_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "import-headers",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Data);
}
