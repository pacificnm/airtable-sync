//! `compare table` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_airtable::{AirtableClient, AirtableModule};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_http_client::HttpClientModule;
use serde::Serialize;

use crate::airtable::{block_on_async, to_airtable_config};
use crate::compare::diff::CompareDiffResult;
use crate::compare::engine::compare_single_table;
use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{ensure_schema_cache, open_database, SchemaStore};

/// Cached table metadata in compare output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Whether sync is enabled for this table.
    pub enabled: bool,
}

/// JSON response for `compare table` with `--json`.
#[derive(Debug, Serialize)]
pub struct CompareTableResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Compared table metadata.
    pub table: CompareTableView,
    /// Airtable primary key field name.
    pub primary_key_field: String,
    /// Normalized CSV column used as the primary key.
    pub primary_key_csv_column: String,
    /// Source CSV file basename.
    pub csv_file: String,
    /// Absolute path to the source CSV file.
    pub csv_path: PathBuf,
    /// Compared fields (sync enabled, mapped, same CSV file).
    pub compared_fields: Vec<String>,
    /// Compare summary and diffs.
    pub compare: CompareDiffResult,
}

/// Compares one configured table's CSV rows against live Airtable records.
pub fn compare_table(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let table_name = matches
        .get_one::<String>("table")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing table name"))?;

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

    let db = open_database(&database_path)?;
    let store = SchemaStore::new(db);
    let files = FileService::new()?;
    let airtable_config = to_airtable_config(&validated.app)?;
    let table_name = table_name.to_string();

    let result = block_on_async(async move {
        let built = AppBuilder::new()
            .module(HttpClientModule::default())
            .module(AirtableModule::with_config(airtable_config))
            .build()?;
        let client = built.context.service::<AirtableClient>()?.clone();
        compare_single_table(
            &validated,
            &store,
            &files,
            &client,
            &table_name,
            quiet,
        )
        .await
    })?;

    print_compare_table_success(&result, json, quiet)
}

pub(crate) fn print_compare_table_success(
    result: &CompareTableResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize compare table result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    print_compare_summary_human(result);
    Ok(())
}

pub(crate) fn print_compare_table_brief(result: &CompareTableResult) {
    let summary = &result.compare.summary;
    println!(
        "  `{}` ({}) — {} matched, {} differing, {} CSV-only, {} Airtable-only",
        result.table.name,
        result.table.table_id,
        summary.matched,
        summary.differing,
        summary.csv_only,
        summary.airtable_only
    );
}

fn print_compare_summary_human(result: &CompareTableResult) {
    let summary = &result.compare.summary;
    println!(
        "Compare `{}` ({}) using primary key `{}` from {}:",
        result.table.name,
        result.table.table_id,
        result.primary_key_field,
        result.csv_file
    );
    println!(
        "Summary: {} CSV row(s), {} Airtable row(s), {} matched, {} differing, {} CSV-only, {} Airtable-only",
        summary.csv_rows,
        summary.airtable_rows,
        summary.matched,
        summary.differing,
        summary.csv_only,
        summary.airtable_only
    );

    if result.compared_fields.is_empty() {
        println!("Compared fields: none (primary key only)");
    } else {
        println!("Compared fields: {}", result.compared_fields.join(", "));
    }

    if !result.compare.differing_records.is_empty() {
        println!();
        println!("Differing records:");
        for record in &result.compare.differing_records {
            println!("  key `{}`:", record.key);
            for diff in &record.differences {
                println!(
                    "    - {} (CSV `{}`): {:?} != {:?}",
                    diff.field_name, diff.csv_column, diff.csv_value, diff.airtable_value
                );
            }
        }
    }

    if !result.compare.csv_only_keys.is_empty() {
        println!();
        println!(
            "CSV-only keys ({}): {}",
            result.compare.csv_only_keys.len(),
            result.compare.csv_only_keys.join(", ")
        );
    }

    if !result.compare.airtable_only_keys.is_empty() {
        println!();
        println!(
            "Airtable-only keys ({}): {}",
            result.compare.airtable_only_keys.len(),
            result.compare.airtable_only_keys.join(", ")
        );
    }
}
