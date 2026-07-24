//! `setup init` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use tempfile::tempdir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

static TEST_LOCK: Mutex<()> = Mutex::new(());

const SCHEMA_SQL: &str = include_str!("../../../schema/airtable-sync.sql");

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_fixture(dir: &tempfile::TempDir, meta_base_url: &str) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("location.csv"), "id,name\n1,Test\n").unwrap();
    fs::write(dir.path().join("space.csv"), "id\n1\n").unwrap();
    fs::write(dir.path().join("schema.sql"), SCHEMA_SQL).unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        format!(
            r#"
[airtable]
api_url = "https://api.airtable.com/v0"
meta_api_url = "{meta_base_url}"
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
"#
        ),
    )
    .unwrap();

    config_path
}

async fn mount_meta_schema(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/meta/bases/appTEST/tables"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tables": [{
                "id": "tblTEST",
                "name": "Assets",
                "primaryFieldId": "fldKEY",
                "fields": [
                    { "id": "fldKEY", "name": "Name", "type": "singleLineText" },
                    { "id": "fldFORM", "name": "Total", "type": "formula" }
                ]
            }]
        })))
        .mount(server)
        .await;
}

#[test]
fn setup_init_fails_without_a_config_file() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            dir.path().join("config.toml").to_str().unwrap(),
            "setup",
            "init",
        ])
        .unwrap_err();

    // No file to load — should fail clearly rather than silently succeed.
    assert!(!error.message().is_empty());
}

#[test]
fn setup_init_stops_after_validation_when_config_is_invalid() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[airtable]
base_id = "appTEST"

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

[sync]
dry_run = true
continue_on_error = true
max_parallel_tables = 2
max_parallel_updates = 5
create_change_plan = true
"#,
    )
    .unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "setup",
            "init",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
    // Nothing past validation should have run — no database file created.
    assert!(!dir.path().join("data/app.db").exists());
}

#[tokio::test]
async fn setup_init_runs_the_full_wizard_end_to_end() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "setup",
            "init",
        ])
        .unwrap();

    let db_path = dir.path().join("data/app.db");
    assert!(db_path.is_file(), "setup init should create the database");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let table_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM airtable_tables", [], |row| row.get(0))
        .unwrap();
    assert_eq!(table_count, 1, "schema pull should have cached the assets table");

    let csv_field_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM csv_fields", [], |row| row.get(0))
        .unwrap();
    assert!(csv_field_count > 0, "csv headers should have been imported");

    let (mapped_csv_field, sync_enabled): (Option<String>, i32) = conn
        .query_row(
            "SELECT csv_field, sync_enabled FROM airtable_fields WHERE field_name = 'Name'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(mapped_csv_field.as_deref(), Some("name"), "auto-map should have matched Name -> name");
    assert_eq!(sync_enabled, 1);
}

#[tokio::test]
async fn setup_init_is_safe_to_rerun() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);

    for _ in 0..2 {
        cli_app()
            .try_run_with([
                "airtable-sync",
                "--config",
                config_path.to_str().unwrap(),
                "setup",
                "init",
            ])
            .unwrap();
    }

    let conn = rusqlite::Connection::open(dir.path().join("data/app.db")).unwrap();
    let table_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM airtable_tables", [], |row| row.get(0))
        .unwrap();
    assert_eq!(table_count, 1);
}
