//! Configuration display for `config show`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_config::ConfigService;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::validate::{ensure_valid_config, print_warning};
use crate::config::{resolve_config_path, AppConfig};

const REDACTED: &str = "(set)";
const NOT_SET: &str = "(not set)";

/// Displays the loaded configuration after validation succeeds.
pub fn show(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    if json {
        let view = build_show_view(&validated.config, &validated.app);
        let payload = serde_json::to_string_pretty(&view).map_err(|error| {
            NestError::data(format!("failed to serialize configuration: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        let view = build_show_view(&validated.config, &validated.app);
        println!("{}", format_show_human(&view));
    }

    Ok(())
}

/// Redacted configuration view for display and JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigShowView {
    /// Resolved configuration file path, if loaded from disk.
    pub config_path: Option<PathBuf>,
    /// Airtable connection settings.
    pub airtable: AirtableShowView,
    /// Global synchronization settings.
    pub sync: SyncShowView,
    /// CSV input paths (resolved).
    pub csv: CsvShowView,
    /// Database settings (resolved paths).
    pub database: DatabaseShowView,
    /// Logging settings (resolved paths).
    pub logging: LoggingShowView,
}

/// Redacted `[airtable]` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AirtableShowView {
    /// API base URL.
    pub api_url: String,
    /// Redacted token status.
    pub token: String,
    /// Token env var name or redacted secret value.
    pub token_env: Option<String>,
    /// Airtable base ID.
    pub base_id: String,
    /// Table definitions sorted by name.
    pub tables: BTreeMap<String, TableShowView>,
}

/// Redacted table entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableShowView {
    /// Airtable table ID.
    pub table_id: String,
    /// Whether sync is enabled.
    pub sync: bool,
    /// Optional primary key field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key_field: Option<String>,
}

/// `[sync]` section for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncShowView {
    /// Dry-run mode.
    pub dry_run: bool,
    /// Continue on error.
    pub continue_on_error: bool,
    /// Max parallel tables.
    pub max_parallel_tables: u32,
    /// Max parallel updates.
    pub max_parallel_updates: u32,
    /// Create change plan.
    pub create_change_plan: bool,
}

/// `[csv]` section with resolved paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvShowView {
    /// Resolved location CSV path.
    pub location_data_file: PathBuf,
    /// Resolved space CSV path.
    pub space_data_file: PathBuf,
}

/// `[database]` section with resolved paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatabaseShowView {
    /// Database provider.
    pub provider: String,
    /// Resolved database file path.
    pub database_path: PathBuf,
    /// Resolved schema file path.
    pub schema: PathBuf,
}

/// `[logging]` section with resolved paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoggingShowView {
    /// Log level.
    pub level: String,
    /// Resolved log directory.
    pub directory: PathBuf,
}

/// Builds a redacted display view from the loaded configuration.
pub fn build_show_view(config: &ConfigService, app: &AppConfig) -> ConfigShowView {
    let api_url = app
        .airtable
        .api_url
        .clone()
        .unwrap_or_else(|| "https://api.airtable.com/v0".to_string());

    let mut tables = BTreeMap::new();
    for (name, table) in &app.airtable.tables {
        tables.insert(
            name.clone(),
            TableShowView {
                table_id: table.table_id.clone(),
                sync: table.sync,
                primary_key_field: table.primary_key_field.clone(),
            },
        );
    }

    ConfigShowView {
        config_path: config.path().map(PathBuf::from),
        airtable: AirtableShowView {
            api_url,
            token: redact_optional_secret(app.airtable.token.as_deref()),
            token_env: app
                .airtable
                .token_env
                .as_deref()
                .map(redact_token_env),
            base_id: app.airtable.base_id.clone(),
            tables,
        },
        sync: SyncShowView {
            dry_run: app.sync.dry_run,
            continue_on_error: app.sync.continue_on_error,
            max_parallel_tables: app.sync.max_parallel_tables,
            max_parallel_updates: app.sync.max_parallel_updates,
            create_change_plan: app.sync.create_change_plan,
        },
        csv: CsvShowView {
            location_data_file: resolve_config_path(config, &app.csv.location_data_file),
            space_data_file: resolve_config_path(config, &app.csv.space_data_file),
        },
        database: DatabaseShowView {
            provider: app.database.provider.clone(),
            database_path: resolve_config_path(config, &app.database.database_path),
            schema: resolve_config_path(config, &app.database.schema),
        },
        logging: LoggingShowView {
            level: app.logging.level.clone(),
            directory: resolve_config_path(config, &app.logging.directory),
        },
    }
}

/// Formats the display view as human-readable sectioned text.
pub fn format_show_human(view: &ConfigShowView) -> String {
    let mut output = String::new();

    match &view.config_path {
        Some(path) => output.push_str(&format!("Configuration: {}\n\n", path.display())),
        None => output.push_str("Configuration: (in memory)\n\n"),
    }

    output.push_str("[airtable]\n");
    output.push_str(&format!("  api_url = {}\n", view.airtable.api_url));
    output.push_str(&format!("  token = {}\n", view.airtable.token));
    if let Some(token_env) = &view.airtable.token_env {
        output.push_str(&format!("  token_env = {token_env}\n"));
    }
    output.push_str(&format!("  base_id = {}\n", view.airtable.base_id));

    for (name, table) in &view.airtable.tables {
        output.push_str(&format!("\n[airtable.tables.{name}]\n"));
        output.push_str(&format!("  table_id = {}\n", table.table_id));
        output.push_str(&format!("  sync = {}\n", table.sync));
        if let Some(primary_key_field) = &table.primary_key_field {
            output.push_str(&format!("  primary_key_field = {primary_key_field}\n"));
        }
    }

    output.push_str("\n[sync]\n");
    output.push_str(&format!("  dry_run = {}\n", view.sync.dry_run));
    output.push_str(&format!(
        "  continue_on_error = {}\n",
        view.sync.continue_on_error
    ));
    output.push_str(&format!(
        "  max_parallel_tables = {}\n",
        view.sync.max_parallel_tables
    ));
    output.push_str(&format!(
        "  max_parallel_updates = {}\n",
        view.sync.max_parallel_updates
    ));
    output.push_str(&format!(
        "  create_change_plan = {}\n",
        view.sync.create_change_plan
    ));

    output.push_str("\n[csv]\n");
    output.push_str(&format!(
        "  location_data_file = {}\n",
        view.csv.location_data_file.display()
    ));
    output.push_str(&format!(
        "  space_data_file = {}\n",
        view.csv.space_data_file.display()
    ));

    output.push_str("\n[database]\n");
    output.push_str(&format!("  provider = {}\n", view.database.provider));
    output.push_str(&format!(
        "  database_path = {}\n",
        view.database.database_path.display()
    ));
    output.push_str(&format!("  schema = {}\n", view.database.schema.display()));

    output.push_str("\n[logging]\n");
    output.push_str(&format!("  level = {}\n", view.logging.level));
    output.push_str(&format!(
        "  directory = {}\n",
        view.logging.directory.display()
    ));

    output
}

fn redact_optional_secret(value: Option<&str>) -> String {
    match value.filter(|value| !value.trim().is_empty()) {
        Some(_) => REDACTED.to_string(),
        None => NOT_SET.to_string(),
    }
}

fn redact_token_env(value: &str) -> String {
    if looks_like_secret(value) {
        REDACTED.to_string()
    } else {
        value.to_string()
    }
}

fn looks_like_secret(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }

    value.starts_with("pat")
        || value.starts_with("key")
        || value.starts_with("crsr_")
        || value.len() > 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AirtableSection, AirtableTableEntry, CsvSection, DatabaseSection, LoggingSection,
        SyncSection,
    };
    use nest_config::{ConfigDocument, ConfigService, ConfigSource, LoadedConfig};
    use std::collections::HashMap;

    fn sample_app() -> AppConfig {
        AppConfig {
            airtable: AirtableSection {
                api_url: Some("https://api.airtable.com/v0".to_string()),
                meta_api_url: None,
                token: Some("pat-secret-token".to_string()),
                token_env: None,
                base_id: "appTEST".to_string(),
                tables: HashMap::from([(
                    "assets".to_string(),
                    AirtableTableEntry {
                        table_id: "tblTEST".to_string(),
                        sync: true,
                        primary_key_field: Some("Name".to_string()),
                    },
                )]),
            },
            sync: SyncSection {
                dry_run: true,
                continue_on_error: true,
                max_parallel_tables: 2,
                max_parallel_updates: 5,
                create_change_plan: true,
            },
            csv: CsvSection {
                location_data_file: PathBuf::from("location.csv"),
                space_data_file: PathBuf::from("space.csv"),
            },
            database: DatabaseSection {
                provider: "sqlite".to_string(),
                database_path: PathBuf::from("data/app.db"),
                schema: PathBuf::from("schema.sql"),
            },
            logging: LoggingSection {
                level: "info".to_string(),
                directory: PathBuf::from("logs"),
            },
        }
    }

    fn sample_config_service() -> ConfigService {
        ConfigService::new(LoadedConfig {
            document: ConfigDocument::empty(),
            source: ConfigSource::Memory(ConfigDocument::empty()),
            path: Some(PathBuf::from("/tmp/config.toml")),
        })
    }

    #[test]
    fn redacts_token_and_literal_token_env() {
        assert_eq!(redact_optional_secret(Some("pat-secret")), REDACTED);
        assert_eq!(redact_optional_secret(None), NOT_SET);
        assert_eq!(redact_token_env("AIRTABLE_TOKEN"), "AIRTABLE_TOKEN");
        assert_eq!(redact_token_env("pat-secret"), REDACTED);
        assert_eq!(redact_token_env("crsr_abc123"), REDACTED);
    }

    #[test]
    fn human_output_redacts_secrets_and_includes_sections() {
        let view = build_show_view(&sample_config_service(), &sample_app());
        let output = format_show_human(&view);

        assert!(output.contains("base_id = appTEST"));
        assert!(output.contains("[airtable.tables.assets]"));
        assert!(output.contains("token = (set)"));
        assert!(!output.contains("pat-secret-token"));
    }

    #[test]
    fn json_output_redacts_secrets() {
        let view = build_show_view(&sample_config_service(), &sample_app());
        let json = serde_json::to_string(&view).unwrap();

        assert!(json.contains("\"token\":\"(set)\""));
        assert!(!json.contains("pat-secret-token"));
    }
}
