//! `mapping auto` command: auto-maps CSV columns to Airtable fields by name.
//!
//! For each sync-enabled configured table: resolves which CSV file the
//! table's rows come from (via its `primary_key_field`'s existing mapping,
//! or by an unambiguous name match if not yet mapped), then maps every other
//! unmapped, non-computed field whose name matches a column in that file
//! (case/whitespace-insensitive, via the same [`normalize_header`] used
//! everywhere else in the mapping layer). Fields with no matching column, or
//! whose table's CSV file can't be resolved, are left unmapped and reported.

use std::collections::HashSet;
use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use nest_file_csv::normalize_header;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path, AirtableTableEntry};
use crate::db::{
    absolute_path, ensure_csv_cache, ensure_schema_cache, open_database, CsvFieldRow, CsvStore,
    FieldMappingRow, FieldMappingUpdate, SchemaStore,
};

/// One field newly mapped (or left unresolved) by an auto-map run.
#[derive(Debug, Clone, Serialize)]
pub struct AutoMapFieldView {
    /// Airtable field name.
    pub field_name: String,
    /// Normalized CSV column it was matched to.
    pub csv_field: String,
    /// Source CSV file basename.
    pub csv_file: String,
}

/// Auto-mapping outcome for one configured table.
#[derive(Debug, Clone, Serialize)]
pub struct AutoMapTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Fields newly mapped during this run.
    pub mapped: Vec<AutoMapFieldView>,
    /// Fields that already had a mapping before this run.
    pub already_mapped: usize,
    /// Non-computed field names with no matching CSV column.
    pub unresolved: Vec<String>,
}

/// JSON response for `mapping auto` with `--json`.
#[derive(Debug, Serialize)]
pub struct AutoMapResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Per-table auto-map outcomes (sync-enabled tables only).
    pub tables: Vec<AutoMapTableView>,
    /// Total fields newly mapped across all tables.
    pub mapped_total: usize,
    /// Total fields left unresolved across all tables.
    pub unresolved_total: usize,
}

/// Auto-maps CSV columns to Airtable fields by name for every sync-enabled table.
pub fn auto_map(ctx: &AppContext) -> NestResult<()> {
    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    let result = compute_auto_map(ctx, quiet)?;
    print_auto_map_success(&result, json, quiet)
}

/// Runs the auto-map pass, returning the structured result without printing.
/// Shared by [`auto_map`] and `setup init`.
pub(crate) fn compute_auto_map(ctx: &AppContext, quiet: bool) -> NestResult<AutoMapResult> {
    let validated = ensure_valid_config(ctx)?;

    if !quiet {
        for warning in &validated.warnings {
            print_warning(warning);
        }
    }

    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);
    ensure_schema_cache(&database_path)?;
    ensure_csv_cache(&database_path)?;

    let db = open_database(&database_path)?;
    let schema_store = SchemaStore::new(db.clone());
    let csv_store = CsvStore::new(db);

    let mut table_names: Vec<&String> = validated.app.airtable.tables.keys().collect();
    table_names.sort();

    let mut tables = Vec::new();
    let mut mapped_total = 0usize;
    let mut unresolved_total = 0usize;

    for name in table_names {
        let table_cfg = &validated.app.airtable.tables[name];
        if !table_cfg.sync {
            continue;
        }

        let Some(table) = schema_store
            .find_table_by_name(name)
            .map_err(NestError::from)?
        else {
            continue;
        };

        let fields = schema_store
            .list_mappable_fields(&table.table_id)
            .map_err(NestError::from)?;

        let csv_file = resolve_table_csv_file(&csv_store, table_cfg, &fields);

        let mut view = AutoMapTableView {
            name: name.clone(),
            table_id: table.table_id.clone(),
            mapped: Vec::new(),
            already_mapped: 0,
            unresolved: Vec::new(),
        };

        for field in &fields {
            if field.csv_field.is_some() {
                view.already_mapped += 1;
                continue;
            }

            let normalized = normalize_header(&field.field_name, true, true);
            let candidates = csv_store
                .find_by_normalized_name(&normalized)
                .map_err(NestError::from)?;

            let matched = match_candidate(&candidates, csv_file.as_deref());
            let Some(matched) = matched else {
                view.unresolved.push(field.field_name.clone());
                continue;
            };

            let updated = schema_store
                .set_field_mapping(
                    &table.table_id,
                    &field.field_name,
                    &FieldMappingUpdate {
                        csv_field: matched.normalized_name.clone(),
                        csv_filename: matched.filename.clone(),
                        sync_enabled: Some(true),
                    },
                )
                .map_err(NestError::from)?;

            if updated {
                view.mapped.push(AutoMapFieldView {
                    field_name: field.field_name.clone(),
                    csv_field: matched.normalized_name.clone(),
                    csv_file: matched.filename.clone(),
                });
            } else {
                view.unresolved.push(field.field_name.clone());
            }
        }

        mapped_total += view.mapped.len();
        unresolved_total += view.unresolved.len();
        tables.push(view);
    }

    Ok(AutoMapResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        tables,
        mapped_total,
        unresolved_total,
    })
}

/// Resolves which CSV file a table's rows come from: the primary key
/// field's existing mapping if it has one, else an unambiguous name match.
/// `None` means it can't be determined yet — auto-map then only accepts
/// unambiguous (single-file) matches for this table's other fields too.
fn resolve_table_csv_file(
    csv_store: &CsvStore,
    table_cfg: &AirtableTableEntry,
    fields: &[FieldMappingRow],
) -> Option<String> {
    let primary_key_field = table_cfg.primary_key_field.as_deref()?;

    if let Some(existing) = fields.iter().find(|field| field.field_name == primary_key_field) {
        if let Some(filename) = &existing.csv_filename {
            return Some(filename.clone());
        }
    }

    let normalized = normalize_header(primary_key_field, true, true);
    let candidates = csv_store.find_by_normalized_name(&normalized).ok()?;
    unique_filename(&candidates)
}

fn match_candidate<'a>(candidates: &'a [CsvFieldRow], csv_file: Option<&str>) -> Option<&'a CsvFieldRow> {
    match csv_file {
        Some(file) => candidates.iter().find(|row| row.filename == file),
        None => {
            if unique_filename(candidates).is_some() {
                candidates.first()
            } else {
                None
            }
        }
    }
}

fn unique_filename(candidates: &[CsvFieldRow]) -> Option<String> {
    let filenames: HashSet<&str> = candidates.iter().map(|row| row.filename.as_str()).collect();
    if filenames.len() == 1 {
        candidates.first().map(|row| row.filename.clone())
    } else {
        None
    }
}

fn print_auto_map_success(result: &AutoMapResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping auto result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.tables.is_empty() {
        println!("No sync-enabled tables with a cached schema — run `airtable pull-schema` first.");
        return Ok(());
    }

    for table in &result.tables {
        println!(
            "`{}` ({}): {} mapped, {} already mapped, {} unresolved",
            table.name,
            table.table_id,
            table.mapped.len(),
            table.already_mapped,
            table.unresolved.len()
        );
        for field in &table.mapped {
            println!("  {} -> {} ({})", field.field_name, field.csv_field, field.csv_file);
        }
        for field_name in &table.unresolved {
            println!("  {field_name}: no matching CSV column");
        }
    }

    println!(
        "Mapped {} field(s), {} left unresolved.",
        result.mapped_total, result.unresolved_total
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(filename: &str) -> CsvFieldRow {
        CsvFieldRow {
            filename: filename.to_string(),
            name: "Name".to_string(),
            normalized_name: "name".to_string(),
        }
    }

    #[test]
    fn match_candidate_prefers_resolved_file() {
        let candidates = vec![row("location.csv"), row("space.csv")];
        let matched = match_candidate(&candidates, Some("space.csv")).unwrap();
        assert_eq!(matched.filename, "space.csv");
    }

    #[test]
    fn match_candidate_accepts_unambiguous_match_without_resolved_file() {
        let candidates = vec![row("location.csv")];
        let matched = match_candidate(&candidates, None).unwrap();
        assert_eq!(matched.filename, "location.csv");
    }

    #[test]
    fn match_candidate_rejects_ambiguous_match_without_resolved_file() {
        let candidates = vec![row("location.csv"), row("space.csv")];
        assert!(match_candidate(&candidates, None).is_none());
    }

    #[test]
    fn match_candidate_rejects_when_resolved_file_has_no_candidate() {
        let candidates = vec![row("location.csv")];
        assert!(match_candidate(&candidates, Some("space.csv")).is_none());
    }
}
