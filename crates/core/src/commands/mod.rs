//! CLI command groups.

mod config;
mod group;
mod spec;

pub use config::ConfigCommand;
pub use group::GroupCommand;
pub use spec::{CommandGroupSpec, SubcommandSpec, COMMAND_GROUPS};

/// Registers the full CLI command tree on the Nest CLI host.
pub fn register_commands(mut app: nest_cli::CliApp) -> nest_cli::CliApp {
    for spec in COMMAND_GROUPS {
        if spec.name == "config" {
            app = app.command(ConfigCommand);
        } else {
            app = app.command(GroupCommand(spec));
        }
    }
    app
}
