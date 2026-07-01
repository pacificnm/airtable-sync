//! `airtable` command group with nested subcommands.

use clap::{Arg, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::airtable;
use crate::commands::spec::COMMAND_GROUPS;

/// Airtable schema and connectivity command group.
pub struct AirtableCommand;

impl CliCommand for AirtableCommand {
    fn name(&self) -> &'static str {
        "airtable"
    }

    fn about(&self) -> &'static str {
        "Airtable schema operations"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "airtable")
            .expect("airtable command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = if sub.name == "list-fields" {
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
            NestError::command("missing airtable subcommand")
        })?;

        match subcommand {
            "test" => airtable::test(ctx),
            "pull-schema" => airtable::pull_schema(ctx),
            "list-tables" => airtable::list_tables(ctx),
            "list-fields" => airtable::list_fields(ctx, sub_matches),
            other => Err(NestError::command(format!(
                "airtable subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
