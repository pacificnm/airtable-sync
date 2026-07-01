//! `csv` command group with nested subcommands.

use clap::{Arg, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::csv;

/// CSV import command group.
pub struct CsvCommand;

impl CliCommand for CsvCommand {
    fn name(&self) -> &'static str {
        "csv"
    }

    fn about(&self) -> &'static str {
        "CSV import operations"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "csv")
            .expect("csv command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = if sub.name == "preview" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("file")
                            .required(true)
                            .value_parser(["location", "space"])
                            .help("Configured CSV file to preview (`location` or `space`)"),
                    )
                    .arg(
                        Arg::new("limit")
                            .long("limit")
                            .default_value("5")
                            .help("Maximum number of data rows to preview (1-100)"),
                    )
            } else if sub.name == "validate" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("file")
                            .required(false)
                            .value_parser(["location", "space"])
                            .help("Validate one CSV file (`location` or `space`); default validates both"),
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
            NestError::command("missing csv subcommand")
        })?;

        match subcommand {
            "import-headers" => csv::import_headers(ctx),
            "preview" => csv::preview(ctx, sub_matches),
            "validate" => csv::validate(ctx, sub_matches),
            other => Err(NestError::command(format!(
                "csv subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
