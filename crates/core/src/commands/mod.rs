//! CLI command groups.

mod airtable;
mod compare;
mod config;
mod csv;
mod db;
mod group;
mod mapping;
mod report;
mod spec;
mod sync;
mod version;

pub use airtable::AirtableCommand;
pub use compare::CompareCommand;
pub use config::ConfigCommand;
pub use csv::CsvCommand;
pub use db::DbCommand;
pub use group::GroupCommand;
pub use mapping::MappingCommand;
pub use report::ReportCommand;
pub use spec::{CommandGroupSpec, SubcommandSpec, COMMAND_GROUPS};
pub use sync::SyncCommand;
pub use version::VersionCommand;

/// Registers the full CLI command tree on the Nest CLI host.
pub fn register_commands(mut app: nest_cli::CliApp) -> nest_cli::CliApp {
    for spec in COMMAND_GROUPS {
        if spec.name == "config" {
            app = app.command(ConfigCommand);
        } else if spec.name == "db" {
            app = app.command(DbCommand);
        } else if spec.name == "airtable" {
            app = app.command(AirtableCommand);
        } else if spec.name == "csv" {
            app = app.command(CsvCommand);
        } else if spec.name == "mapping" {
            app = app.command(MappingCommand);
        } else if spec.name == "compare" {
            app = app.command(CompareCommand);
        } else if spec.name == "sync" {
            app = app.command(SyncCommand);
        } else if spec.name == "report" {
            app = app.command(ReportCommand);
        } else if spec.name == "version" {
            app = app.command(VersionCommand);
        } else {
            app = app.command(GroupCommand(spec));
        }
    }
    app
}
