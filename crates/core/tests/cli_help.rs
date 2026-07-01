//! CLI help integration tests.

use std::sync::Mutex;

use airtable_sync_core::{cli_app, cli_help_text, group_help_text, COMMAND_GROUPS};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn help_lists_all_top_level_groups() {
    let _lock = test_lock();
    let help = cli_help_text().unwrap();

    assert!(help.contains("Airtable Sync"));
    assert!(help.contains("Synchronize Airtable tables from LDP CSV exports"));

    for group in COMMAND_GROUPS {
        assert!(
            help.contains(group.name),
            "root help missing command group: {}",
            group.name
        );
        assert!(
            help.contains(group.about),
            "root help missing description for: {}",
            group.name
        );
    }
}

#[test]
fn group_help_lists_nested_subcommands() {
    let _lock = test_lock();

    for group in COMMAND_GROUPS {
        if group.subcommands.is_empty() {
            continue;
        }

        let help = group_help_text(group.name).unwrap();
        for sub in group.subcommands {
            assert!(
                help.contains(sub.name),
                "help for `{}` missing subcommand: {}",
                group.name,
                sub.name
            );
            assert!(
                help.contains(sub.about),
                "help for `{}` missing description for: {}",
                group.name,
                sub.name
            );
        }
    }
}

#[test]
fn help_flag_returns_ok() {
    let _lock = test_lock();
    cli_app().try_run_with(["airtable-sync", "--help"]).unwrap();
}

#[test]
fn nested_help_flag_returns_ok() {
    let _lock = test_lock();
    cli_app()
        .try_run_with(["airtable-sync", "config", "--help"])
        .unwrap();
}

#[test]
fn version_command_prints_configured_version() {
    let _lock = test_lock();
    cli_app()
        .try_run_with(["airtable-sync", "version"])
        .unwrap();
}

#[test]
fn version_flag_prints_configured_version() {
    let _lock = test_lock();
    cli_app()
        .try_run_with(["airtable-sync", "--version"])
        .unwrap();
}

#[test]
fn setup_init_stub_runs() {
    let _lock = test_lock();
    cli_app()
        .try_run_with(["airtable-sync", "setup", "init"])
        .unwrap();
}
