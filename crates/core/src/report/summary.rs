//! `report summary` command handler.

use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::csv::{validate_all_configured_csv, ValidateResult};
use crate::db::{
    absolute_path, ensure_change_plan_schema, ensure_csv_cache, ensure_schema_cache,
    open_database, ChangePlanHeader, CsvStore, SchemaStore,
};
use crate::mapping::{
    build_mapping_table_reports, MappingReportSummary, MappingReportTableView,
};
use crate::sync::plan_context::open_plan_store;

/// Sync-related settings from config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncSettingsView {
    /// When true, sync apply refuses to write to Airtable.
    pub dry_run: bool,
    /// When true, keep processing after non-fatal errors.
    pub continue_on_error: bool,
    /// When true, persist change plans from dry-run.
    pub create_change_plan: bool,
    /// Tables defined in config.
    pub tables_configured: usize,
    /// Tables with `sync = true`.
    pub tables_sync_enabled: usize,
}

/// Cached schema and CSV header counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CacheSummaryView {
    /// Whether schema and CSV caches are populated.
    pub ready: bool,
    /// Cached Airtable tables.
    pub schema_tables: usize,
    /// Cached Airtable fields across all tables.
    pub schema_fields: usize,
    /// Imported CSV header fields.
    pub csv_fields: usize,
}

/// Per-table mapping counts for the summary dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingSummaryTableView {
    /// Logical table name.
    pub name: String,
    /// Whether sync is enabled for this table.
    pub enabled: bool,
    /// Mappable fields in cache.
    pub fields_total: usize,
    /// Fields with a CSV mapping.
    pub mapped: usize,
    /// Fields without a CSV mapping.
    pub unmapped: usize,
    /// Fields with sync enabled.
    pub sync_enabled: usize,
}

/// Mapping section of the sync summary.
#[derive(Debug, Serialize)]
pub struct MappingSummarySection {
    /// Report-wide mapping summary.
    pub summary: MappingReportSummary,
    /// Per-table mapping counts (sync-enabled tables only).
    pub tables: Vec<MappingSummaryTableView>,
}

/// JSON response for `report summary` with `--json`.
#[derive(Debug, Serialize)]
pub struct SyncSummaryResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Sync settings and table counts.
    pub sync: SyncSettingsView,
    /// Schema and CSV cache counts.
    pub cache: CacheSummaryView,
    /// Mapping rollup.
    pub mappings: MappingSummarySection,
    /// CSV file row counts.
    pub csv: ValidateResult,
    /// Latest change plan, when `create_change_plan` is enabled.
    pub change_plan: Option<ChangePlanHeader>,
}

/// Generates an overall sync summary from local config and SQLite state.
pub fn report_summary(ctx: &AppContext) -> NestResult<()> {
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
    let database_path_abs = absolute_path(&database_path);
    let base_id = validated.app.airtable.base_id.clone();

    let sync = build_sync_settings(&validated.app);
    let cache = build_cache_summary(&database_path);
    let mappings = build_mapping_summary(&database_path);
    let csv = validate_all_configured_csv(&validated.config, &validated.app);
    let change_plan = load_latest_change_plan(
        &database_path,
        &base_id,
        validated.app.sync.create_change_plan,
    );

    let result = SyncSummaryResult {
        database_path: database_path_abs,
        base_id,
        sync,
        cache,
        mappings,
        csv,
        change_plan,
    };

    print_sync_summary(&result, json, quiet)
}

fn build_sync_settings(app: &crate::config::AppConfig) -> SyncSettingsView {
    let tables_configured = app.airtable.tables.len();
    let tables_sync_enabled = app
        .airtable
        .tables
        .values()
        .filter(|table| table.sync)
        .count();

    SyncSettingsView {
        dry_run: app.sync.dry_run,
        continue_on_error: app.sync.continue_on_error,
        create_change_plan: app.sync.create_change_plan,
        tables_configured,
        tables_sync_enabled,
    }
}

fn build_cache_summary(database_path: &std::path::Path) -> CacheSummaryView {
    let mut view = CacheSummaryView {
        ready: false,
        schema_tables: 0,
        schema_fields: 0,
        csv_fields: 0,
    };

    if ensure_schema_cache(database_path).is_err() {
        return view;
    }

    let Ok(db) = open_database(database_path) else {
        return view;
    };

    let schema_store = SchemaStore::new(db);
    if let Ok(tables) = schema_store.list_tables_summary() {
        view.schema_tables = tables.len();
        view.schema_fields = tables.iter().map(|table| table.field_count).sum();
    }

    if ensure_csv_cache(database_path).is_ok() {
        if let Ok(db) = open_database(database_path) {
            let csv_store = CsvStore::new(db);
            if let Ok(fields) = csv_store.list_fields() {
                view.csv_fields = fields.len();
            }
        }
    }

    view.ready = view.schema_tables > 0 && view.csv_fields > 0;
    view
}

fn build_mapping_summary(database_path: &std::path::Path) -> MappingSummarySection {
    let empty = MappingSummarySection {
        summary: MappingReportSummary::default(),
        tables: Vec::new(),
    };

    if ensure_schema_cache(database_path).is_err() {
        return empty;
    }

    let Ok(db) = open_database(database_path) else {
        return empty;
    };

    let store = SchemaStore::new(db);
    let tables = match store.list_tables_summary() {
        Ok(tables) => tables,
        Err(_) => return empty,
    };

    let (table_reports, summary) = match build_mapping_table_reports(&store, &tables) {
        Ok(reports) => reports,
        Err(_) => return empty,
    };

    MappingSummarySection {
        summary,
        tables: table_reports
            .iter()
            .filter(|table| table.enabled)
            .map(mapping_table_summary)
            .collect(),
    }
}

fn mapping_table_summary(table: &MappingReportTableView) -> MappingSummaryTableView {
    MappingSummaryTableView {
        name: table.name.clone(),
        enabled: table.enabled,
        fields_total: table.summary.fields_total,
        mapped: table.summary.mapped,
        unmapped: table.summary.unmapped,
        sync_enabled: table.summary.sync_enabled,
    }
}

fn load_latest_change_plan(
    database_path: &std::path::Path,
    base_id: &str,
    create_change_plan: bool,
) -> Option<ChangePlanHeader> {
    if !create_change_plan {
        return None;
    }

    ensure_change_plan_schema(database_path).ok()?;
    let store = open_plan_store(database_path).ok()?;
    store.find_latest_plan(base_id).ok().flatten()
}

fn print_sync_summary(result: &SyncSummaryResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize sync summary result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    println!("Overall sync summary for base {}", result.base_id);
    println!(
        "Sync: dry_run={}, {}/{} tables enabled for sync",
        result.sync.dry_run,
        result.sync.tables_sync_enabled,
        result.sync.tables_configured,
    );

    if result.cache.schema_tables == 0 && result.cache.csv_fields == 0 {
        println!("Cache: not initialized — run `db init`, `airtable pull-schema`, and `csv import-headers`");
    } else {
        println!(
            "Cache: {} table(s), {} field(s), {} CSV header(s)",
            result.cache.schema_tables, result.cache.schema_fields, result.cache.csv_fields,
        );
    }

    let mapping = &result.mappings.summary;
    println!(
        "Mappings: {}/{} mapped, {} sync enabled, {} unmapped",
        mapping.mapped, mapping.fields_total, mapping.sync_enabled, mapping.unmapped,
    );

    if result.csv.files.is_empty() {
        println!("CSV: not available");
    } else {
        let parts: Vec<String> = result
            .csv
            .files
            .iter()
            .map(|file| format!("{} {} row(s)", file.role, file.row_count))
            .collect();
        println!("CSV: {}", parts.join(", "));
    }

    match &result.change_plan {
        Some(plan) => {
            let counts = &plan.status_counts;
            println!(
                "Change plan #{} ({}): {} pending, {} approved, {} denied, {} applied, {} failed",
                plan.id,
                plan.status,
                counts.pending,
                counts.approved,
                counts.denied,
                counts.applied,
                counts.failed,
            );
        }
        None if result.sync.create_change_plan => {
            println!("Change plan: none — run `sync dry-run` to generate one");
        }
        None => {}
    }

    Ok(())
}
