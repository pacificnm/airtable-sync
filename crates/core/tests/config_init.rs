//! `config init` integration tests.

use std::fs;
use std::sync::Mutex;

use airtable_sync_core::cli_app;
use tempfile::tempdir;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn init_creates_config_toml() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "init",
        ])
        .unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[airtable]"));
    assert!(content.contains("[sync]"));
}

#[test]
fn init_fails_when_file_exists() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, "[airtable]\nbase_id = \"appEXISTING\"\n").unwrap();

    let error = cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "init",
        ])
        .unwrap_err();

    assert_eq!(error.kind(), nest_error::NestErrorKind::Config);
    assert!(error.message().contains("already exists"));
}

#[test]
fn init_force_overwrites() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, "stale = true\n").unwrap();

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "init",
            "--force",
        ])
        .unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[airtable.tables.business_unit]"));
    assert!(!content.contains("stale = true"));
}

#[test]
fn init_respects_output_flag() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let result = cli_app().try_run_with(["airtable-sync", "config", "init", "--output", "custom.toml"]);

    std::env::set_current_dir(original).unwrap();
    result.unwrap();

    let content = fs::read_to_string(dir.path().join("custom.toml")).unwrap();
    assert!(content.contains("[csv]"));
}

#[test]
fn init_json_output_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "config",
            "init",
        ])
        .unwrap();
}

#[test]
fn init_quiet_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "--quiet",
            "config",
            "init",
        ])
        .unwrap();

    assert!(config_path.is_file());
}

#[test]
fn init_with_missing_explicit_config_path_succeeds() {
    let _lock = test_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("nested").join("config.toml");
    assert!(!config_path.exists());

    cli_app()
        .try_run_with([
            "airtable-sync",
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "init",
        ])
        .unwrap();

    assert!(config_path.is_file());
}
