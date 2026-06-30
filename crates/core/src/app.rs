//! CLI host wiring for Airtable Sync.

use nest_airtable::AirtableModule;
use nest_cli::CliApp;
use nest_http_client::HttpClientModule;
use nest_logging::LoggingConfig;

use crate::commands::{ListCommand, TablesCommand};

/// Builds the Airtable Sync CLI host with modules and commands registered.
pub fn cli_app() -> CliApp {
    CliApp::new("airtable-sync")
        .with_logging(LoggingConfig::for_cli("airtable-sync"))
        .with_log_level_from_args(true)
        .module(HttpClientModule::default())
        .module(AirtableModule::new())
        .async_command(ListCommand)
        .command(TablesCommand)
}
