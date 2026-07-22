//! `sync dry-run` integration tests.

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

fn write_fixture(
    dir: &tempfile::TempDir,
    meta_base_url: &str,
    api_url: &str,
    continue_on_error: bool,
) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(
        dir.path().join("location.csv"),
        "id,name\n1,Alice\n2,Bob\n3,Charlie\n",
    )
    .unwrap();
    fs::write(dir.path().join("space.csv"), "id,name\n1,Room A\n").unwrap();
    fs::write(dir.path().join("schema.sql"), SCHEMA_SQL).unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        format!(
            r#"
[airtable]
api_url = "{api_url}"
meta_api_url = "{meta_base_url}"
token = "pat-test"
base_id = "appTEST"

[airtable.tables.assets]
table_id = "tblTEST"
sync = true
primary_key_field = "ID"

[airtable.tables.spaces]
table_id = "tblSPACE"
sync = true
primary_key_field = "ID"

[airtable.tables.buildings]
table_id = "tblBLDG"
sync = false
primary_key_field = "ID"

[sync]
dry_run = true
continue_on_error = {continue_on_error}
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
                    { "id": "fldKEY", "name": "ID", "type": "singleLineText" },
                    { "id": "fldNAME", "name": "Name", "type": "singleLineText" }
                ]
            }, {
                "id": "tblSPACE",
                "name": "Spaces",
                "primaryFieldId": "fldSKEY",
                "fields": [
                    { "id": "fldSKEY", "name": "ID", "type": "singleLineText" },
                    { "id": "fldSNAME", "name": "Name", "type": "singleLineText" }
                ]
            }, {
                "id": "tblBLDG",
                "name": "Buildings",
                "primaryFieldId": "fldBKEY",
                "fields": [
                    { "id": "fldBKEY", "name": "ID", "type": "singleLineText" }
                ]
            }]
        })))
        .mount(server)
        .await;
}

async fn mount_assets_records(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/appTEST/tblTEST"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "records": [
                {
                    "id": "recA",
                    "createdTime": "2024-01-01T00:00:00.000Z",
                    "fields": { "ID": "1", "Name": "Alice" }
                },
                {
                    "id": "recB",
                    "createdTime": "2024-01-01T00:00:00.000Z",
                    "fields": { "ID": "2", "Name": "Robert" }
                }
            ]
        })))
        .expect(1)
        .mount(server)
        .await;
}

fn prepare_cache(config_path: &std::path::Path) {
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "airtable",
            "pull-schema",
        ])
        .unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "import-headers",
        ])
        .unwrap();
}

fn set_assets_mappings(config_path: &std::path::Path) {
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "set",
            "assets",
            "ID",
            "id",
            "--csv-file",
            "location",
            "--enable",
        ])
        .unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "set",
            "assets",
            "Name",
            "name",
            "--csv-file",
            "location",
            "--enable",
        ])
        .unwrap();
}

fn operation_count(db_path: &std::path::Path) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM change_plan_operations",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

#[tokio::test]
async fn sync_dry_run_plans_updates_and_persists_change_plan() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;
    mount_assets_records(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url, &server.uri(), true);
    let db_path = dir.path().join("data/app.db");
    init_database(&config_path);
    prepare_cache(&config_path);
    set_assets_mappings(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "sync",
            "dry-run",
        ])
        .unwrap();

    assert!(operation_count(&db_path) >= 1);
}

#[tokio::test]
async fn sync_dry_run_continue_on_error_collects_failures() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;
    mount_assets_records(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url, &server.uri(), true);
    init_database(&config_path);
    prepare_cache(&config_path);
    set_assets_mappings(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "sync",
            "dry-run",
        ])
        .unwrap();
}

#[tokio::test]
async fn sync_dry_run_stops_on_error_when_continue_on_error_disabled() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;
    mount_assets_records(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url, &server.uri(), false);
    init_database(&config_path);
    prepare_cache(&config_path);
    set_assets_mappings(&config_path);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "sync",
            "dry-run",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}
