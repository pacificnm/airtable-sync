//! `csv preview` command handler.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning};
use crate::csv::common::{csv_filename, resolve_csv_path, CsvFileRole};
use crate::csv::preview_reader::read_csv_preview;
use crate::db::absolute_path;

const MAX_LIMIT: usize = 100;

/// JSON response for `csv preview` with `--json`.
#[derive(Debug, Serialize)]
pub struct PreviewResult {
    /// Config role (`location` or `space`).
    pub role: String,
    /// Absolute path to the CSV file.
    pub path: PathBuf,
    /// Source file basename.
    pub filename: String,
    /// Requested row limit.
    pub limit: usize,
    /// Whether more rows exist beyond the preview.
    pub truncated: bool,
    /// Normalized header names in file order.
    pub headers: Vec<String>,
    /// Previewed data rows.
    pub rows: Vec<BTreeMap<String, String>>,
    /// Non-fatal warnings from parsing.
    pub warnings: Vec<String>,
}

/// Previews sample rows from one configured CSV file.
pub fn preview(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let role_name = matches
        .get_one::<String>("file")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing CSV file role (location or space)"))?;
    let role = CsvFileRole::parse(role_name)?;
    let limit = parse_limit(matches)?;

    let path = resolve_csv_path(&validated.config, &validated.app, role);
    let files = FileService::new()?;
    let preview = read_csv_preview(&files, &path, limit)?;

    let rows = preview
        .rows
        .iter()
        .map(|values| row_to_map(&preview.headers, values))
        .collect();

    let result = PreviewResult {
        role: role.as_str().to_string(),
        path: absolute_path(&path),
        filename: csv_filename(&path),
        limit,
        truncated: preview.truncated,
        headers: preview.headers.clone(),
        rows,
        warnings: preview.warnings,
    };

    print_preview_success(&result, json, quiet)
}

fn parse_limit(matches: &ArgMatches) -> NestResult<usize> {
    let raw = matches
        .get_one::<String>("limit")
        .map(String::as_str)
        .unwrap_or("5");
    let limit: usize = raw.parse().map_err(|_| {
        NestError::command(format!("invalid --limit value `{raw}` (expected a positive integer)"))
    })?;
    if limit == 0 {
        return Err(NestError::command("--limit must be at least 1"));
    }
    if limit > MAX_LIMIT {
        return Err(NestError::command(format!(
            "--limit must be at most {MAX_LIMIT}"
        )));
    }
    Ok(limit)
}

fn row_to_map(headers: &[String], values: &[String]) -> BTreeMap<String, String> {
    headers
        .iter()
        .zip(values.iter())
        .map(|(header, value)| (header.clone(), value.clone()))
        .collect()
}

fn print_preview_success(result: &PreviewResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize csv preview result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    println!(
        "Preview of {} CSV ({}) — limit {} row(s){}:",
        result.role,
        result.path.display(),
        result.limit,
        if result.truncated {
            " (truncated)"
        } else {
            ""
        }
    );

    if result.headers.is_empty() {
        println!("No headers found — check that the file has a header row.");
        return Ok(());
    }

    let widths = column_widths(&result.headers, &result.rows);
    print_row(&result.headers, &widths);
    for row in &result.rows {
        let values: Vec<String> = result
            .headers
            .iter()
            .map(|header| row.get(header).cloned().unwrap_or_default())
            .collect();
        print_row(&values, &widths);
    }

    if result.truncated {
        println!(
            "Showing {} of more rows — increase --limit to preview more.",
            result.rows.len()
        );
    }

    for warning in &result.warnings {
        println!("warning: {warning}");
    }

    Ok(())
}

fn column_widths(headers: &[String], rows: &[BTreeMap<String, String>]) -> Vec<usize> {
    headers
        .iter()
        .map(|header| {
            let mut width = header.len();
            for row in rows {
                let value_len = row.get(header).map(String::len).unwrap_or(0);
                width = width.max(value_len);
            }
            width
        })
        .collect()
}

fn print_row(values: &[String], widths: &[usize]) {
    let line = values
        .iter()
        .zip(widths.iter())
        .map(|(value, width)| format!("{value:<width$}"))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_LIMIT: usize = 5;

    #[test]
    fn parse_limit_defaults_and_validates() {
        let cmd = clap::Command::new("preview").arg(
            clap::Arg::new("limit")
                .long("limit")
                .default_value("5"),
        );
        let matches = cmd.clone().try_get_matches_from(["preview"]).unwrap();
        assert_eq!(parse_limit(&matches).unwrap(), DEFAULT_LIMIT);

        let matches = cmd
            .clone()
            .try_get_matches_from(["preview", "--limit", "10"])
            .unwrap();
        assert_eq!(parse_limit(&matches).unwrap(), 10);

        let matches = cmd
            .try_get_matches_from(["preview", "--limit", "0"])
            .unwrap();
        assert!(parse_limit(&matches).is_err());
    }
}
