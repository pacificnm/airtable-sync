//! `sync approve` and `sync approve-all` command handlers.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning};
use crate::db::OPERATION_STATUS_APPROVED;
use crate::sync::plan_context::{
    load_plan_command_context, open_plan_store, resolve_operation_id, resolve_plan_id,
};

/// JSON response for `sync approve` with `--json`.
#[derive(Debug, Serialize)]
pub struct SyncApproveResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Change plan id.
    pub plan_id: i64,
    /// Approved operation id, when approving one operation.
    pub operation_id: Option<i64>,
    /// Number of operations approved.
    pub approved: usize,
}

/// Approves one pending operation in the active change plan.
pub fn sync_approve(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in &validated.warnings {
            print_warning(warning);
        }
    }

    let command_ctx = load_plan_command_context(ctx)?;
    let store = open_plan_store(&command_ctx.database_path)?;
    let plan_id = resolve_plan_id(&store, &command_ctx.base_id, matches)?;
    let operation_id = resolve_operation_id(&store, plan_id, matches)?;

    store
        .set_operation_status(operation_id, OPERATION_STATUS_APPROVED)
        .map_err(NestError::from)?;

    let result = SyncApproveResult {
        database_path: command_ctx.database_path,
        base_id: command_ctx.base_id,
        plan_id,
        operation_id: Some(operation_id),
        approved: 1,
    };

    print_sync_approve_success(&result, json, quiet)
}

/// Approves all pending operations in the active change plan.
pub fn sync_approve_all(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in &validated.warnings {
            print_warning(warning);
        }
    }

    let command_ctx = load_plan_command_context(ctx)?;
    let store = open_plan_store(&command_ctx.database_path)?;
    let plan_id = resolve_plan_id(&store, &command_ctx.base_id, matches)?;
    let approved = store
        .set_pending_status_for_plan(plan_id, OPERATION_STATUS_APPROVED)
        .map_err(NestError::from)?;

    let result = SyncApproveResult {
        database_path: command_ctx.database_path,
        base_id: command_ctx.base_id,
        plan_id,
        operation_id: None,
        approved,
    };

    print_sync_approve_success(&result, json, quiet)
}

fn print_sync_approve_success(
    result: &SyncApproveResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize sync approve result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if let Some(operation_id) = result.operation_id {
        println!(
            "Approved operation {} in plan {}.",
            operation_id, result.plan_id
        );
    } else {
        println!(
            "Approved {} pending operation(s) in plan {}.",
            result.approved, result.plan_id
        );
    }

    Ok(())
}
