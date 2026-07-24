//! `airtable records` integration tests.

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

fn write_fixture(dir: &tempfile::TempDir, api_url: &str) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("location.csv"), "id,name\n1,Alice\n").unwrap();
    fs::write(dir.path().join("space.csv"), "id\n1\n").unwrap();
    fs::write(dir.path().join("schema.sql"), SCHEMA_SQL).unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        format!(
            r#"
[airtable]
api_url = "{api_url}"
meta_api_url = "https://example.invalid/meta"
token = "pat-test"
base_id = "appTEST"

[airtable.tables.assets]
table_id = "tblTEST"
sync = true
primary_key_field = "ID"

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

async fn mount_list_records_paginated(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/appTEST/tblTEST"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(|request: &wiremock::Request| {
            let has_offset = request.url.query_pairs().any(|(key, _)| key == "offset");
            if has_offset {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "records": [
                        { "id": "recB", "createdTime": "2024-01-01T00:00:00.000Z", "fields": { "ID": "2", "Name": "Bob" } }
                    ]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "records": [
                        { "id": "recA", "createdTime": "2024-01-01T00:00:00.000Z", "fields": { "ID": "1", "Name": "Alice" } }
                    ],
                    "offset": "itrXXX"
                }))
            }
        })
        .mount(server)
        .await;
}

#[tokio::test]
async fn records_fetches_every_page_for_table() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    mount_list_records_paginated(&server).await;

    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir, &server.uri());
    init_database(&config_path);

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "airtable",
            "records",
            "assets",
        ])
        .unwrap();
}

#[tokio::test]
async fn records_fails_for_unconfigured_table() {
    let _lock = test_lock();
    let server = MockServer::start().await;

    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir, &server.uri());
    init_database(&config_path);

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "airtable",
            "records",
            "missing",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Config);
}
