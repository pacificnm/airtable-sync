//! `airtable pull-schema` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use rusqlite::Connection;
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
    fs::write(dir.path().join("location.csv"), "id\n1\n").unwrap();
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

[airtable.tables.missing]
table_id = "tblMISSING"
sync = false

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

fn init_database(dir: &tempfile::TempDir, config_path: &std::path::Path) {
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "db",
            "init",
        ])
        .unwrap();
    assert!(dir.path().join("data/app.db").is_file());
}

fn field_count(db_path: &std::path::Path) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM airtable_fields", [], |row| row.get(0))
        .unwrap()
}

#[tokio::test]
async fn pull_schema_upserts_tables_and_fields() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/meta/bases/appTEST/tables"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tables": [{
                "id": "tblTEST",
                "name": "Assets",
                "primaryFieldId": "fldKEY",
                "fields": [
                    {
                        "id": "fldKEY",
                        "name": "Name",
                        "type": "singleLineText"
                    },
                    {
                        "id": "fldFORM",
                        "name": "Total",
                        "type": "formula"
                    }
                ]
            }]
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&dir, &config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "airtable",
            "pull-schema",
        ])
        .unwrap();

    let db_path = dir.path().join("data/app.db");
    assert_eq!(field_count(&db_path), 2);

    let conn = Connection::open(&db_path).unwrap();
    let name: String = conn
        .query_row(
            "SELECT name FROM airtable_tables WHERE table_id = 'tblTEST'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "assets");

    let computed: i32 = conn
        .query_row(
            "SELECT is_computed FROM airtable_fields WHERE field_name = 'Total'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(computed, 1);
}

#[tokio::test]
async fn pull_schema_fails_when_database_missing() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir, "https://example.invalid/meta");

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "airtable",
            "pull-schema",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Data);
}

#[tokio::test]
async fn pull_schema_warns_for_missing_configured_table() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/meta/bases/appTEST/tables"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tables": []
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&dir, &config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "airtable",
            "pull-schema",
        ])
        .unwrap();

    assert_eq!(field_count(&dir.path().join("data/app.db")), 0);
}
