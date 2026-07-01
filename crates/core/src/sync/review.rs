//! `sync review` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning};
use crate::db::{ChangePlanHeader, ChangePlanOperationView, ChangePlanStatusCounts};
use crate::sync::plan_context::{load_plan_command_context, open_plan_store, resolve_plan_id};

/// JSON response for `sync review` with `--json`.
#[derive(Debug, Serialize)]
pub struct SyncReviewResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Reviewed change plan.
    pub plan: ChangePlanHeader,
    /// Operations in the plan.
    pub operations: Vec<ChangePlanOperationView>,
    /// Operation counts by review status.
    pub summary: ChangePlanStatusCounts,
}

/// Reviews the active or selected change plan.
pub fn sync_review(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
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

    let detail = store
        .load_plan(plan_id)
        .map_err(NestError::from)?
        .ok_or_else(|| NestError::data(format!("change plan `{plan_id}` not found")))?;

    let result = SyncReviewResult {
        database_path: command_ctx.database_path,
        base_id: command_ctx.base_id,
        summary: detail.plan.status_counts.clone(),
        plan: detail.plan,
        operations: detail.operations,
    };

    print_sync_review_success(&result, json, quiet)
}

fn print_sync_review_success(
    result: &SyncReviewResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize sync review result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    let counts = &result.summary;
    println!(
        "Change plan {} ({}) for base {}:",
        result.plan.id,
        result.plan.status,
        result.base_id
    );
    println!(
        "Summary: {} pending, {} approved, {} denied",
        counts.pending, counts.approved, counts.denied
    );

    if result.operations.is_empty() {
        println!("No operations in this plan.");
        return Ok(());
    }

    println!();
    for operation in &result.operations {
        let record_id = operation
            .airtable_record_id
            .as_deref()
            .unwrap_or("-");
        println!(
            "  [{}] {} `{}` key `{}` ({}) {}",
            operation.operation_id,
            operation.operation,
            operation.table_name,
            operation.record_key,
            record_id,
            operation.status
        );
        for field_change in &operation.field_changes {
            println!(
                "    {}: {:?} -> {:?}",
                field_change.field_name, field_change.old_value, field_change.new_value
            );
        }
    }

    Ok(())
}
