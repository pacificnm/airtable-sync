//! `csv validate` command handler.

use std::path::{Path, PathBuf};

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::csv::common::{csv_filename, resolve_csv_path, CsvFileRole};
pub(crate) use crate::csv::validate_reader::{validate_csv_file, CsvValidateReadResult};
use crate::db::absolute_path;

/// Validation outcome for one configured CSV file.
#[derive(Debug, Serialize)]
pub struct ValidateFileView {
    /// Config role (`location` or `space`).
    pub role: String,
    /// Absolute path to the CSV file.
    pub path: PathBuf,
    /// Source file basename.
    pub filename: String,
    /// Whether this file passed structural validation.
    pub valid: bool,
    /// Physical column count from the header row.
    pub column_count: usize,
    /// Number of data rows scanned.
    pub row_count: usize,
    /// Normalized header names kept in file order.
    pub headers: Vec<String>,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
    /// Fatal structural errors.
    pub errors: Vec<String>,
}

/// JSON response for `csv validate` with `--json`.
#[derive(Debug, Serialize)]
pub struct ValidateResult {
    /// Per-file validation results.
    pub files: Vec<ValidateFileView>,
    /// Whether every file passed validation.
    pub valid: bool,
    /// Total data rows across all validated files.
    pub rows_total: usize,
}

struct ValidateTarget {
    role: CsvFileRole,
    path: PathBuf,
}

/// Validates structural integrity of configured CSV file(s).
pub fn validate(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let targets = resolve_targets(&validated.config, &validated.app, matches)?;
    let files = FileService::new()?;

    let mut views = Vec::with_capacity(targets.len());
    for target in targets {
        let read = validate_csv_file(&files, &target.path)?;
        views.push(ValidateFileView::from_target(target.role, &target.path, read));
    }

    let result = summarize_validation(views);
    print_validate_result(&result, json, quiet)
}

/// Validates both configured CSV files.
pub(crate) fn validate_all_configured_csv(
    config: &nest_config::ConfigService,
    app: &crate::config::AppConfig,
) -> ValidateResult {
    let files = match FileService::new() {
        Ok(files) => files,
        Err(_) => {
            return ValidateResult {
                files: Vec::new(),
                valid: false,
                rows_total: 0,
            };
        }
    };

    let targets = [
        (CsvFileRole::Location, resolve_config_path(config, &app.csv.location_data_file)),
        (CsvFileRole::Space, resolve_config_path(config, &app.csv.space_data_file)),
    ];

    let mut views = Vec::with_capacity(targets.len());
    for (role, path) in targets {
        let read = match validate_csv_file(&files, &path) {
            Ok(read) => read,
            Err(error) => CsvValidateReadResult {
                headers: Vec::new(),
                column_count: 0,
                row_count: 0,
                warnings: Vec::new(),
                errors: vec![error.message().to_string()],
            },
        };
        views.push(ValidateFileView::from_target(role, &path, read));
    }

    summarize_validation(views)
}

fn resolve_targets(
    config: &nest_config::ConfigService,
    app: &crate::config::AppConfig,
    matches: &ArgMatches,
) -> NestResult<Vec<ValidateTarget>> {
    if let Some(role_name) = matches.get_one::<String>("file") {
        let role = CsvFileRole::parse(role_name)?;
        return Ok(vec![ValidateTarget {
            role,
            path: resolve_csv_path(config, app, role),
        }]);
    }

    Ok(vec![
        ValidateTarget {
            role: CsvFileRole::Location,
            path: resolve_config_path(config, &app.csv.location_data_file),
        },
        ValidateTarget {
            role: CsvFileRole::Space,
            path: resolve_config_path(config, &app.csv.space_data_file),
        },
    ])
}

impl ValidateFileView {
    pub(crate) fn from_target(role: CsvFileRole, path: &Path, read: CsvValidateReadResult) -> Self {
        Self {
            role: role.as_str().to_string(),
            path: absolute_path(path),
            filename: csv_filename(path),
            valid: read.is_valid(),
            column_count: read.column_count,
            row_count: read.row_count,
            headers: read.headers,
            warnings: read.warnings,
            errors: read.errors,
        }
    }
}

pub(crate) fn summarize_validation(files: Vec<ValidateFileView>) -> ValidateResult {
    let rows_total = files.iter().map(|file| file.row_count).sum();
    let valid = files.iter().all(|file| file.valid);
    ValidateResult {
        files,
        valid,
        rows_total,
    }
}

fn print_validate_result(result: &ValidateResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize csv validate result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        print_validate_human(result);
    }

    if result.valid {
        return Ok(());
    }

    let messages: Vec<String> = result
        .files
        .iter()
        .flat_map(|file| {
            file.errors
                .iter()
                .map(move |error| format!("{}: {error}", file.filename))
        })
        .collect();

    Err(NestError::validation(format!(
        "CSV validation failed ({} error(s)): {}",
        messages.len(),
        messages.join("; ")
    )))
}

fn print_validate_human(result: &ValidateResult) {
    if result.valid {
        println!(
            "CSV validation passed ({} file(s), {} row(s) total).",
            result.files.len(),
            result.rows_total
        );
    } else {
        println!("CSV validation failed:");
    }

    for file in &result.files {
        let status = if file.valid { "ok" } else { "failed" };
        println!(
            "- {} ({}) — {status}, {} column(s), {} row(s)",
            file.role,
            file.path.display(),
            file.column_count,
            file.row_count
        );
        for warning in &file.warnings {
            println!("  warning: {warning}");
        }
        for error in &file.errors {
            println!("  error: {error}");
        }
    }
}
