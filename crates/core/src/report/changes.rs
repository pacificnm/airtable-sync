//! `report changes` command handler.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning};
use crate::db::{ChangePlanHeader, ChangePlanOperationView, ChangePlanStatusCounts};
use crate::sync::plan_context::{
    load_plan_command_context, open_plan_store, resolve_plan_id_for_report,
};

/// Operations grouped under one logical table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeReportTableView {
    /// Logical table name from config.
    pub table_name: String,
    /// Operations for this table.
    pub operations: Vec<ChangePlanOperationView>,
}

/// JSON response for `report changes` with `--json`.
#[derive(Debug, Serialize)]
pub struct ChangeReportResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Reported change plan.
    pub plan: ChangePlanHeader,
    /// Operation counts by status.
    pub summary: ChangePlanStatusCounts,
    /// Operations grouped by table name.
    pub tables: Vec<ChangeReportTableView>,
}

/// Generates a change report from the active or selected change plan.
pub fn report_changes(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
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
    let plan_id = resolve_plan_id_for_report(&store, &command_ctx.base_id, matches)?;

    let detail = store
        .load_plan(plan_id)
        .map_err(NestError::from)?
        .ok_or_else(|| NestError::data(format!("change plan `{plan_id}` not found")))?;

    let status_filter = matches.get_one::<String>("status").map(String::as_str);
    let operations: Vec<ChangePlanOperationView> = detail
        .operations
        .into_iter()
        .filter(|operation| {
            status_filter.is_none_or(|status| operation.status == status)
        })
        .collect();

    let tables = group_operations_by_table(&operations);
    let result = ChangeReportResult {
        database_path: command_ctx.database_path,
        base_id: command_ctx.base_id,
        summary: detail.plan.status_counts.clone(),
        plan: detail.plan,
        tables,
    };

    print_change_report_success(&result, json, quiet)
}

fn group_operations_by_table(
    operations: &[ChangePlanOperationView],
) -> Vec<ChangeReportTableView> {
    let mut by_table: BTreeMap<String, Vec<ChangePlanOperationView>> = BTreeMap::new();
    for operation in operations {
        by_table
            .entry(operation.table_name.clone())
            .or_default()
            .push(operation.clone());
    }

    by_table
        .into_iter()
        .map(|(table_name, operations)| ChangeReportTableView {
            table_name,
            operations,
        })
        .collect()
}

fn print_change_report_success(
    result: &ChangeReportResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize change report result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    let counts = &result.summary;
    println!(
        "Change report for plan {} ({}, created {}) — base {}",
        result.plan.id,
        result.plan.status,
        result.plan.created_at,
        result.base_id,
    );
    println!(
        "Summary: {} pending, {} approved, {} denied, {} applied, {} failed",
        counts.pending,
        counts.approved,
        counts.denied,
        counts.applied,
        counts.failed,
    );

    if result.tables.is_empty() {
        println!("No operations match this report.");
        return Ok(());
    }

    for table in &result.tables {
        println!();
        println!("Table `{}` ({} operation(s))", table.table_name, table.operations.len());
        for operation in &table.operations {
            let record_id = operation
                .airtable_record_id
                .as_deref()
                .unwrap_or("-");
            println!(
                "  [{}] {} key `{}` ({}) — {}",
                operation.operation_id,
                operation.operation,
                operation.record_key,
                record_id,
                operation.status,
            );
            for field_change in &operation.field_changes {
                println!(
                    "    {}: {:?} -> {:?}",
                    field_change.field_name, field_change.old_value, field_change.new_value
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ChangePlanFieldChange;

    fn sample_operation(table: &str, key: &str, status: &str) -> ChangePlanOperationView {
        ChangePlanOperationView {
            operation_id: 1,
            table_name: table.to_string(),
            operation: "update".to_string(),
            record_key: key.to_string(),
            airtable_record_id: Some(format!("rec{key}")),
            status: status.to_string(),
            field_changes: vec![ChangePlanFieldChange {
                field_name: "Name".to_string(),
                old_value: "Old".to_string(),
                new_value: "New".to_string(),
            }],
        }
    }

    #[test]
    fn group_operations_by_table_sorts_tables_and_preserves_operations() {
        let grouped = group_operations_by_table(&[
            sample_operation("space", "2", "pending"),
            sample_operation("assets", "1", "approved"),
            sample_operation("assets", "3", "denied"),
        ]);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].table_name, "assets");
        assert_eq!(grouped[0].operations.len(), 2);
        assert_eq!(grouped[1].table_name, "space");
        assert_eq!(grouped[1].operations.len(), 1);
    }
}
