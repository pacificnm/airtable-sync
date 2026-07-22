//! `sync dry-run` command handler.

use std::path::PathBuf;

use nest_airtable::{AirtableClient, AirtableModule};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_http_client::HttpClientModule;
use serde::Serialize;

use crate::airtable::{block_on_async, to_airtable_config};
use crate::compare::{
    compare_single_table, merge_compare_summaries, CompareSummary, CompareTableResult,
};
use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{
    absolute_path, ensure_change_plan_schema, ensure_schema_cache, open_database, ChangePlanStore,
    SchemaStore,
};
use crate::sync::plan::{
    build_table_plan, count_field_changes, merge_plan_summaries, SyncTablePlan, SyncTablePlanSummary,
};

/// Rollup summary across planned tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncDryRunSummary {
    /// Tables planned successfully.
    pub tables_planned: usize,
    /// Tables that failed during compare/plan.
    pub tables_failed: usize,
    /// Total planned update operations.
    pub updates_total: usize,
    /// Total field changes across all operations.
    pub fields_changed: usize,
    /// Compare totals for context.
    pub compare_totals: CompareSummary,
    /// Planning totals for context.
    pub plan_totals: SyncTablePlanSummary,
}

/// One table sync dry-run failure when `continue_on_error` is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncDryRunFailure {
    /// Logical table name from config.
    pub table: String,
    /// Error message.
    pub message: String,
}

/// Per-table sync dry-run result.
#[derive(Debug, Serialize)]
pub struct SyncDryRunTableResult {
    /// Compare result for this table.
    pub compare: CompareTableResult,
    /// Planned operations for this table.
    pub plan: SyncTablePlan,
}

/// JSON response for `sync dry-run` with `--json`.
#[derive(Debug, Serialize)]
pub struct SyncDryRunResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Persisted change plan id, when `create_change_plan` is enabled.
    pub plan_id: Option<i64>,
    /// Rollup summary across tables.
    pub summary: SyncDryRunSummary,
    /// Per-table compare and plan results ordered by table name.
    pub tables: Vec<SyncDryRunTableResult>,
    /// Per-table failures when continuing after errors.
    pub failures: Vec<SyncDryRunFailure>,
}

/// Generates update-only change plans for all sync-enabled tables.
pub fn sync_dry_run(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in &validated.warnings {
            print_warning(warning);
        }
    }

    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);
    ensure_schema_cache(&database_path)?;

    let create_change_plan = validated.app.sync.create_change_plan;
    if create_change_plan {
        ensure_change_plan_schema(&database_path)?;
    }

    let mut table_names: Vec<String> = validated
        .app
        .airtable
        .tables
        .iter()
        .filter(|(_, table)| table.sync)
        .map(|(name, _)| name.clone())
        .collect();
    table_names.sort();

    let db = open_database(&database_path)?;
    let store = SchemaStore::new(db);
    let files = FileService::new()?;
    let airtable_config = to_airtable_config(&validated.app)?;
    let continue_on_error = validated.app.sync.continue_on_error;
    let base_id = validated.app.airtable.base_id.clone();
    let database_path_abs = absolute_path(&database_path);

    let (table_results, failures) = if table_names.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        block_on_async(async move {
            let built = AppBuilder::new()
                .module(HttpClientModule::default())
                .module(AirtableModule::with_config(airtable_config))
                .build()?;
            let client = built.context.service::<AirtableClient>()?.clone();

            let mut table_results = Vec::with_capacity(table_names.len());
            let mut failures = Vec::new();

            for table_name in table_names {
                match compare_single_table(
                    &validated,
                    &store,
                    &files,
                    &client,
                    &table_name,
                    quiet,
                )
                .await
                {
                    Ok(compare_result) => {
                        let allow_update = store
                            .find_table_by_name(&table_name)
                            .map_err(NestError::from)?
                            .map(|table| table.allow_update)
                            .unwrap_or(true);
                        let plan = build_table_plan(&compare_result, allow_update);
                        if !quiet {
                            for warning in &plan.warnings {
                                println!("warning: {warning}");
                            }
                        }
                        table_results.push(SyncDryRunTableResult { compare: compare_result, plan });
                    }
                    Err(error) => {
                        if continue_on_error {
                            failures.push(SyncDryRunFailure {
                                table: table_name,
                                message: error.message().to_string(),
                            });
                        } else {
                            return Err(error);
                        }
                    }
                }
            }

            Ok((table_results, failures))
        })?
    };

    let mut compare_totals = CompareSummary::default();
    let mut plan_totals = SyncTablePlanSummary::default();
    let mut all_operations = Vec::new();

    for table in &table_results {
        compare_totals = merge_compare_summaries(
            &compare_totals,
            &table.compare.compare.summary,
        );
        plan_totals = merge_plan_summaries(&plan_totals, &table.plan.summary);
        all_operations.extend(table.plan.operations.clone());
    }

    let plan_id = if create_change_plan && !all_operations.is_empty() {
        let plan_db = open_database(&database_path)?;
        let plan_store = ChangePlanStore::new(plan_db);
        plan_store
            .supersede_draft_plans(&base_id)
            .map_err(NestError::from)?;
        Some(
            plan_store
                .insert_plan(&base_id, table_results.len(), &all_operations)
                .map_err(NestError::from)?,
        )
    } else if create_change_plan && table_results.is_empty() {
        None
    } else {
        None
    };

    let result = SyncDryRunResult {
        database_path: database_path_abs,
        base_id,
        plan_id,
        summary: SyncDryRunSummary {
            tables_planned: table_results.len(),
            tables_failed: failures.len(),
            updates_total: all_operations.len(),
            fields_changed: count_field_changes(&all_operations),
            compare_totals,
            plan_totals,
        },
        tables: table_results,
        failures,
    };

    print_sync_dry_run_success(&result, json, quiet)
}

fn print_sync_dry_run_success(
    result: &SyncDryRunResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize sync dry-run result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.tables.is_empty() && result.failures.is_empty() {
        println!(
            "No sync-enabled tables configured — add `sync = true` under [airtable.tables.<name>]."
        );
        return Ok(());
    }

    let plan_suffix = result
        .plan_id
        .map(|plan_id| format!(" [plan id={plan_id}]"))
        .unwrap_or_default();

    println!(
        "Sync dry-run for base {} ({}):",
        result.base_id,
        result.database_path.display()
    );
    println!(
        "Summary: {} table(s), {} update(s), {} field change(s){}",
        result.summary.tables_planned,
        result.summary.updates_total,
        result.summary.fields_changed,
        plan_suffix
    );

    for table in &result.tables {
        println!(
            "  `{}` ({}) — {} update(s)",
            table.compare.table.name,
            table.compare.table.table_id,
            table.plan.summary.updates
        );
    }

    if !result.failures.is_empty() {
        println!();
        println!("Failures ({}):", result.failures.len());
        for failure in &result.failures {
            println!("  `{}`: {}", failure.table, failure.message);
        }
    }

    Ok(())
}
