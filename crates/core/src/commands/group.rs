//! Command group placeholders with nested subcommand help.

use clap::Command;
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::NestResult;

use crate::commands::spec::CommandGroupSpec;

/// Placeholder for a top-level command group (and its nested subcommands in help).
#[derive(Clone, Copy)]
pub struct GroupCommand(pub &'static CommandGroupSpec);

impl CliCommand for GroupCommand {
    fn name(&self) -> &'static str {
        self.0.name
    }

    fn about(&self) -> &'static str {
        self.0.about
    }

    fn configure(&self, cmd: Command) -> Command {
        if self.0.subcommands.is_empty() {
            return cmd;
        }

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in self.0.subcommands {
            cmd = cmd.subcommand(Command::new(sub.name).about(sub.about));
        }

        cmd
    }

    fn run(&self, _ctx: &AppContext, _matches: &clap::ArgMatches) -> NestResult<()> {
        Ok(())
    }
}
