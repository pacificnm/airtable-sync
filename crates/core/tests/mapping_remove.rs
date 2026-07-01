//! `mapping remove` integration tests.

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
        .mount(server)
        .await;
}

fn prepare_schema(config_path: &std::path::Path) {
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "airtable",
            "pull-schema",
        ])
        .unwrap();
}

fn field_mapping(db_path: &std::path::Path, field_name: &str) -> (Option<String>, Option<String>, bool) {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.query_row(
        "SELECT csv_field, csv_filename, sync_enabled FROM airtable_fields WHERE field_name = ?1",
        [field_name],
        |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i32>(2)? != 0)),
    )
    .unwrap()
}

#[tokio::test]
async fn mapping_remove_clears_mapping_after_set() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_schema(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "csv",
            "import-headers",
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
            "--enable",
        ])
        .unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "remove",
            "assets",
            "Name",
        ])
        .unwrap();

    let (csv_field, csv_filename, sync_enabled) =
        field_mapping(&dir.path().join("data/app.db"), "Name");
    assert!(csv_field.is_none());
    assert!(csv_filename.is_none());
    assert!(!sync_enabled);
}

#[tokio::test]
async fn mapping_remove_succeeds_when_field_is_unmapped() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_schema(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "remove",
            "assets",
            "Name",
        ])
        .unwrap();
}

#[tokio::test]
async fn mapping_remove_fails_for_unknown_table_and_field() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_schema(&config_path);

    let table_error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "remove",
            "missing",
            "Name",
        ])
        .unwrap_err();
    assert_eq!(table_error.kind(), nest_error::NestErrorKind::Data);

    let field_error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "remove",
            "assets",
            "Missing",
        ])
        .unwrap_err();
    assert_eq!(field_error.kind(), nest_error::NestErrorKind::Data);
}

#[tokio::test]
async fn mapping_remove_rejects_computed_field() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_schema(&config_path);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "remove",
            "assets",
            "Total",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
}
