//! CLI host wiring for Airtable Sync.

use nest_cli::{CliApp, LoggingConfig};

use crate::commands::register_commands;

const LONG_ABOUT: &str = "Airtable Sync\n\nSynchronize Airtable tables from LDP CSV exports.";

/// Builds the Airtable Sync CLI host with Milestone 1 placeholder command groups.
pub fn cli_app() -> CliApp {
    register_commands(
        CliApp::new("airtable-sync")
            .with_long_about(LONG_ABOUT)
            .with_logging(LoggingConfig::for_cli("airtable-sync"))
            .with_log_level_from_args(true),
    )
}

/// Renders the long `--help` text for the CLI (used by tests).
pub fn cli_help_text() -> nest_error::NestResult<String> {
    cli_app().render_long_help()
}

/// Renders the long `--help` text for a top-level command group.
pub fn group_help_text(group: &str) -> nest_error::NestResult<String> {
    cli_app().render_group_long_help(group)
}
