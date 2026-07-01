//! `sync apply` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_airtable::{AirtableClient, AirtableFields, AirtableModule};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_http_client::HttpClientModule;
use serde::Serialize;
use serde_json::Value;

use crate::airtable::{block_on_async, to_airtable_config};
use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{
    open_database, ChangePlanFieldChange, ChangePlanOperationView, PLAN_STATUS_APPLIED,
    SchemaStore,
};
use crate::sync::plan_context::{
    load_plan_command_context, open_plan_store, resolve_plan_id,
};

/// One operation skipped before calling Airtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncApplySkipped {
    /// Operation row id.
    pub operation_id: i64,
    /// Logical table name.
    pub table: String,
    /// Primary key value.
    pub key: String,
    /// Why the operation was skipped.
    pub reason: String,
}

/// One operation that failed during apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncApplyFailure {
    /// Operation row id.
    pub operation_id: i64,
    /// Logical table name.
    pub table: String,
    /// Primary key value.
    pub key: String,
    /// Error message.
    pub message: String,
}

/// Rollup counts for `sync apply`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct SyncApplySummary {
    /// Operations successfully pushed to Airtable.
    pub applied: usize,
    /// Operations that failed during apply.
    pub failed: usize,
    /// Operations skipped before calling Airtable.
    pub skipped: usize,
}

/// JSON response for `sync apply` with `--json`.
#[derive(Debug, Serialize)]
pub struct SyncApplyResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Change plan id.
    pub plan_id: i64,
    /// Rollup counts.
    pub summary: SyncApplySummary,
    /// Skipped operations.
    pub skipped: Vec<SyncApplySkipped>,
    /// Failed operations when continuing after errors.
    pub failures: Vec<SyncApplyFailure>,
    /// Pending operations still awaiting review.
    pub pending_remaining: usize,
    /// Whether the plan was marked applied.
    pub plan_applied: bool,
}

struct PendingUpdate {
    operation_id: i64,
    table_name: String,
    record_key: String,
    record_id: String,
    fields: AirtableFields,
}

enum ApplyOutcome {
    Applied(i64),
    Failed {
        operation_id: i64,
        table: String,
        key: String,
        message: String,
    },
}

/// Applies approved update operations from the active change plan to Airtable.
pub fn sync_apply(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    if validated.app.sync.dry_run {
        return Err(NestError::validation(
            "`sync apply` requires `[sync].dry_run = false`",
        )
        .with_help("Set `dry_run = false` in config.toml before applying changes to Airtable."));
    }

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
    let operations = store
        .load_approved_operations(plan_id)
        .map_err(NestError::from)?;

    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);
    let schema_db = open_database(&database_path)?;
    let schema_store = SchemaStore::new(schema_db);
    let continue_on_error = validated.app.sync.continue_on_error;

    let mut summary = SyncApplySummary::default();
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    if operations.is_empty() {
        let counts = store
            .count_operations_by_status(plan_id)
            .map_err(NestError::from)?;
        let result = SyncApplyResult {
            database_path: command_ctx.database_path,
            base_id: command_ctx.base_id,
            plan_id,
            summary,
            skipped,
            failures,
            pending_remaining: counts.pending,
            plan_applied: false,
        };
        return print_sync_apply_success(&result, json, quiet);
    }

    let mut pending_updates = Vec::new();
    for operation in operations {
        match classify_operation(&schema_store, &operation) {
            Ok(update) => pending_updates.push(update),
            Err(reason) => {
                summary.skipped += 1;
                skipped.push(SyncApplySkipped {
                    operation_id: operation.operation_id,
                    table: operation.table_name.clone(),
                    key: operation.record_key.clone(),
                    reason,
                });
            }
        }
    }

    if !pending_updates.is_empty() {
        let airtable_config = to_airtable_config(&validated.app)?;
        let outcomes = block_on_async(async move {
            let built = AppBuilder::new()
                .module(HttpClientModule::default())
                .module(AirtableModule::with_config(airtable_config))
                .build()?;
            let client = built.context.service::<AirtableClient>()?.clone();

            let mut outcomes = Vec::new();
            for update in pending_updates {
                match client
                    .update_record(&update.table_name, &update.record_id, update.fields)
                    .await
                {
                    Ok(_) => outcomes.push(ApplyOutcome::Applied(update.operation_id)),
                    Err(error) => {
                        outcomes.push(ApplyOutcome::Failed {
                            operation_id: update.operation_id,
                            table: update.table_name,
                            key: update.record_key,
                            message: error.message().to_string(),
                        });
                        if !continue_on_error {
                            break;
                        }
                    }
                }
            }
            Ok(outcomes)
        })?;

        for outcome in outcomes {
            match outcome {
                ApplyOutcome::Applied(operation_id) => {
                    store
                        .mark_operation_applied(operation_id)
                        .map_err(NestError::from)?;
                    summary.applied += 1;
                }
                ApplyOutcome::Failed {
                    operation_id,
                    table,
                    key,
                    message,
                } => {
                    store
                        .mark_operation_failed(operation_id)
                        .map_err(NestError::from)?;
                    summary.failed += 1;
                    failures.push(SyncApplyFailure {
                        operation_id,
                        table,
                        key,
                        message,
                    });
                }
            }
        }
    }

    let counts = store
        .count_operations_by_status(plan_id)
        .map_err(NestError::from)?;
    let plan_applied = counts.pending == 0 && counts.approved == 0;
    if plan_applied {
        store
            .set_plan_status(plan_id, PLAN_STATUS_APPLIED)
            .map_err(NestError::from)?;
    }

    let failed_count = summary.failed;
    let result = SyncApplyResult {
        database_path: command_ctx.database_path,
        base_id: command_ctx.base_id,
        plan_id,
        summary,
        skipped,
        failures,
        pending_remaining: counts.pending,
        plan_applied,
    };

    if failed_count > 0 && !continue_on_error {
        print_sync_apply_success(&result, json, quiet)?;
        return Err(NestError::data(format!(
            "apply stopped after {failed_count} failed operation(s)"
        )));
    }

    print_sync_apply_success(&result, json, quiet)
}

fn classify_operation(
    schema_store: &SchemaStore,
    operation: &ChangePlanOperationView,
) -> Result<PendingUpdate, String> {
    if operation.operation != "update" {
        return Err(format!(
            "unsupported operation `{}` (only `update` is supported)",
            operation.operation
        ));
    }

    let allow_update = schema_store
        .find_table_by_name(&operation.table_name)
        .map_err(|error| error.to_string())?
        .map(|table| table.allow_update)
        .unwrap_or(true);
    if !allow_update {
        return Err(format!(
            "updates are disabled for table `{}`",
            operation.table_name
        ));
    }

    let Some(record_id) = operation.airtable_record_id.clone() else {
        return Err("missing Airtable record id".to_string());
    };

    Ok(PendingUpdate {
        operation_id: operation.operation_id,
        table_name: operation.table_name.clone(),
        record_key: operation.record_key.clone(),
        record_id,
        fields: fields_from_changes(&operation.field_changes),
    })
}

fn fields_from_changes(changes: &[ChangePlanFieldChange]) -> AirtableFields {
    let mut fields = AirtableFields::new();
    for change in changes {
        fields.insert(
            change.field_name.clone(),
            Value::String(change.new_value.clone()),
        );
    }
    fields
}

fn print_sync_apply_success(
    result: &SyncApplyResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize sync apply result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.summary.applied == 0 && result.summary.failed == 0 && result.summary.skipped == 0 {
        println!("No approved operations to apply in plan {}.", result.plan_id);
    } else {
        println!(
            "Applied {} operation(s) from plan {} ({} failed, {} skipped).",
            result.summary.applied,
            result.plan_id,
            result.summary.failed,
            result.summary.skipped,
        );
    }

    if result.pending_remaining > 0 {
        println!(
            "warning: {} pending operation(s) remain — approve them and run `sync apply` again.",
            result.pending_remaining
        );
    }

    if result.plan_applied {
        println!("Plan {} marked applied.", result.plan_id);
    }

    for failure in &result.failures {
        println!(
            "error: operation {} ({} key {}): {}",
            failure.operation_id, failure.table, failure.key, failure.message
        );
    }

    for skip in &result.skipped {
        println!(
            "skipped: operation {} ({} key {}): {}",
            skip.operation_id, skip.table, skip.key, skip.reason
        );
    }

    Ok(())
}
