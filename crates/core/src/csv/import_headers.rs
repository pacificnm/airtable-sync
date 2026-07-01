//! `csv import-headers` command handler.

use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::csv::common::csv_filename;
use crate::csv::headers::{read_csv_headers, CsvHeaderReadResult};
use crate::db::{
    absolute_path, ensure_csv_cache, open_database, CsvFieldRow, CsvStore,
};

/// One configured CSV file included in the import result.
#[derive(Debug, Serialize)]
pub struct ImportHeadersFileView {
    /// Absolute path to the CSV file.
    pub path: PathBuf,
    /// Config role (`location` or `space`).
    pub role: &'static str,
    /// Source file basename stored in SQLite.
    pub filename: String,
    /// Original header names read from the file.
    pub headers: Vec<String>,
}

/// One imported CSV field in JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportHeadersFieldView {
    /// Source CSV file name (basename).
    pub filename: String,
    /// Trimmed original header text.
    pub name: String,
    /// Lowercase normalized header.
    pub normalized_name: String,
}

/// JSON response for `csv import-headers` with `--json`.
#[derive(Debug, Serialize)]
pub struct ImportHeadersResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Per-file header reads.
    pub files: Vec<ImportHeadersFileView>,
    /// Number of fields stored across all files.
    pub fields_imported: usize,
    /// Non-fatal warnings from header parsing.
    pub warnings: Vec<String>,
    /// Fields stored in SQLite.
    pub fields: Vec<ImportHeadersFieldView>,
}

struct CsvSource<'a> {
    path: PathBuf,
    role: &'static str,
    read: &'a CsvHeaderReadResult,
}

/// Imports CSV column headers from configured files into SQLite.
pub fn import_headers(ctx: &AppContext) -> NestResult<()> {
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
    let location_path =
        resolve_config_path(&validated.config, &validated.app.csv.location_data_file);
    let space_path = resolve_config_path(&validated.config, &validated.app.csv.space_data_file);

    ensure_csv_cache(&database_path)?;

    let files = FileService::new()?;
    let location_read = read_csv_headers(&files, &location_path)?;
    let space_read = read_csv_headers(&files, &space_path)?;

    let sources = [
        CsvSource {
            path: location_path,
            role: "location",
            read: &location_read,
        },
        CsvSource {
            path: space_path,
            role: "space",
            read: &space_read,
        },
    ];

    let warnings = sources
        .iter()
        .flat_map(|source| source.read.warnings.iter().cloned())
        .collect::<Vec<_>>();

    let rows = sources
        .iter()
        .flat_map(|source| {
            let filename = csv_filename(&source.path);
            source.read.headers.iter().map(move |header| CsvFieldRow {
                filename: filename.clone(),
                name: header.name.clone(),
                normalized_name: header.normalized_name.clone(),
            })
        })
        .collect::<Vec<_>>();

    let db = open_database(&database_path)?;
    let fields_imported = CsvStore::new(db)
        .replace_fields(&rows)
        .map_err(NestError::from)?;

    let result = ImportHeadersResult {
        database_path: absolute_path(&database_path),
        files: sources
            .iter()
            .map(|source| ImportHeadersFileView {
                path: absolute_path(&source.path),
                role: source.role,
                filename: csv_filename(&source.path),
                headers: source
                    .read
                    .headers
                    .iter()
                    .map(|header| header.name.clone())
                    .collect(),
            })
            .collect(),
        fields_imported,
        warnings,
        fields: rows.into_iter().map(ImportHeadersFieldView::from).collect(),
    };

    print_import_headers_success(&result, json, quiet)
}

impl From<CsvFieldRow> for ImportHeadersFieldView {
    fn from(row: CsvFieldRow) -> Self {
        Self {
            filename: row.filename,
            name: row.name,
            normalized_name: row.normalized_name,
        }
    }
}

fn print_import_headers_success(
    result: &ImportHeadersResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize import-headers result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    for file in &result.files {
        println!(
            "Read {} header(s) from {} CSV ({}): {}",
            file.headers.len(),
            file.role,
            file.filename,
            file.path.display()
        );
    }

    if result.fields.is_empty() {
        println!("No CSV headers imported — check that the configured files have header rows.");
    } else {
        println!(
            "Imported {} CSV field(s) into {}:",
            result.fields_imported,
            result.database_path.display()
        );
        println!("{:<20} {:<30} {}", "filename", "name", "normalized_name");
        for field in &result.fields {
            println!(
                "{:<20} {:<30} {}",
                field.filename, field.name, field.normalized_name
            );
        }
    }

    for warning in &result.warnings {
        println!("warning: {warning}");
    }

    Ok(())
}
