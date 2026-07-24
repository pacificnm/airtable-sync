//! `mapping auto` integration tests.

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

[airtable.tables.archived]
table_id = "tblARCHIVED"
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

async fn mount_meta_schema(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/meta/bases/appTEST/tables"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tables": [
                {
                    "id": "tblTEST",
                    "name": "Assets",
                    "primaryFieldId": "fldKEY",
                    "fields": [
                        { "id": "fldKEY", "name": "Name", "type": "singleLineText" },
                        { "id": "fldSTATUS", "name": "Status", "type": "singleLineText" },
                        { "id": "fldFORM", "name": "Total", "type": "formula" }
                    ]
                },
                {
                    "id": "tblARCHIVED",
                    "name": "Archived",
                    "primaryFieldId": "fldARCHKEY",
                    "fields": [
                        { "id": "fldARCHKEY", "name": "Name", "type": "singleLineText" }
                    ]
                }
            ]
        })))
        .mount(server)
        .await;
}

fn init_database(config_path: &std::path::Path) {
    cli_app()
        .try_run_with(["airtable-sync", "--config", config_path.to_str().unwrap(), "db", "init"])
        .unwrap();
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

// Both `assets` (tblTEST) and `archived` (tblARCHIVED) have a field named
// "Name" in the mock schema below — table_id must be part of the lookup, or
// this can non-deterministically return whichever table's "Name" row SQLite
// happens to scan first.
fn field_mapping(db_path: &std::path::Path, table_id: &str, field_name: &str) -> (Option<String>, Option<String>, bool) {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.query_row(
        "SELECT csv_field, csv_filename, sync_enabled FROM airtable_fields WHERE table_id = ?1 AND field_name = ?2",
        [table_id, field_name],
        |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i32>(2)? != 0)),
    )
    .unwrap()
}

#[tokio::test]
async fn auto_maps_matching_fields_and_leaves_unmatched_unresolved() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_cache(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "auto",
        ])
        .unwrap();

    let db_path = dir.path().join("data/app.db");

    let (csv_field, csv_filename, sync_enabled) = field_mapping(&db_path, "tblTEST", "Name");
    assert_eq!(csv_field.as_deref(), Some("name"));
    assert_eq!(csv_filename.as_deref(), Some("location.csv"));
    assert!(sync_enabled);

    let (status_csv_field, _, _) = field_mapping(&db_path, "tblTEST", "Status");
    assert!(status_csv_field.is_none(), "Status has no matching CSV column");
}

#[tokio::test]
async fn auto_map_does_not_touch_sync_disabled_tables() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_cache(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "auto",
        ])
        .unwrap();

    let conn = rusqlite::Connection::open(dir.path().join("data/app.db")).unwrap();
    let (csv_field, sync_enabled): (Option<String>, i32) = conn
        .query_row(
            "SELECT csv_field, sync_enabled FROM airtable_fields WHERE table_id = 'tblARCHIVED' AND field_name = 'Name'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(csv_field.is_none());
    assert_eq!(sync_enabled, 0);
}

#[tokio::test]
async fn auto_map_does_not_overwrite_an_existing_mapping() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_meta_schema(&server).await;

    let dir = tempdir().unwrap();
    let meta_base_url = format!("{}/meta", server.uri());
    let config_path = write_fixture(&dir, &meta_base_url);
    init_database(&config_path);
    prepare_cache(&config_path);

    // Manually map Name to "id" first (deliberately not the name match).
    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "set",
            "assets",
            "Name",
            "id",
            "--csv-file",
            "location",
        ])
        .unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "mapping",
            "auto",
        ])
        .unwrap();

    let (csv_field, _, _) = field_mapping(&dir.path().join("data/app.db"), "tblTEST", "Name");
    assert_eq!(csv_field.as_deref(), Some("id"), "auto-map must not override an existing mapping");
}
