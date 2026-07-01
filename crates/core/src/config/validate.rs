//! Configuration validation for `config validate`.

use std::path::Path;

use nest_cli::CliGlobals;
use nest_config::ConfigService;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use nest_validation::codes::NEST_VALIDATION_FAILED;
use nest_validation::{ValidationError, ValidationIssue};

use crate::config::{resolve_config_path, AppConfig};

const LOG_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];

/// Validates the loaded application configuration.
pub fn validate(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let quiet = ctx
        .service::<CliGlobals>()
        .map(|globals| globals.quiet)
        .unwrap_or(false);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }

        let table_count = validated.app.airtable.tables.len();
        let sync_count = validated
            .app
            .airtable
            .tables
            .values()
            .filter(|table| table.sync)
            .count();
        println!("Configuration valid ({table_count} tables, {sync_count} enabled for sync)");
    }

    Ok(())
}

/// Loaded configuration that passed full validation.
pub struct ValidatedConfig {
    /// Configuration service for the loaded document.
    pub config: ConfigService,
    /// Parsed application configuration sections.
    pub app: AppConfig,
    /// Non-blocking validation warnings.
    pub warnings: Vec<ValidationIssue>,
}

/// Loads configuration and ensures it passes full validation.
pub fn ensure_valid_config(ctx: &AppContext) -> NestResult<ValidatedConfig> {
    let config = ctx.service::<ConfigService>()?;
    let app = AppConfig::from_service(&config)?;

    let issues = collect_validation_issues(&config, &app);
    let warnings: Vec<_> = issues
        .iter()
        .filter(|issue| !issue.is_blocking())
        .cloned()
        .collect();
    let blocking: Vec<_> = issues
        .into_iter()
        .filter(|issue| issue.is_blocking())
        .collect();

    if !blocking.is_empty() {
        return Err(fail_validation(blocking));
    }

    Ok(ValidatedConfig {
        config: config.clone(),
        app,
        warnings,
    })
}

/// Collects all semantic and path validation issues for the loaded configuration.
pub fn collect_validation_issues(config: &ConfigService, app: &AppConfig) -> Vec<ValidationIssue> {
    let mut issues = semantic_issues(app);
    issues.extend(path_issues(config, app));
    issues
}

/// Builds a validation error with field-level details for blocking issues.
pub fn fail_validation(issues: Vec<ValidationIssue>) -> NestError {
    let validation_error = ValidationError::from_issues_strict(issues.clone()).unwrap_err();
    let lines: Vec<String> = issues.iter().map(format_issue_line).collect();
    let message = if lines.len() == 1 {
        lines[0].clone()
    } else {
        format!("{} validation errors:\n{}", lines.len(), lines.join("\n"))
    };

    NestError::validation(message)
        .with_code(NEST_VALIDATION_FAILED)
        .with_module("airtable-sync")
        .with_source(validation_error)
}

/// Prints a non-blocking validation warning to stdout.
pub fn print_warning(issue: &ValidationIssue) {
    if let Some(field) = &issue.field {
        println!("warning: {}: {}", field, issue.message);
    } else {
        println!("warning: {}", issue.message);
    }
}

fn semantic_issues(app: &AppConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if !app.airtable.token_present() {
        issues.push(
            ValidationIssue::field_error(
                "airtable.token",
                "config.airtable.token.missing",
                "Airtable token is required",
            )
            .with_help(
                "Set `token` in [airtable] or provide a value in `token_env` inside config.toml.",
            ),
        );
    }

    if app.airtable.base_id.trim().is_empty() {
        issues.push(ValidationIssue::field_error(
            "airtable.base_id",
            "config.airtable.base_id.required",
            "Airtable base_id is required",
        ));
    } else if !app.airtable.base_id.starts_with("app") {
        issues.push(
            ValidationIssue::field_error(
                "airtable.base_id",
                "config.airtable.base_id.format",
                "Airtable base_id should start with \"app\"",
            )
            .with_help("Use the base ID from your Airtable URL or API settings."),
        );
    }

    if app.airtable.tables.is_empty() {
        issues.push(
            ValidationIssue::field_error(
                "airtable.tables",
                "config.airtable.tables.required",
                "At least one [airtable.tables.<name>] section is required",
            )
            .with_help("Add a table block with table_id for each logical table name."),
        );
    }

    for (name, table) in &app.airtable.tables {
        let field_prefix = format!("airtable.tables.{name}.table_id");
        if table.table_id.trim().is_empty() {
            issues.push(ValidationIssue::field_error(
                field_prefix,
                "config.airtable.tables.table_id.required",
                format!("table_id is required for table \"{name}\""),
            ));
        } else if !table.table_id.starts_with("tbl") {
            issues.push(ValidationIssue::field_error(
                field_prefix,
                "config.airtable.tables.table_id.format",
                format!("table_id for \"{name}\" should start with \"tbl\""),
            ));
        }
    }

    if app.sync.max_parallel_tables == 0 {
        issues.push(ValidationIssue::field_error(
            "sync.max_parallel_tables",
            "config.sync.max_parallel_tables.range",
            "max_parallel_tables must be at least 1",
        ));
    }

    if app.sync.max_parallel_updates == 0 {
        issues.push(ValidationIssue::field_error(
            "sync.max_parallel_updates",
            "config.sync.max_parallel_updates.range",
            "max_parallel_updates must be at least 1",
        ));
    }

    if app.database.provider != "sqlite" {
        issues.push(
            ValidationIssue::field_error(
                "database.provider",
                "config.database.provider.unsupported",
                format!("Unsupported database provider: {}", app.database.provider),
            )
            .with_help("Only \"sqlite\" is supported in this version."),
        );
    }

    let level = app.logging.level.trim().to_ascii_lowercase();
    if !LOG_LEVELS.contains(&level.as_str()) {
        issues.push(
            ValidationIssue::field_error(
                "logging.level",
                "config.logging.level.invalid",
                format!("Invalid log level: {}", app.logging.level),
            )
            .with_help("Use one of: trace, debug, info, warn, error."),
        );
    }

    let sync_enabled = app.airtable.tables.values().any(|table| table.sync);
    if !sync_enabled {
        issues.push(
            ValidationIssue::field_warning(
                "airtable.tables",
                "config.airtable.tables.sync.none",
                "No tables are enabled for sync",
            )
            .with_help("Set sync = true on tables you want included in sync all."),
        );
    }

    issues
}

fn path_issues(config: &ConfigService, app: &AppConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    check_file_exists(
        &mut issues,
        "csv.location_data_file",
        "config.csv.location_data_file.missing",
        &resolve_config_path(config, &app.csv.location_data_file),
        "Location CSV file not found",
    );
    check_file_exists(
        &mut issues,
        "csv.space_data_file",
        "config.csv.space_data_file.missing",
        &resolve_config_path(config, &app.csv.space_data_file),
        "Space CSV file not found",
    );
    check_file_exists(
        &mut issues,
        "database.schema",
        "config.database.schema.missing",
        &resolve_config_path(config, &app.database.schema),
        "Database schema file not found",
    );
    check_parent_exists(
        &mut issues,
        "database.database_path",
        "config.database.database_path.parent_missing",
        &resolve_config_path(config, &app.database.database_path),
        "Parent directory for database_path does not exist",
    );
    check_parent_exists(
        &mut issues,
        "logging.directory",
        "config.logging.directory.parent_missing",
        &resolve_config_path(config, &app.logging.directory),
        "Parent directory for logging.directory does not exist",
    );

    issues
}

fn check_file_exists(
    issues: &mut Vec<ValidationIssue>,
    field: &str,
    code: &str,
    path: &Path,
    message: &str,
) {
    if path.is_file() {
        return;
    }

    issues.push(
        ValidationIssue::field_error(field, code, message)
            .with_help(format!("Expected file at {}", path.display())),
    );
}

fn check_parent_exists(
    issues: &mut Vec<ValidationIssue>,
    field: &str,
    code: &str,
    path: &Path,
    message: &str,
) {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty());
    if parent.is_some_and(|dir| dir.is_dir()) || path.is_dir() {
        return;
    }

    issues.push(
        ValidationIssue::field_error(field, code, message)
            .with_help(format!("Expected directory at {}", path.display())),
    );
}

fn format_issue_line(issue: &ValidationIssue) -> String {
    let field = issue
        .field
        .as_ref()
        .map(|path| path.as_str())
        .unwrap_or("config");

    match &issue.help {
        Some(help) => format!("  {field}: {} ({help})", issue.message),
        None => format!("  {field}: {}", issue.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AirtableSection, AirtableTableEntry, CsvSection, DatabaseSection, LoggingSection,
        SyncSection,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn sample_app() -> AppConfig {
        AppConfig {
            airtable: AirtableSection {
                api_url: None,
                token: Some("pat-test".to_string()),
                token_env: None,
                base_id: "appTEST".to_string(),
                tables: HashMap::from([(
                    "assets".to_string(),
                    AirtableTableEntry {
                        table_id: "tblTEST".to_string(),
                        sync: true,
                        primary_key_field: None,
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

    #[test]
    fn semantic_validation_accepts_valid_config() {
        let issues: Vec<_> = semantic_issues(&sample_app())
            .into_iter()
            .filter(|issue| issue.is_blocking())
            .collect();
        assert!(issues.is_empty());
    }

    #[test]
    fn semantic_validation_requires_token() {
        let mut app = sample_app();
        app.airtable.token = None;
        let issues = semantic_issues(&app);
        assert!(issues.iter().any(|issue| {
            issue
                .field
                .as_ref()
                .is_some_and(|field| field.as_str() == "airtable.token")
        }));
    }

    #[test]
    fn fail_validation_includes_field_details() {
        let issues = vec![ValidationIssue::field_error(
            "csv.location_data_file",
            "config.csv.location_data_file.missing",
            "Location CSV file not found",
        )
        .with_help("Expected file at /tmp/location.csv")];

        let error = fail_validation(issues);
        assert!(error.message().contains("csv.location_data_file"));
        assert!(error.message().contains("/tmp/location.csv"));
    }
}
