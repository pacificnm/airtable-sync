//! `version` top-level command.

use clap::Command;
use nest_cli::{CliCommand, CliHostInfo};
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

/// Displays application version information.
pub struct VersionCommand;

impl CliCommand for VersionCommand {
    fn name(&self) -> &'static str {
        "version"
    }

    fn about(&self) -> &'static str {
        "Display version information"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd
    }

    fn run(&self, ctx: &AppContext, _matches: &clap::ArgMatches) -> NestResult<()> {
        let host = ctx.service::<CliHostInfo>()?;
        let version = host.version.as_deref().ok_or_else(|| {
            NestError::command("application version is not configured")
                .with_help("Pass a version to `CliApp::with_version` when building the CLI host.")
        })?;

        println!("{version}");
        Ok(())
    }
}
