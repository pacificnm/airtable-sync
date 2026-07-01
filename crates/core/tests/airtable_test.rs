//! `airtable test` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use tempfile::tempdir;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

static TEST_LOCK: Mutex<()> = Mutex::new(());

const SCHEMA_SQL: &str = r#"
CREATE TABLE airtable_tables (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  table_id TEXT NOT NULL UNIQUE
);
"#;

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_valid_fixture(dir: &tempfile::TempDir, api_url: &str) -> std::path::PathBuf {
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
api_url = "{api_url}"
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

#[tokio::test]
async fn test_succeeds_against_mock_api() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/appTEST/tblTEST"))
        .and(query_param("pageSize", "1"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "records": [{
                "id": "recAAA",
                "createdTime": "2024-01-01T00:00:00.000Z",
                "fields": { "Name": "Widget" }
            }]
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir, &server.uri());

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "airtable",
            "test",
        ])
        .unwrap();
}

#[tokio::test]
async fn test_fails_on_unauthorized_response() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/appTEST/tblTEST"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": { "type": "AUTHENTICATION_REQUIRED" }
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir, &server.uri());

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "airtable",
            "test",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Network);
}

#[tokio::test]
async fn test_uses_inline_token_without_env_var() {
    let _lock = test_lock();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/appTEST/tblTEST"))
        .and(query_param("pageSize", "1"))
        .and(header("authorization", "Bearer pat-inline-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "records": []
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let config_path = write_valid_fixture(&dir, &server.uri());
    let content = fs::read_to_string(&config_path).unwrap();
    let content = content.replace("token = \"pat-test\"", "token = \"pat-inline-token\"");
    fs::write(&config_path, content).unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "airtable",
            "test",
        ])
        .unwrap();
}
