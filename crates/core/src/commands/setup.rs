//! `setup` command group with nested subcommands.

use clap::Command;
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::setup;

/// First-run setup and onboarding command group.
pub struct SetupCommand;

impl CliCommand for SetupCommand {
    fn name(&self) -> &'static str {
        "setup"
    }

    fn about(&self) -> &'static str {
        "First-run setup and onboarding"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "setup")
            .expect("setup command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd.subcommand_required(true).arg_required_else_help(true);

        for sub in spec.subcommands {
            cmd = cmd.subcommand(Command::new(sub.name).about(sub.about));
        }

        cmd
    }

    fn run(&self, ctx: &AppContext, matches: &clap::ArgMatches) -> NestResult<()> {
        let (subcommand, _) = matches
            .subcommand()
            .ok_or_else(|| NestError::command("missing setup subcommand"))?;

        match subcommand {
            "init" => setup::init(ctx),
            other => Err(NestError::command(format!(
                "setup subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
