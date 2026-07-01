//! Shared helpers for sync change-plan commands.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::config::{ensure_valid_config, resolve_config_path};
use crate::db::{
    ensure_change_plan_schema, open_database, ChangePlanStore, PLAN_STATUS_DRAFT,
};

/// Resolved plan context for review/approval commands.
pub(crate) struct PlanCommandContext {
    /// Absolute database path.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
}

/// Loads config and validates change-plan prerequisites.
pub(crate) fn load_plan_command_context(ctx: &AppContext) -> NestResult<PlanCommandContext> {
    let validated = ensure_valid_config(ctx)?;
    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);

    if validated.app.sync.create_change_plan {
        ensure_change_plan_schema(&database_path)?;
    } else {
        return Err(NestError::validation(
            "change plan review requires `[sync].create_change_plan = true`",
        )
        .with_help("Enable `create_change_plan` in config.toml and run `sync dry-run`."));
    }

    Ok(PlanCommandContext {
        database_path: crate::db::absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
    })
}

/// Resolves a plan id from `--plan-id` or the latest draft for the base.
pub(crate) fn resolve_plan_id(
    store: &ChangePlanStore,
    base_id: &str,
    matches: &ArgMatches,
) -> NestResult<i64> {
    if let Some(plan_id) = matches.get_one::<i64>("plan-id") {
        let header = store
            .load_plan_header(*plan_id)
            .map_err(NestError::from)?
            .ok_or_else(|| NestError::data(format!("change plan `{plan_id}` not found")))?;
        if header.base_id != base_id {
            return Err(NestError::validation(format!(
                "change plan `{plan_id}` belongs to base `{}`, not `{base_id}`",
                header.base_id
            )));
        }
        return Ok(*plan_id);
    }

    let latest = store
        .find_latest_draft_plan(base_id)
        .map_err(NestError::from)?
        .ok_or_else(|| {
            NestError::data("no draft change plan found")
                .with_help("Run `sync dry-run` to generate a change plan first.")
        })?;

    if latest.status != PLAN_STATUS_DRAFT {
        return Err(NestError::data(format!(
            "latest plan `{}` is not a draft",
            latest.id
        )));
    }

    Ok(latest.id)
}

/// Resolves a plan id for reporting: latest draft, else most recent plan.
pub(crate) fn resolve_plan_id_for_report(
    store: &ChangePlanStore,
    base_id: &str,
    matches: &ArgMatches,
) -> NestResult<i64> {
    if let Some(plan_id) = matches.get_one::<i64>("plan-id") {
        let header = store
            .load_plan_header(*plan_id)
            .map_err(NestError::from)?
            .ok_or_else(|| NestError::data(format!("change plan `{plan_id}` not found")))?;
        if header.base_id != base_id {
            return Err(NestError::validation(format!(
                "change plan `{plan_id}` belongs to base `{}`, not `{base_id}`",
                header.base_id
            )));
        }
        return Ok(*plan_id);
    }

    if let Some(latest) = store
        .find_latest_draft_plan(base_id)
        .map_err(NestError::from)?
    {
        return Ok(latest.id);
    }

    let latest = store
        .find_latest_plan(base_id)
        .map_err(NestError::from)?
        .ok_or_else(|| {
            NestError::data("no change plan found")
                .with_help("Run `sync dry-run` to generate a change plan first.")
        })?;

    Ok(latest.id)
}

/// Resolves one operation id from a positional arg or `--table` + `--key`.
pub(crate) fn resolve_operation_id(
    store: &ChangePlanStore,
    plan_id: i64,
    matches: &ArgMatches,
) -> NestResult<i64> {
    if let Some(operation_id) = matches.get_one::<i64>("operation_id") {
        let operation = store
            .find_operation(plan_id, *operation_id)
            .map_err(NestError::from)?
            .ok_or_else(|| {
                NestError::data(format!(
                    "operation `{operation_id}` not found in plan `{plan_id}`"
                ))
            })?;
        return Ok(operation.operation_id);
    }

    let table_name = matches
        .get_one::<String>("table")
        .map(String::as_str)
        .ok_or_else(|| {
            NestError::command("missing operation id — pass <operation_id> or --table and --key")
        })?;
    let record_key = matches
        .get_one::<String>("key")
        .map(String::as_str)
        .ok_or_else(|| {
            NestError::command("missing --key when approving or denying by table name")
        })?;

    let operation = store
        .find_operation_by_key(plan_id, table_name, record_key)
        .map_err(NestError::from)?
        .ok_or_else(|| {
            NestError::data(format!(
                "no operation found for table `{table_name}` with key `{record_key}` in plan `{plan_id}`"
            ))
        })?;

    Ok(operation.operation_id)
}
/// Opens a change plan store for the configured database path.
pub(crate) fn open_plan_store(database_path: &std::path::Path) -> NestResult<ChangePlanStore> {
    let db = open_database(database_path)?;
    Ok(ChangePlanStore::new(db))
}
