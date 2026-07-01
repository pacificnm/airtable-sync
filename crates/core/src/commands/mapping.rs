//! `mapping` command group with nested subcommands.

use clap::{Arg, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::mapping;

/// Field mapping management command group.
pub struct MappingCommand;

impl CliCommand for MappingCommand {
    fn name(&self) -> &'static str {
        "mapping"
    }

    fn about(&self) -> &'static str {
        "Field mapping management"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "mapping")
            .expect("mapping command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = if sub.name == "list" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("table")
                            .required(true)
                            .help("Logical table name from config (see `airtable list-tables`)"),
                    )
            } else if sub.name == "disable" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("table")
                            .required(true)
                            .help("Logical table name from config (see `airtable list-tables`)"),
                    )
                    .arg(
                        Arg::new("field")
                            .required(true)
                            .help("Airtable field name (as in schema cache)"),
                    )
            } else if sub.name == "enable" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("table")
                            .required(true)
                            .help("Logical table name from config (see `airtable list-tables`)"),
                    )
                    .arg(
                        Arg::new("field")
                            .required(true)
                            .help("Airtable field name (as in schema cache)"),
                    )
            } else if sub.name == "remove" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("table")
                            .required(true)
                            .help("Logical table name from config (see `airtable list-tables`)"),
                    )
                    .arg(
                        Arg::new("field")
                            .required(true)
                            .help("Airtable field name (as in schema cache)"),
                    )
            } else if sub.name == "set" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("table")
                            .required(true)
                            .help("Logical table name from config (see `airtable list-tables`)"),
                    )
                    .arg(
                        Arg::new("field")
                            .required(true)
                            .help("Airtable field name (as in schema cache)"),
                    )
                    .arg(
                        Arg::new("csv_column")
                            .required(true)
                            .help("CSV header or column name to map"),
                    )
                    .arg(
                        Arg::new("csv-file")
                            .long("csv-file")
                            .value_parser(["location", "space"])
                            .help("Disambiguate when the column exists in both CSV files"),
                    )
                    .arg(
                        Arg::new("enable")
                            .long("enable")
                            .action(clap::ArgAction::SetTrue)
                            .help("Enable field synchronization"),
                    )
                    .arg(
                        Arg::new("disable")
                            .long("disable")
                            .action(clap::ArgAction::SetTrue)
                            .help("Disable field synchronization"),
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
            NestError::command("missing mapping subcommand")
        })?;

        match subcommand {
            "list" => mapping::list_mappings(ctx, sub_matches),
            "set" => mapping::set_mapping(ctx, sub_matches),
            "remove" => mapping::remove_mapping(ctx, sub_matches),
            "enable" => mapping::enable_mapping(ctx, sub_matches),
            "disable" => mapping::disable_mapping(ctx, sub_matches),
            "report" => mapping::mapping_report(ctx),
            other => Err(NestError::command(format!(
                "mapping subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
