//! `airtable records` command handler — fetch every record for one table.

use clap::ArgMatches;
use nest_airtable::{AirtableClient, AirtableListParams, AirtableModule, AirtableRecord};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_http_client::HttpClientModule;
use serde::Serialize;
use std::collections::BTreeSet;

use crate::config::{ensure_valid_config, print_warning};

use super::bridge::to_airtable_config;
use super::runtime::block_on_async;

const MAX_COLUMN_WIDTH: usize = 24;

/// JSON response for `airtable records <table>` with `--json`.
#[derive(Debug, Serialize)]
pub struct RecordsResult {
    /// Airtable base id.
    pub base_id: String,
    /// Logical table name from config.
    pub table: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Number of records fetched.
    pub record_count: usize,
    /// Every record, in Airtable's returned order.
    pub records: Vec<AirtableRecord>,
}

/// Fetches every record for one configured Airtable table, following pagination.
pub fn records(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let table_name = matches
        .get_one::<String>("table")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing table name"))?
        .to_string();

    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let config = to_airtable_config(&validated.app)?;
    let table_id = config.table(&table_name)?.table_id.clone();
    let base_id = config.base_id.clone();
    let client_config = config;
    let fetch_table = table_name.clone();

    let all_records = block_on_async(async move {
        let built = AppBuilder::new()
            .module(HttpClientModule::default())
            .module(AirtableModule::with_config(client_config))
            .build()?;
        let client = built.context.service::<AirtableClient>()?.clone();
        client
            .list_all_records(&fetch_table, AirtableListParams::default())
            .await
    })?;

    let result = RecordsResult {
        base_id,
        table: table_name,
        table_id,
        record_count: all_records.len(),
        records: all_records,
    };

    print_records_success(&result, json, quiet)
}

fn print_records_success(result: &RecordsResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize airtable records result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    println!(
        "Fetched {} record(s) from table `{}` ({}) in base {}:",
        result.record_count, result.table, result.table_id, result.base_id
    );

    if result.records.is_empty() {
        return Ok(());
    }

    let mut field_names: BTreeSet<&str> = BTreeSet::new();
    for record in &result.records {
        field_names.extend(record.fields.0.keys().map(String::as_str));
    }
    let mut headers: Vec<String> = vec!["id".to_string()];
    headers.extend(field_names.into_iter().map(str::to_string));

    let rows: Vec<Vec<String>> = result
        .records
        .iter()
        .map(|record| {
            headers
                .iter()
                .map(|header| {
                    if header == "id" {
                        record.id.clone()
                    } else {
                        record
                            .fields
                            .0
                            .get(header)
                            .map(value_to_display)
                            .unwrap_or_default()
                    }
                })
                .collect()
        })
        .collect();

    let widths = column_widths(&headers, &rows);
    print_row(&headers, &widths);
    for row in &rows {
        print_row(row, &widths);
    }

    Ok(())
}

fn value_to_display(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn truncate_display(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

fn column_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let mut width = header.chars().count();
            for row in rows {
                if let Some(value) = row.get(index) {
                    width = width.max(value.chars().count().min(MAX_COLUMN_WIDTH));
                }
            }
            width.min(MAX_COLUMN_WIDTH)
        })
        .collect()
}

fn print_row(values: &[String], widths: &[usize]) {
    let line = values
        .iter()
        .zip(widths.iter())
        .map(|(value, width)| format!("{:<width$}", truncate_display(value, *width), width = width))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{line}");
}
