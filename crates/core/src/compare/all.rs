//! `compare all` command handler.

use std::path::PathBuf;

use nest_airtable::{AirtableClient, AirtableModule};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_http_client::HttpClientModule;
use serde::Serialize;

use crate::airtable::{block_on_async, to_airtable_config};
use crate::compare::diff::{merge_compare_summaries, CompareSummary};
use crate::compare::engine::compare_single_table;
use crate::compare::table::{print_compare_table_brief, CompareTableResult};
use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{absolute_path, ensure_schema_cache, open_database, SchemaStore};

/// Rollup summary across compared tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareAllSummary {
    /// Tables compared successfully.
    pub tables_compared: usize,
    /// Tables that failed during compare.
    pub tables_failed: usize,
    /// Totals of per-table compare summaries.
    pub totals: CompareSummary,
}

/// One table compare failure when `continue_on_error` is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareTableFailure {
    /// Logical table name from config.
    pub table: String,
    /// Error message.
    pub message: String,
}

/// JSON response for `compare all` with `--json`.
#[derive(Debug, Serialize)]
pub struct CompareAllResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Rollup summary across tables.
    pub summary: CompareAllSummary,
    /// Per-table compare results ordered by table name.
    pub tables: Vec<CompareTableResult>,
    /// Per-table failures when continuing after errors.
    pub failures: Vec<CompareTableFailure>,
}

/// Compares all sync-enabled configured tables against live Airtable records.
pub fn compare_all(ctx: &AppContext) -> NestResult<()> {
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

    let (tables, failures) = if table_names.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        block_on_async(async move {
            let built = AppBuilder::new()
                .module(HttpClientModule::default())
                .module(AirtableModule::with_config(airtable_config))
                .build()?;
            let client = built.context.service::<AirtableClient>()?.clone();

            let mut tables = Vec::with_capacity(table_names.len());
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
                    Ok(result) => tables.push(result),
                    Err(error) => {
                        if continue_on_error {
                            failures.push(CompareTableFailure {
                                table: table_name,
                                message: error.message().to_string(),
                            });
                        } else {
                            return Err(error);
                        }
                    }
                }
            }

            Ok((tables, failures))
        })?
    };

    let mut totals = CompareSummary::default();
    for table in &tables {
        totals = merge_compare_summaries(&totals, &table.compare.summary);
    }

    let result = CompareAllResult {
        database_path: database_path_abs,
        base_id,
        summary: CompareAllSummary {
            tables_compared: tables.len(),
            tables_failed: failures.len(),
            totals,
        },
        tables,
        failures,
    };

    print_compare_all_success(&result, json, quiet)
}

fn print_compare_all_success(
    result: &CompareAllResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize compare all result: {error}"))
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

    let totals = &result.summary.totals;
    println!(
        "Compare all for base {} ({}):",
        result.base_id,
        result.database_path.display()
    );
    println!(
        "Summary: {} table(s) compared, {} failed — {} matched, {} differing, {} CSV-only, {} Airtable-only",
        result.summary.tables_compared,
        result.summary.tables_failed,
        totals.matched,
        totals.differing,
        totals.csv_only,
        totals.airtable_only
    );

    for table in &result.tables {
        print_compare_table_brief(table);
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
