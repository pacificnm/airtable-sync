//! `mapping report` command handler.

use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{
    absolute_path, ensure_schema_cache, open_database, AirtableTableSummary, FieldMappingRow,
    SchemaStore,
};

/// Mapping counts for one table or the full report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingReportSummary {
    /// Non-computed mappable fields.
    pub fields_total: usize,
    /// Fields with a CSV mapping.
    pub mapped: usize,
    /// Fields without a CSV mapping.
    pub unmapped: usize,
    /// Fields with sync enabled.
    pub sync_enabled: usize,
    /// Mapped fields with sync disabled.
    pub mapped_sync_disabled: usize,
}

/// One mapped field with sync disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappedSyncDisabledFieldView {
    /// Field display name.
    pub field_name: String,
    /// Mapped CSV column name.
    pub csv_field: String,
    /// Source CSV file basename.
    pub csv_file: Option<String>,
}

/// Mapping report for one cached table.
#[derive(Debug, Serialize)]
pub struct MappingReportTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Whether sync is enabled for this table.
    pub enabled: bool,
    /// Per-table mapping summary.
    pub summary: MappingReportSummary,
    /// Non-computed fields without a CSV mapping.
    pub unmapped_fields: Vec<String>,
    /// Mapped fields with sync disabled.
    pub mapped_sync_disabled: Vec<MappedSyncDisabledFieldView>,
}

/// JSON response for `mapping report` with `--json`.
#[derive(Debug, Serialize)]
pub struct MappingReportResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Report-wide mapping summary.
    pub summary: MappingReportSummary,
    /// Per-table mapping reports ordered by table name.
    pub tables: Vec<MappingReportTableView>,
}

/// Generates a mapping report across all cached tables.
pub fn mapping_report(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);
    ensure_schema_cache(&database_path)?;

    let db = open_database(&database_path)?;
    let store = SchemaStore::new(db);

    let tables = store.list_tables_summary().map_err(NestError::from)?;
    let mut table_reports = Vec::with_capacity(tables.len());
    let mut overall = MappingReportSummary::default();

    for table in tables {
        let fields = store
            .list_mappable_fields(&table.table_id)
            .map_err(NestError::from)?;
        let report = build_table_report(&table, &fields);
        overall = merge_summaries(&overall, &report.summary);
        table_reports.push(report);
    }

    let result = MappingReportResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        summary: overall,
        tables: table_reports,
    };

    print_mapping_report_success(&result, json, quiet)
}

fn build_table_report(
    table: &AirtableTableSummary,
    fields: &[FieldMappingRow],
) -> MappingReportTableView {
    let summary = summarize_fields(fields);
    let unmapped_fields = fields
        .iter()
        .filter(|field| field.csv_field.is_none())
        .map(|field| field.field_name.clone())
        .collect();
    let mapped_sync_disabled = fields
        .iter()
        .filter(|field| field.csv_field.is_some() && !field.sync_enabled)
        .map(|field| MappedSyncDisabledFieldView {
            field_name: field.field_name.clone(),
            csv_field: field.csv_field.clone().unwrap_or_default(),
            csv_file: field.csv_filename.clone(),
        })
        .collect();

    MappingReportTableView {
        name: table.name.clone(),
        table_id: table.table_id.clone(),
        enabled: table.enabled,
        summary,
        unmapped_fields,
        mapped_sync_disabled,
    }
}

fn summarize_fields(fields: &[FieldMappingRow]) -> MappingReportSummary {
    let fields_total = fields.len();
    let mapped = fields.iter().filter(|field| field.csv_field.is_some()).count();
    let sync_enabled = fields.iter().filter(|field| field.sync_enabled).count();
    let mapped_sync_disabled = fields
        .iter()
        .filter(|field| field.csv_field.is_some() && !field.sync_enabled)
        .count();

    MappingReportSummary {
        fields_total,
        mapped,
        unmapped: fields_total.saturating_sub(mapped),
        sync_enabled,
        mapped_sync_disabled,
    }
}

fn merge_summaries(
    left: &MappingReportSummary,
    right: &MappingReportSummary,
) -> MappingReportSummary {
    MappingReportSummary {
        fields_total: left.fields_total + right.fields_total,
        mapped: left.mapped + right.mapped,
        unmapped: left.unmapped + right.unmapped,
        sync_enabled: left.sync_enabled + right.sync_enabled,
        mapped_sync_disabled: left.mapped_sync_disabled + right.mapped_sync_disabled,
    }
}

impl Default for MappingReportSummary {
    fn default() -> Self {
        Self {
            fields_total: 0,
            mapped: 0,
            unmapped: 0,
            sync_enabled: 0,
            mapped_sync_disabled: 0,
        }
    }
}

fn print_mapping_report_success(
    result: &MappingReportResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping report result: {error}"))
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

    println!(
        "Mapping report for base {} ({}):",
        result.base_id,
        result.database_path.display()
    );
    println!(
        "Summary: {} table(s), {} mappable field(s), {} mapped, {} unmapped, {} sync enabled, {} mapped but sync disabled",
        result.tables.len(),
        result.summary.fields_total,
        result.summary.mapped,
        result.summary.unmapped,
        result.summary.sync_enabled,
        result.summary.mapped_sync_disabled
    );

    for table in &result.tables {
        println!();
        println!(
            "Table `{}` ({}) — {} mapped / {} fields, {} sync enabled",
            table.name,
            table.table_id,
            table.summary.mapped,
            table.summary.fields_total,
            table.summary.sync_enabled
        );

        if table.unmapped_fields.is_empty() {
            println!("  Unmapped: none");
        } else {
            println!("  Unmapped ({}): {}", table.unmapped_fields.len(), table.unmapped_fields.join(", "));
        }

        if !table.mapped_sync_disabled.is_empty() {
            println!("  Mapped, sync disabled:");
            for field in &table.mapped_sync_disabled {
                println!(
                    "    - {} -> {} ({})",
                    field.field_name,
                    field.csv_field,
                    field.csv_file.as_deref().unwrap_or("-")
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_fields_counts_unmapped_and_mapped_sync_disabled() {
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
            FieldMappingRow {
                field_id: Some("fld3".to_string()),
                field_name: "Area".to_string(),
                field_type: Some("number".to_string()),
                is_key: false,
                csv_field: Some("area".to_string()),
                csv_filename: Some("space.csv".to_string()),
                sync_enabled: false,
            },
        ];

        let summary = summarize_fields(&fields);
        assert_eq!(summary.fields_total, 3);
        assert_eq!(summary.mapped, 2);
        assert_eq!(summary.unmapped, 1);
        assert_eq!(summary.sync_enabled, 1);
        assert_eq!(summary.mapped_sync_disabled, 1);
    }
}
