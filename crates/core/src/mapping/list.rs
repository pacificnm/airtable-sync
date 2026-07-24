//! `mapping list` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{
    absolute_path, ensure_schema_cache, open_database, AirtableTableRow, FieldMappingRow,
    SchemaStore,
};

/// Cached table metadata in mapping list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingListTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Whether sync is enabled for this table.
    pub enabled: bool,
}

/// One field in mapping list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingListFieldView {
    /// Field display name.
    pub field_name: String,
    /// Airtable field id (`fld…`), when known.
    pub field_id: Option<String>,
    /// Airtable field type.
    pub field_type: Option<String>,
    /// Whether this field is the table primary key.
    pub is_key: bool,
    /// Mapped CSV column name, if any.
    pub csv_field: Option<String>,
    /// Source CSV file basename for the mapped column, if any.
    pub csv_file: Option<String>,
    /// Whether field sync is enabled.
    pub sync_enabled: bool,
}

/// Mapping summary counts for one table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingListSummary {
    /// Non-computed fields returned.
    pub fields_total: usize,
    /// Fields with a CSV mapping.
    pub mapped: usize,
    /// Fields with sync enabled.
    pub sync_enabled: usize,
}

/// One table's mapping state — the shared shape for both a single-table
/// lookup and each entry of an all-tables listing.
#[derive(Debug, Clone, Serialize)]
pub struct MappingListTableResult {
    /// Cached table metadata.
    pub table: MappingListTableView,
    /// Mapping summary counts.
    pub summary: MappingListSummary,
    /// Mappable fields ordered by name.
    pub fields: Vec<MappingListFieldView>,
}

/// JSON response for `mapping list <table>` with `--json`.
#[derive(Debug, Serialize)]
pub struct MappingListResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Cached table metadata.
    pub table: MappingListTableView,
    /// Mapping summary counts.
    pub summary: MappingListSummary,
    /// Mappable fields ordered by name.
    pub fields: Vec<MappingListFieldView>,
}

/// JSON response for `mapping list` with no table (every cached table) with `--json`.
#[derive(Debug, Serialize)]
pub struct MappingListAllResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Per-table mapping state, ordered by table name.
    pub tables: Vec<MappingListTableResult>,
}

/// Lists field mapping state from SQLite — for one configured table when
/// `table` is given, or every cached table when it's omitted.
pub fn list_mappings(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let table_name = matches.get_one::<String>("table").map(String::as_str);

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
    let base_id = &validated.app.airtable.base_id;

    match table_name {
        Some(name) => {
            let Some(table) = store.find_table_by_name(name).map_err(NestError::from)? else {
                return Err(NestError::data(format!(
                    "table `{name}` not in cache — run `airtable pull-schema` or `airtable list-tables`"
                )));
            };
            let entry = build_table_result(&store, &table.name, &table.table_id, table.enabled)?;
            let result = MappingListResult {
                database_path: absolute_path(&database_path),
                base_id: base_id.clone(),
                table: entry.table,
                summary: entry.summary,
                fields: entry.fields,
            };
            print_mapping_list_success(&result, json, quiet)
        }
        None => {
            let summaries = store.list_tables_summary().map_err(NestError::from)?;
            let mut tables = Vec::with_capacity(summaries.len());
            for summary in &summaries {
                tables.push(build_table_result(
                    &store,
                    &summary.name,
                    &summary.table_id,
                    summary.enabled,
                )?);
            }
            let result = MappingListAllResult {
                database_path: absolute_path(&database_path),
                base_id: base_id.clone(),
                tables,
            };
            print_mapping_list_all_success(&result, json, quiet)
        }
    }
}

fn build_table_result(
    store: &SchemaStore,
    name: &str,
    table_id: &str,
    enabled: bool,
) -> NestResult<MappingListTableResult> {
    let fields = store.list_mappable_fields(table_id).map_err(NestError::from)?;
    let summary = summarize_fields(&fields);
    Ok(MappingListTableResult {
        table: MappingListTableView {
            name: name.to_string(),
            table_id: table_id.to_string(),
            enabled,
        },
        summary,
        fields: fields.into_iter().map(MappingListFieldView::from).collect(),
    })
}

impl From<AirtableTableRow> for MappingListTableView {
    fn from(table: AirtableTableRow) -> Self {
        Self {
            name: table.name,
            table_id: table.table_id,
            enabled: table.enabled,
        }
    }
}

impl From<FieldMappingRow> for MappingListFieldView {
    fn from(field: FieldMappingRow) -> Self {
        Self {
            field_name: field.field_name,
            field_id: field.field_id,
            field_type: field.field_type,
            is_key: field.is_key,
            csv_field: field.csv_field,
            csv_file: field.csv_filename,
            sync_enabled: field.sync_enabled,
        }
    }
}

fn summarize_fields(fields: &[FieldMappingRow]) -> MappingListSummary {
    MappingListSummary {
        fields_total: fields.len(),
        mapped: fields.iter().filter(|field| field.csv_field.is_some()).count(),
        sync_enabled: fields
            .iter()
            .filter(|field| field.sync_enabled)
            .count(),
    }
}

fn print_mapping_list_success(
    result: &MappingListResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping list result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    println!(
        "Field mappings for table `{}` ({}) in base {}:",
        result.table.name,
        result.table.table_id,
        result.base_id
    );
    println!(
        "Summary: {} mappable field(s), {} mapped, {} sync enabled",
        result.summary.fields_total, result.summary.mapped, result.summary.sync_enabled
    );

    print_fields_table(&result.table.name, &result.fields);

    if result.summary.mapped == 0 {
        println!("No CSV mappings yet — run `mapping auto` or `mapping set`.");
    }

    Ok(())
}

fn print_mapping_list_all_success(
    result: &MappingListAllResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping list result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.tables.is_empty() {
        println!("No tables in cache — run `airtable pull-schema` first.");
        return Ok(());
    }

    println!("Field mappings for base {}:", result.base_id);
    for entry in &result.tables {
        println!(
            "\n`{}` ({}): {} mappable field(s), {} mapped, {} sync enabled",
            entry.table.name,
            entry.table.table_id,
            entry.summary.fields_total,
            entry.summary.mapped,
            entry.summary.sync_enabled
        );
        print_fields_table(&entry.table.name, &entry.fields);
    }

    Ok(())
}

fn print_fields_table(table_name: &str, fields: &[MappingListFieldView]) {
    if fields.is_empty() {
        println!("No mappable fields in cache — run `airtable pull-schema` first.");
        return;
    }

    println!(
        "{:<20} {:<20} {:<12} {:<16} {:<6} {}",
        "table", "field_name", "csv_field", "csv_file", "sync", "key"
    );
    for field in fields {
        println!(
            "{:<20} {:<20} {:<12} {:<16} {:<6} {}",
            table_name,
            field.field_name,
            field.csv_field.as_deref().unwrap_or("-"),
            field.csv_file.as_deref().unwrap_or("-"),
            yes_no(field.sync_enabled),
            yes_no(field.is_key)
        );
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_fields_counts_mapped_and_sync_enabled() {
        let fields = vec![
            FieldMappingRow {
                field_id: Some("fld1".to_string()),
                field_name: "Name".to_string(),
                field_type: Some("singleLineText".to_string()),
                is_key: true,
                csv_field: Some("name".to_string()),
                csv_filename: Some("location.csv".to_string()),
                sync_enabled: true,
            },
            FieldMappingRow {
                field_id: Some("fld2".to_string()),
                field_name: "Status".to_string(),
                field_type: Some("singleSelect".to_string()),
                is_key: false,
                csv_field: None,
                csv_filename: None,
                sync_enabled: false,
            },
        ];

        let summary = summarize_fields(&fields);
        assert_eq!(summary.fields_total, 2);
        assert_eq!(summary.mapped, 1);
        assert_eq!(summary.sync_enabled, 1);
    }
}
