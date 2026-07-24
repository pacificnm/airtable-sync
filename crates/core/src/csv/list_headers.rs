//! `csv list-headers` command: lists cached CSV column headers for editing UIs.

use std::collections::BTreeMap;
use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::csv::{csv_filename, resolve_csv_path, CsvFileRole};
use crate::db::{absolute_path, ensure_csv_cache, open_database, CsvStore};

/// One cached CSV column.
#[derive(Debug, Clone, Serialize)]
pub struct CsvHeaderView {
    /// Original header text.
    pub name: String,
    /// Lowercase normalized header, used for matching against Airtable field names.
    pub normalized_name: String,
}

/// Cached headers for one configured CSV file.
#[derive(Debug, Clone, Serialize)]
pub struct CsvFileHeadersView {
    /// Configured role (`location` or `space`) this file corresponds to.
    pub role: &'static str,
    /// Source file basename.
    pub filename: String,
    /// Cached columns, in cache order.
    pub columns: Vec<CsvHeaderView>,
}

/// JSON response for `csv list-headers` with `--json`.
#[derive(Debug, Serialize)]
pub struct CsvListHeadersResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Cached headers, one entry per file that has imported columns.
    pub files: Vec<CsvFileHeadersView>,
}

/// Lists cached CSV column headers for both configured files, for editing UIs
/// (e.g. a CSV file / CSV column picker). Read-only — requires a prior
/// `csv import-headers`.
pub fn list_headers(ctx: &AppContext) -> NestResult<()> {
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
    ensure_csv_cache(&database_path)?;

    let db = open_database(&database_path)?;
    let store = CsvStore::new(db);
    let fields = store.list_fields().map_err(NestError::from)?;

    let mut by_file: BTreeMap<String, Vec<CsvHeaderView>> = BTreeMap::new();
    for field in fields {
        by_file
            .entry(field.filename)
            .or_default()
            .push(CsvHeaderView {
                name: field.name,
                normalized_name: field.normalized_name,
            });
    }

    let location_filename = csv_filename(&resolve_csv_path(
        &validated.config,
        &validated.app,
        CsvFileRole::Location,
    ));
    let space_filename = csv_filename(&resolve_csv_path(
        &validated.config,
        &validated.app,
        CsvFileRole::Space,
    ));

    let files = by_file
        .into_iter()
        .map(|(filename, columns)| {
            let role = if filename == location_filename {
                "location"
            } else if filename == space_filename {
                "space"
            } else {
                "unknown"
            };
            CsvFileHeadersView {
                role,
                filename,
                columns,
            }
        })
        .collect();

    let result = CsvListHeadersResult {
        database_path: absolute_path(&database_path),
        files,
    };

    print_list_headers_success(&result, json, quiet)
}

fn print_list_headers_success(
    result: &CsvListHeadersResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize csv list-headers result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.files.is_empty() {
        println!("No CSV headers cached — run `csv import-headers` first.");
        return Ok(());
    }

    for file in &result.files {
        println!(
            "{} ({}): {} column(s)",
            file.filename,
            file.role,
            file.columns.len()
        );
        for column in &file.columns {
            println!("  {} ({})", column.name, column.normalized_name);
        }
    }

    Ok(())
}
