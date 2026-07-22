//! `compare` command group with nested subcommands.

use clap::{Arg, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::compare;

/// Compare CSV to Airtable command group.
pub struct CompareCommand;

impl CliCommand for CompareCommand {
    fn name(&self) -> &'static str {
        "compare"
    }

    fn about(&self) -> &'static str {
        "Compare CSV to Airtable"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "compare")
            .expect("compare command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = if sub.name == "table" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("table")
                            .required(true)
                            .help("Logical table name from config (see `airtable list-tables`)"),
                    )
            } else {
                Command::new(sub.name).about(sub.about)
            };
            cmd = cmd.subcommand(sub_cmd);
        }

        cmd
    }

    fn run(&self, ctx: &AppContext, matches: &clap::ArgMatches) -> NestResult<()> {
        let (subcommand, sub_matches) = matches.subcommand().ok_or_else(|| {
            NestError::command("missing compare subcommand")
        })?;

        match subcommand {
            "table" => compare::compare_table(ctx, sub_matches),
            "all" => compare::compare_all(ctx),
            other => Err(NestError::command(format!(
                "compare subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
