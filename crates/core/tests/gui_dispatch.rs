//! GUI dispatch integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::CommandDispatch;
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
        "id,name\n1,Alice\n2,Bob\n",
    )
    .unwrap();
    fs::write(dir.path().join("space.csv"), "id,name\n1,Room A\n").unwrap();
    fs::write(dir.path().join("schema.sql"), SCHEMA_SQL).unwrap();

    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[airtable]
api_url = "http://example.invalid"
meta_api_url = "http://example.invalid/meta"
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
"#,
    )
    .unwrap();

    config_path
}

#[test]
fn dispatch_report_summary_json_captures_stdout() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = write_fixture(&dir);
    let dispatch = CommandDispatch::new(Some(config_path));

    let result = dispatch.run(&["report", "summary"], true);

    assert!(result.success, "expected success: {:?}", result.error);
    assert!(
        result.stdout.contains("\"sync\"") || result.stdout.contains("sync"),
        "expected JSON summary in stdout, got: {}",
        result.stdout
    );
}
