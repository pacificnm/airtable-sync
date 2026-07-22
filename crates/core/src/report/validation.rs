//! `report validation` command handler.

use std::collections::HashMap;
use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_config::ConfigService;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use nest_validation::ValidationIssue;
use serde::Serialize;

use crate::config::{collect_validation_issues, resolve_config_path, AppConfig};
use crate::csv::{validate_all_configured_csv, ValidateResult};
use crate::mapping::{
    build_mapping_table_reports, MappingReportSummary, MappingReportTableView,
};
use crate::db::{
    absolute_path, ensure_csv_cache, ensure_schema_cache, open_database, CsvStore, SchemaStore,
};

/// One validation issue in report output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssueView {
    /// Config field path when applicable.
    pub field: Option<String>,
    /// Human-readable message.
    pub message: String,
    /// Optional remediation hint.
    pub help: Option<String>,
    /// `error` or `warning`.
    pub severity: String,
}

/// Database and cache readiness for compare/sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatabaseReadinessView {
    /// Whether database prerequisites passed.
    pub ready: bool,
    /// Cached Airtable tables.
    pub schema_tables: usize,
    /// Imported CSV header fields.
    pub csv_fields: usize,
    /// Blocking issues.
    pub errors: Vec<String>,
    /// Non-blocking issues.
    pub warnings: Vec<String>,
}

/// Mapping readiness for one sync-enabled table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingReadinessView {
    /// Logical table name.
    pub table_name: String,
    /// Whether this table is ready for compare/sync.
    pub ready: bool,
    /// Configured primary key field name.
    pub primary_key_field: Option<String>,
    /// Whether the primary key is mapped to a CSV column.
    pub primary_key_mapped: bool,
    /// Whether the primary key mapping has sync enabled.
    pub primary_key_sync_enabled: bool,
    /// Non-computed fields without a CSV mapping.
    pub unmapped_fields: Vec<String>,
    /// Blocking issues.
    pub errors: Vec<String>,
    /// Non-blocking issues.
    pub warnings: Vec<String>,
}

/// Rollup counts across all sections.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ValidationReportSummary {
    /// Blocking issues.
    pub errors: usize,
    /// Non-blocking issues.
    pub warnings: usize,
}

/// Configuration section of the validation report.
#[derive(Debug, Serialize)]
pub struct ValidationSectionView {
    /// Whether configuration has no blocking issues.
    pub valid: bool,
    /// Blocking configuration issues.
    pub errors: Vec<ValidationIssueView>,
    /// Non-blocking configuration warnings.
    pub warnings: Vec<ValidationIssueView>,
}

/// Mapping section of the validation report.
#[derive(Debug, Serialize)]
pub struct MappingSectionView {
    /// Whether all sync-enabled tables are mapping-ready.
    pub valid: bool,
    /// Report-wide mapping summary.
    pub summary: MappingReportSummary,
    /// Per-table mapping readiness.
    pub tables: Vec<MappingReadinessView>,
}

/// JSON response for `report validation` with `--json`.
#[derive(Debug, Serialize)]
pub struct ValidationReportResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Whether the environment is ready for compare/sync.
    pub valid: bool,
    /// Rollup counts.
    pub summary: ValidationReportSummary,
    /// Configuration validation results.
    pub config: ValidationSectionView,
    /// CSV structural validation results.
    pub csv: ValidateResult,
    /// Database and cache readiness.
    pub database: DatabaseReadinessView,
    /// Mapping readiness for sync-enabled tables.
    pub mappings: MappingSectionView,
}

/// Generates an end-to-end pre-sync validation report.
pub fn report_validation(ctx: &AppContext) -> NestResult<()> {
    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    let config = ctx.service::<ConfigService>()?;
    let app = AppConfig::from_service(&config).map_err(|error| {
        NestError::validation(format!("failed to load configuration: {error}"))
    })?;

    let database_path = resolve_config_path(&config, &app.database.database_path);
    let database_path_abs = absolute_path(&database_path);

    let config_section = build_config_section(&collect_validation_issues(&config, &app));

    if !quiet {
        for warning in &config_section.warnings {
            print_warning_from_view(warning);
        }
    }

    let csv = validate_csv_section(&config, &app);
    let database = assess_database_readiness(&database_path);
    let mappings = if database.ready {
        assess_mapping_readiness(&database_path, &app)?
    } else {
        MappingSectionView {
            valid: false,
            summary: MappingReportSummary::default(),
            tables: Vec::new(),
        }
    };

    let summary = summarize_report(&config_section, &csv, &database, &mappings);
    let valid = summary.errors == 0;
    let result = ValidationReportResult {
        database_path: database_path_abs,
        base_id: app.airtable.base_id.clone(),
        valid,
        summary,
        config: config_section,
        csv,
        database,
        mappings,
    };

    print_validation_report(&result, json, quiet)?;

    if valid {
        Ok(())
    } else {
        Err(NestError::validation(format!(
            "validation report found {} blocking issue(s)",
            result.summary.errors
        )))
    }
}

fn build_config_section(issues: &[ValidationIssue]) -> ValidationSectionView {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for issue in issues {
        let view = issue_view(issue);
        if issue.is_blocking() {
            errors.push(view);
        } else {
            warnings.push(view);
        }
    }

    ValidationSectionView {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

fn issue_view(issue: &ValidationIssue) -> ValidationIssueView {
    ValidationIssueView {
        field: issue.field.as_ref().map(|path| path.as_str().to_string()),
        message: issue.message.clone(),
        help: issue.help.clone(),
        severity: if issue.is_blocking() {
            "error".to_string()
        } else {
            "warning".to_string()
        },
    }
}

fn print_warning_from_view(issue: &ValidationIssueView) {
    if let Some(field) = &issue.field {
        println!("warning: {}: {}", field, issue.message);
    } else {
        println!("warning: {}", issue.message);
    }
}

fn validate_csv_section(config: &ConfigService, app: &AppConfig) -> ValidateResult {
    validate_all_configured_csv(config, app)
}

fn assess_database_readiness(database_path: &std::path::Path) -> DatabaseReadinessView {
    let mut errors = Vec::new();
    let warnings = Vec::new();
    let mut schema_tables = 0usize;
    let mut csv_fields = 0usize;

    if let Err(error) = ensure_schema_cache(database_path) {
        errors.push(error.message().to_string());
        return DatabaseReadinessView {
            ready: false,
            schema_tables,
            csv_fields,
            errors,
            warnings,
        };
    }

    if let Err(error) = ensure_csv_cache(database_path) {
        errors.push(error.message().to_string());
        return DatabaseReadinessView {
            ready: false,
            schema_tables,
            csv_fields,
            errors,
            warnings,
        };
    }

    let db = match open_database(database_path) {
        Ok(db) => db,
        Err(error) => {
            errors.push(error.message().to_string());
            return DatabaseReadinessView {
                ready: false,
                schema_tables,
                csv_fields,
                errors,
                warnings,
            };
        }
    };

    let schema_store = SchemaStore::new(db);
    let csv_store = match open_database(database_path) {
        Ok(db) => CsvStore::new(db),
        Err(error) => {
            errors.push(error.message().to_string());
            return DatabaseReadinessView {
                ready: false,
                schema_tables,
                csv_fields,
                errors,
                warnings,
            };
        }
    };

    match schema_store.list_tables_summary() {
        Ok(tables) => {
            schema_tables = tables.len();
            if schema_tables == 0 {
                errors.push(
                    "schema cache is empty — run `airtable pull-schema`".to_string(),
                );
            }
        }
        Err(error) => errors.push(error.to_string()),
    }

    match csv_store.list_fields() {
        Ok(fields) => {
            csv_fields = fields.len();
            if csv_fields == 0 {
                errors.push(
                    "CSV header cache is empty — run `csv import-headers`".to_string(),
                );
            }
        }
        Err(error) => errors.push(error.to_string()),
    }

    DatabaseReadinessView {
        ready: errors.is_empty(),
        schema_tables,
        csv_fields,
        errors,
        warnings,
    }
}

fn assess_mapping_readiness(
    database_path: &std::path::Path,
    app: &AppConfig,
) -> NestResult<MappingSectionView> {
    let db = open_database(database_path)?;
    let store = SchemaStore::new(db);
    let tables = store.list_tables_summary().map_err(NestError::from)?;
    let (table_reports, summary) = build_mapping_table_reports(&store, &tables)?;

    let primary_keys: HashMap<String, String> = app
        .airtable
        .tables
        .iter()
        .filter_map(|(name, table)| {
            table
                .primary_key_field
                .as_ref()
                .map(|field| (name.clone(), field.clone()))
        })
        .collect();

    let mut readiness_tables = Vec::new();
    for report in &table_reports {
        if !report.enabled {
            continue;
        }

        readiness_tables.push(build_mapping_readiness(
            report,
            primary_keys.get(&report.name).map(String::as_str),
        ));
    }

    let valid = readiness_tables.iter().all(|table| table.ready);
    Ok(MappingSectionView {
        valid,
        summary,
        tables: readiness_tables,
    })
}

fn build_mapping_readiness(
    report: &MappingReportTableView,
    primary_key_field: Option<&str>,
) -> MappingReadinessView {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let (primary_key_mapped, primary_key_sync_enabled) =
        if let Some(primary_key_field) = primary_key_field {
            let pk_unmapped = report.unmapped_fields.iter().any(|name| name == primary_key_field);
            let pk_disabled = report
                .mapped_sync_disabled
                .iter()
                .any(|field| field.field_name == primary_key_field);

            if pk_unmapped {
                errors.push(format!(
                    "primary key field `{primary_key_field}` is not mapped to a CSV column"
                ));
                (false, false)
            } else if pk_disabled {
                errors.push(format!(
                    "primary key field `{primary_key_field}` is mapped but sync is disabled"
                ));
                (true, false)
            } else {
                (true, true)
            }
        } else {
            errors.push("primary_key_field is not configured for this sync-enabled table".to_string());
            (false, false)
        };

    if !report.unmapped_fields.is_empty() {
        warnings.push(format!(
            "{} unmapped field(s): {}",
            report.unmapped_fields.len(),
            report.unmapped_fields.join(", ")
        ));
    }

    let ready = errors.is_empty();
    MappingReadinessView {
        table_name: report.name.clone(),
        ready,
        primary_key_field: primary_key_field.map(str::to_string),
        primary_key_mapped,
        primary_key_sync_enabled,
        unmapped_fields: report.unmapped_fields.clone(),
        errors,
        warnings,
    }
}

fn summarize_report(
    config: &ValidationSectionView,
    csv: &ValidateResult,
    database: &DatabaseReadinessView,
    mappings: &MappingSectionView,
) -> ValidationReportSummary {
    let mut errors = config.errors.len();
    let mut warnings = config.warnings.len();

    for file in &csv.files {
        errors += file.errors.len();
        warnings += file.warnings.len();
    }
    if !csv.valid && csv.files.iter().all(|file| file.errors.is_empty()) {
        errors += 1;
    }

    errors += database.errors.len();
    warnings += database.warnings.len();

    for table in &mappings.tables {
        errors += table.errors.len();
        warnings += table.warnings.len();
    }
    if !mappings.valid && mappings.tables.is_empty() && !database.ready {
        // mapping section skipped — already counted via database
    }

    ValidationReportSummary { errors, warnings }
}

fn print_validation_report(
    result: &ValidationReportResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize validation report result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    let status = if result.valid { "passed" } else { "failed" };
    println!(
        "Validation report for base {} ({status}) — {} error(s), {} warning(s)",
        result.base_id, result.summary.errors, result.summary.warnings
    );

    println!();
    println!(
        "Config: {}",
        if result.config.valid {
            "ok"
        } else {
            "failed"
        }
    );
    for issue in &result.config.errors {
        print_issue_line("error", issue);
    }

    println!();
    println!(
        "CSV: {} file(s), {} row(s) — {}",
        result.csv.files.len(),
        result.csv.rows_total,
        if result.csv.valid { "ok" } else { "failed" }
    );
    for file in &result.csv.files {
        println!(
            "  {} ({}) — {}",
            file.role,
            file.path.display(),
            if file.valid { "ok" } else { "failed" }
        );
        for error in &file.errors {
            println!("    error: {error}");
        }
        for warning in &file.warnings {
            println!("    warning: {warning}");
        }
    }

    println!();
    println!(
        "Database: {} table(s) cached, {} CSV field(s) imported — {}",
        result.database.schema_tables,
        result.database.csv_fields,
        if result.database.ready {
            "ok"
        } else {
            "not ready"
        }
    );
    for error in &result.database.errors {
        println!("  error: {error}");
    }
    for warning in &result.database.warnings {
        println!("  warning: {warning}");
    }

    println!();
    println!(
        "Mappings: {} sync-enabled table(s) — {}",
        result.mappings.tables.len(),
        if result.mappings.valid {
            "ok"
        } else {
            "not ready"
        }
    );
    for table in &result.mappings.tables {
        println!(
            "  `{}` — {}",
            table.table_name,
            if table.ready { "ok" } else { "failed" }
        );
        for error in &table.errors {
            println!("    error: {error}");
        }
        for warning in &table.warnings {
            println!("    warning: {warning}");
        }
    }

    Ok(())
}

fn print_issue_line(prefix: &str, issue: &ValidationIssueView) {
    let field = issue.field.as_deref().unwrap_or("config");
    if let Some(help) = &issue.help {
        println!("  {prefix}: {field}: {} ({help})", issue.message);
    } else {
        println!("  {prefix}: {field}: {}", issue.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::MappingReportTableView;

    fn sample_mapping_report(unmapped: &[&str]) -> MappingReportTableView {
        MappingReportTableView {
            name: "assets".to_string(),
            table_id: "tblTEST".to_string(),
            enabled: true,
            summary: MappingReportSummary::default(),
            unmapped_fields: unmapped.iter().map(|value| (*value).to_string()).collect(),
            mapped_sync_disabled: Vec::new(),
        }
    }

    #[test]
    fn build_mapping_readiness_errors_when_primary_key_unmapped() {
        let readiness = build_mapping_readiness(&sample_mapping_report(&["ID", "Name"]), Some("ID"));
        assert!(!readiness.ready);
        assert!(!readiness.primary_key_mapped);
        assert!(readiness
            .errors
            .iter()
            .any(|error| error.contains("primary key")));
    }

    #[test]
    fn build_mapping_readiness_passes_when_primary_key_mapped() {
        let readiness = build_mapping_readiness(&sample_mapping_report(&["Status"]), Some("Name"));
        assert!(readiness.ready);
        assert!(readiness.primary_key_mapped);
        assert!(readiness.primary_key_sync_enabled);
    }
}
