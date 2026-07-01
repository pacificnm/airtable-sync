//! Shared helpers for CSV commands.

use std::path::{Path, PathBuf};

use nest_config::ConfigService;
use nest_error::{NestError, NestResult};

use crate::config::{resolve_config_path, AppConfig};

/// Configured CSV file role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvFileRole {
    /// Location data CSV (`[csv].location_data_file`).
    Location,
    /// Space data CSV (`[csv].space_data_file`).
    Space,
}

impl CsvFileRole {
    /// Parses a role name from CLI input.
    pub fn parse(value: &str) -> NestResult<Self> {
        match value {
            "location" => Ok(Self::Location),
            "space" => Ok(Self::Space),
            other => Err(NestError::command(format!(
                "unknown CSV file role `{other}` (expected `location` or `space`)"
            ))),
        }
    }

    /// Returns the stable role label used in output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Location => "location",
            Self::Space => "space",
        }
    }
}

/// Returns the CSV file basename for display and SQLite storage.
pub fn csv_filename(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown.csv")
        .to_string()
}

/// Resolves the configured CSV path for a role.
pub fn resolve_csv_path(
    config: &ConfigService,
    app: &AppConfig,
    role: CsvFileRole,
) -> PathBuf {
    let path = match role {
        CsvFileRole::Location => &app.csv.location_data_file,
        CsvFileRole::Space => &app.csv.space_data_file,
    };
    resolve_config_path(config, path)
}

/// Resolves a configured CSV path from an imported file basename.
pub fn resolve_csv_path_by_filename(
    config: &ConfigService,
    app: &AppConfig,
    filename: &str,
) -> NestResult<PathBuf> {
    let location = resolve_csv_path(config, app, CsvFileRole::Location);
    let space = resolve_csv_path(config, app, CsvFileRole::Space);

    if csv_filename(&location) == filename {
        return Ok(location);
    }
    if csv_filename(&space) == filename {
        return Ok(space);
    }

    Err(NestError::validation(format!(
        "CSV file `{filename}` from mapping does not match configured location or space CSV paths"
    ))
    .with_help("Re-run `csv import-headers` after updating [csv] paths in config.toml."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_filename_uses_basename() {
        assert_eq!(
            csv_filename(Path::new("/data/location_current_v.csv")),
            "location_current_v.csv"
        );
        assert_eq!(csv_filename(Path::new("space.csv")), "space.csv");
    }

    #[test]
    fn parse_role_accepts_location_and_space() {
        assert_eq!(CsvFileRole::parse("location").unwrap(), CsvFileRole::Location);
        assert_eq!(CsvFileRole::parse("space").unwrap(), CsvFileRole::Space);
        assert!(CsvFileRole::parse("missing").is_err());
    }
}
