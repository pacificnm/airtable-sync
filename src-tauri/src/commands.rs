//! Airtable Sync IPC commands.
//!
//! The desktop UI drives the same command tree as the CLI. Every action is a
//! subcommand dispatched in-process through [`CommandDispatch`] (no business
//! logic is duplicated here). Commands are invoked from the UI as
//! `plugin:airtable-sync|<command>`.

use airtable_sync_core::{CommandDispatch, DispatchResult, COMMAND_GROUPS};
use serde::Serialize;
use tauri::State;

/// Serializable outcome of a dispatched command.
#[derive(Debug, Serialize)]
pub struct RunResult {
    /// Whether the command returned `Ok(())`.
    pub success: bool,
    /// Captured standard output (Unix only; empty on other platforms).
    pub stdout: String,
    /// Human-readable error message when the command failed.
    pub error: Option<String>,
}

impl From<DispatchResult> for RunResult {
    fn from(result: DispatchResult) -> Self {
        Self {
            success: result.success,
            stdout: result.stdout,
            error: result.error.map(|e| e.to_string()),
        }
    }
}

/// A subcommand exposed to the UI.
#[derive(Debug, Serialize)]
pub struct SubcommandInfo {
    /// Subcommand name (kebab-case).
    pub name: String,
    /// Short help text.
    pub about: String,
}

/// A top-level command group and its subcommands.
#[derive(Debug, Serialize)]
pub struct GroupInfo {
    /// Group name (kebab-case).
    pub name: String,
    /// Short help text for the group.
    pub about: String,
    /// Nested subcommands (empty for leaf commands such as `version`).
    pub subcommands: Vec<SubcommandInfo>,
}

/// Builds the `airtable_sync` Tauri plugin carrying all IPC commands.
pub fn airtable_sync_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("airtable-sync")
        .invoke_handler(tauri::generate_handler![
            airtable_sync_run,
            airtable_sync_command_groups,
        ])
        .build()
}

/// Runs an `airtable-sync` subcommand (e.g. `["sync", "dry-run"]`).
///
/// When `json` is true the CLI emits machine-readable output. Runs on a
/// blocking thread so the CLI can drive its own async runtime without
/// conflicting with Tauri's.
#[tauri::command]
async fn airtable_sync_run(
    dispatch: State<'_, CommandDispatch>,
    args: Vec<String>,
    json: bool,
) -> Result<RunResult, String> {
    let dispatch = dispatch.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        dispatch.run(&arg_refs, json)
    })
    .await
    .map_err(|e| format!("dispatch task failed: {e}"))?;

    Ok(RunResult::from(result))
}

/// Returns the full command tree so the UI can build its ribbon dynamically.
#[tauri::command]
async fn airtable_sync_command_groups() -> Result<Vec<GroupInfo>, String> {
    let groups = COMMAND_GROUPS
        .iter()
        .map(|group| GroupInfo {
            name: group.name.to_string(),
            about: group.about.to_string(),
            subcommands: group
                .subcommands
                .iter()
                .map(|sub| SubcommandInfo {
                    name: sub.name.to_string(),
                    about: sub.about.to_string(),
                })
                .collect(),
        })
        .collect();
    Ok(groups)
}
