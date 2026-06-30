//! `config` command group with nested subcommands.

use clap::Command;
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::config;

/// Configuration management command group.
pub struct ConfigCommand;

impl CliCommand for ConfigCommand {
    fn name(&self) -> &'static str {
        "config"
    }

    fn about(&self) -> &'static str {
        "Configuration management"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "config")
            .expect("config command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            cmd = cmd.subcommand(Command::new(sub.name).about(sub.about));
        }

        cmd
    }

    fn run(&self, ctx: &AppContext, matches: &clap::ArgMatches) -> NestResult<()> {
        let (subcommand, _sub_matches) = matches.subcommand().ok_or_else(|| {
            NestError::command("missing config subcommand")
        })?;

        match subcommand {
            "validate" => config::validate(ctx),
            "show" => config::show(ctx),
            "init" => Ok(()),
            other => Err(NestError::command(format!("unknown config subcommand: {other}"))),
        }
    }
}
