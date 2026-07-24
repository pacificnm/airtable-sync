//! `setup init` — first-run wizard.
//!
//! Runs, in order: validate `config.toml`, create/migrate the SQLite
//! database, download the Airtable schema, import CSV headers, auto-map
//! matching fields by name, and report any fields still unmapped. Safe to
//! re-run — every step is idempotent (validation and reporting are read-only;
//! the database step only applies pending migrations; schema pull, header
//! import, and auto-map all upsert/skip rather than duplicate).
//!
//! Stops after the first failing step: a step's own prerequisites (a valid
//! config, a migrated database, a pulled schema) come from the steps before
//! it, so continuing past a failure would just fail again for a less useful
//! reason.

use nest_cli::CliGlobals;
use nest_config::ConfigService;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::airtable::{compute_pull_schema, PullSchemaResult};
use crate::config::{collect_validation_issues, resolve_config_path, AppConfig};
use crate::csv::{compute_import_headers, ImportHeadersResult};
use crate::db::{apply_pending_migrations, open_database, SchemaStore};
use crate::mapping::{build_mapping_table_reports, compute_auto_map, AutoMapResult, MappingReportSummary};

/// One step's pass/fail outcome in `setup init`'s output.
#[derive(Debug, Serialize)]
pub struct SetupStepView {
    /// Step name (e.g. `"validate config"`).
    pub name: &'static str,
    /// Whether the step completed successfully.
    pub ok: bool,
    /// Human-readable outcome or error message.
    pub message: String,
}

/// JSON response for `setup init --json`.
#[derive(Debug, Serialize)]
pub struct SetupInitResult {
    /// Whether every step completed successfully.
    pub ok: bool,
    /// Each step's outcome, in the order it ran.
    pub steps: Vec<SetupStepView>,
    /// Whether the SQLite database file was created during this run.
    pub database_created: bool,
    /// Airtable schema pull result, when that step ran.
    pub schema: Option<PullSchemaResult>,
    /// CSV header import result, when that step ran.
    pub csv: Option<ImportHeadersResult>,
    /// Auto-map result, when that step ran.
    pub mapping: Option<AutoMapResult>,
    /// Mapping rollup across sync-enabled tables, when computed.
    pub unmapped_summary: Option<MappingReportSummary>,
}

/// Runs the first-run setup wizard: validate config, init the database, pull
/// schema, import CSV headers, auto-map fields, and report what's unmapped.
pub fn init(ctx: &AppContext) -> NestResult<()> {
    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    let mut steps = Vec::new();
    let mut database_created = false;

    let Some(app) = validate_step(ctx, &mut steps)? else {
        return finish(steps, false, database_created, None, None, None, None, json, quiet);
    };

    let config = ctx.service::<ConfigService>()?;

    database_created = match database_step(config, &app, &mut steps) {
        Ok(created) => created,
        Err(_) => {
            return finish(steps, false, database_created, None, None, None, None, json, quiet);
        }
    };

    let schema = match compute_pull_schema(ctx, quiet) {
        Ok(result) => {
            steps.push(SetupStepView {
                name: "pull schema",
                ok: true,
                message: format!(
                    "{} table(s), {} field(s) upserted",
                    result.tables_updated, result.fields_upserted
                ),
            });
            Some(result)
        }
        Err(error) => {
            steps.push(SetupStepView {
                name: "pull schema",
                ok: false,
                message: error.message().to_string(),
            });
            None
        }
    };

    let csv = match compute_import_headers(ctx, quiet) {
        Ok(result) => {
            steps.push(SetupStepView {
                name: "import csv headers",
                ok: true,
                message: format!("{} field(s) imported", result.fields_imported),
            });
            Some(result)
        }
        Err(error) => {
            steps.push(SetupStepView {
                name: "import csv headers",
                ok: false,
                message: error.message().to_string(),
            });
            None
        }
    };

    if schema.is_none() || csv.is_none() {
        steps.push(SetupStepView {
            name: "auto-map fields",
            ok: false,
            message: "skipped — schema pull or CSV import did not complete".to_string(),
        });
        return finish(steps, false, database_created, schema, csv, None, None, json, quiet);
    }

    let mapping = match compute_auto_map(ctx, quiet) {
        Ok(result) => {
            steps.push(SetupStepView {
                name: "auto-map fields",
                ok: true,
                message: format!(
                    "{} field(s) mapped, {} unresolved",
                    result.mapped_total, result.unresolved_total
                ),
            });
            Some(result)
        }
        Err(error) => {
            steps.push(SetupStepView {
                name: "auto-map fields",
                ok: false,
                message: error.message().to_string(),
            });
            None
        }
    };

    let unmapped_summary = report_unmapped(config, &app, &mut steps);
    let ok = steps.iter().all(|step| step.ok);

    finish(steps, ok, database_created, schema, csv, mapping, unmapped_summary, json, quiet)
}

/// Validates config, recording the step outcome. Returns `None` (step already
/// recorded as failed) when validation finds blocking issues.
fn validate_step(ctx: &AppContext, steps: &mut Vec<SetupStepView>) -> NestResult<Option<AppConfig>> {
    let config = ctx.service::<ConfigService>()?;
    let app = AppConfig::from_service(config)?;
    let issues = collect_validation_issues(config, &app);
    let blocking: Vec<_> = issues.iter().filter(|issue| issue.is_blocking()).collect();

    if blocking.is_empty() {
        steps.push(SetupStepView {
            name: "validate config",
            ok: true,
            message: "config.toml is valid".to_string(),
        });
        Ok(Some(app))
    } else {
        let messages: Vec<_> = blocking.iter().map(|issue| issue.message.clone()).collect();
        steps.push(SetupStepView {
            name: "validate config",
            ok: false,
            message: messages.join("; "),
        });
        Ok(None)
    }
}

/// Creates the database if missing and applies pending migrations. Unlike
/// `db init`, safe to re-run against an already-initialized database.
fn database_step(config: &ConfigService, app: &AppConfig, steps: &mut Vec<SetupStepView>) -> NestResult<bool> {
    let database_path = resolve_config_path(config, &app.database.database_path);
    let schema_path = resolve_config_path(config, &app.database.schema);

    match apply_pending_migrations(&database_path, &schema_path, true) {
        Ok(migrate) => {
            let message = if migrate.database_created {
                format!(
                    "created {} and applied {} migration(s)",
                    database_path.display(),
                    migrate.applied.len()
                )
            } else if migrate.applied.is_empty() {
                format!("{} already up to date", database_path.display())
            } else {
                format!("applied {} pending migration(s)", migrate.applied.len())
            };
            steps.push(SetupStepView {
                name: "database",
                ok: true,
                message,
            });
            Ok(migrate.database_created)
        }
        Err(error) => {
            steps.push(SetupStepView {
                name: "database",
                ok: false,
                message: error.message().to_string(),
            });
            Err(error)
        }
    }
}

/// Builds the final mapping rollup (same building block `report validation`
/// uses), recording it as the last step.
fn report_unmapped(
    config: &ConfigService,
    app: &AppConfig,
    steps: &mut Vec<SetupStepView>,
) -> Option<MappingReportSummary> {
    let database_path = resolve_config_path(config, &app.database.database_path);

    let outcome = (|| -> NestResult<MappingReportSummary> {
        let db = open_database(&database_path)?;
        let store = SchemaStore::new(db);
        let tables = store.list_tables_summary().map_err(NestError::from)?;
        let (_, summary) = build_mapping_table_reports(&store, &tables)?;
        Ok(summary)
    })();

    match outcome {
        Ok(summary) => {
            steps.push(SetupStepView {
                name: "unmapped fields",
                ok: true,
                message: format!(
                    "{} unmapped field(s) across sync-enabled tables",
                    summary.unmapped
                ),
            });
            Some(summary)
        }
        Err(error) => {
            steps.push(SetupStepView {
                name: "unmapped fields",
                ok: false,
                message: error.message().to_string(),
            });
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn finish(
    steps: Vec<SetupStepView>,
    ok: bool,
    database_created: bool,
    schema: Option<PullSchemaResult>,
    csv: Option<ImportHeadersResult>,
    mapping: Option<AutoMapResult>,
    unmapped_summary: Option<MappingReportSummary>,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    let result = SetupInitResult {
        ok,
        steps,
        database_created,
        schema,
        csv,
        mapping,
        unmapped_summary,
    };

    print_setup_init_result(&result, json, quiet)?;

    if result.ok {
        Ok(())
    } else {
        Err(NestError::validation(
            "setup init did not complete — see the step list above",
        ))
    }
}

fn print_setup_init_result(result: &SetupInitResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize setup init result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    for step in &result.steps {
        let mark = if step.ok { "OK" } else { "FAILED" };
        println!("[{mark}] {}: {}", step.name, step.message);
    }

    Ok(())
}
