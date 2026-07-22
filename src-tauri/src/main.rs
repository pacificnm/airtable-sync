#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::path::PathBuf;

use airtable_sync_core::CommandDispatch;
use nest_logging::LoggingConfig;
use nest_tauri::TauriApp;
use nest_theme::ThemeModule;

/// Locates `config.toml` (Airtable credentials, table config, DB path).
///
/// `tauri dev` runs with the working directory at `src-tauri/`, while the
/// config lives one level up in `apps/airtable-sync/`. Returns `None` to fall
/// back to Nest's default search when neither candidate exists.
fn resolve_config_path() -> Option<PathBuf> {
    for candidate in ["config.toml", "../config.toml"] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Chooses a writable log directory for the current run.
///
/// Under `tauri dev` the working directory is `src-tauri/` (where
/// `tauri.conf.json` lives), so logs go to `../logs`. A shipped binary can be
/// launched from anywhere, where `../logs` may be unwritable; there we use the
/// platform data dir, falling back to the temp dir.
fn resolve_log_dir() -> PathBuf {
    if PathBuf::from("tauri.conf.json").exists() {
        return PathBuf::from("../logs");
    }
    dirs::data_dir()
        .map(|dir| dir.join("airtable-sync").join("logs"))
        .unwrap_or_else(|| std::env::temp_dir().join("airtable-sync").join("logs"))
}

fn main() {
    let config_path = resolve_config_path();
    let dispatch = CommandDispatch::new(config_path.clone());

    let mut app = TauriApp::new("airtable-sync")
        .with_logging(LoggingConfig::for_tauri("airtable-sync").with_file(resolve_log_dir()))
        .module(ThemeModule::default());

    if let Some(path) = config_path {
        app = app.with_config_path(path);
    }

    app.with_builder(move |builder| {
        builder
            .manage(dispatch)
            .plugin(commands::airtable_sync_plugin())
    })
    .run(tauri::generate_context!());
}
