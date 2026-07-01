//! Application configuration sections loaded from `config.toml`.

mod init;
mod show;
mod validate;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nest_config::ConfigService;
use nest_error::NestResult;
use serde::Deserialize;

pub use init::init;
pub use show::show;
pub use validate::{ensure_valid_config, print_warning, validate, ValidatedConfig};

/// Per-table configuration under `[airtable.tables.<name>]`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AirtableTableEntry {
    /// Airtable table ID (`tbl…`).
    pub table_id: String,
    /// When true, included in bulk sync and compare operations.
    #[serde(default)]
    pub sync: bool,
    /// Optional primary key field name for compare/sync.
    #[serde(default)]
    pub primary_key_field: Option<String>,
}

/// `[airtable]` configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AirtableSection {
    /// Airtable REST API base URL.
    #[serde(default)]
    pub api_url: Option<String>,
    /// Airtable Meta API base URL (default derived by nest-airtable if omitted).
    #[serde(default)]
    pub meta_api_url: Option<String>,
    /// Personal access token stored directly in gitignored `config.toml`.
    #[serde(default)]
    pub token: Option<String>,
    /// Environment variable name for the token, or legacy direct token value.
    #[serde(default)]
    pub token_env: Option<String>,
    /// Airtable base ID (`app…`).
    pub base_id: String,
    /// Logical table name → table configuration.
    #[serde(default)]
    pub tables: HashMap<String, AirtableTableEntry>,
}

impl AirtableSection {
    /// Returns whether credentials are present in the configuration file.
    pub fn token_present(&self) -> bool {
        self.token
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || self
                .token_env
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

/// `[sync]` configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SyncSection {
    /// When true, sync commands plan changes without writing to Airtable.
    #[serde(default = "default_true")]
    pub dry_run: bool,
    /// When true, keep processing after non-fatal errors.
    #[serde(default = "default_true")]
    pub continue_on_error: bool,
    /// Maximum tables processed concurrently.
    #[serde(default = "default_max_parallel_tables")]
    pub max_parallel_tables: u32,
    /// Maximum concurrent update operations per table.
    #[serde(default = "default_max_parallel_updates")]
    pub max_parallel_updates: u32,
    /// When true, persist a change plan before apply.
    #[serde(default = "default_true")]
    pub create_change_plan: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_parallel_tables() -> u32 {
    2
}

fn default_max_parallel_updates() -> u32 {
    5
}

/// `[csv]` configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CsvSection {
    /// Path to the location CSV file.
    pub location_data_file: PathBuf,
    /// Path to the space CSV file.
    pub space_data_file: PathBuf,
}

/// `[database]` configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DatabaseSection {
    /// Database backend (`sqlite`).
    pub provider: String,
    /// Path to the SQLite database file.
    pub database_path: PathBuf,
    /// Path to the SQL schema file applied by `db init`.
    pub schema: PathBuf,
}

/// `[logging]` configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LoggingSection {
    /// Log level (`trace`, `debug`, `info`, `warn`, `error`).
    pub level: String,
    /// Directory for log files.
    pub directory: PathBuf,
}

/// Fully loaded application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// Airtable connection and table definitions.
    pub airtable: AirtableSection,
    /// Global synchronization behavior.
    pub sync: SyncSection,
    /// CSV input file paths.
    pub csv: CsvSection,
    /// Local SQLite database settings.
    pub database: DatabaseSection,
    /// Logging settings.
    pub logging: LoggingSection,
}

impl AppConfig {
    /// Loads all required configuration sections from a [`ConfigService`].
    pub fn from_service(config: &ConfigService) -> NestResult<Self> {
        Ok(Self {
            airtable: config.section("airtable")?,
            sync: config.section("sync")?,
            csv: config.section("csv")?,
            database: config.section("database")?,
            logging: config.section("logging")?,
        })
    }
}

/// Resolves a config path relative to the configuration file directory.
pub fn resolve_config_path(config: &ConfigService, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let base = config
        .path()
        .and_then(|config_path| config_path.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

    base.join(path)
}
