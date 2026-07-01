//! `db` command group with nested subcommands.

use clap::{Arg, ArgAction, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::db;

/// SQLite database management command group.
pub struct DbCommand;

impl CliCommand for DbCommand {
    fn name(&self) -> &'static str {
        "db"
    }

    fn about(&self) -> &'static str {
        "SQLite database management"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "db")
            .expect("db command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = if sub.name == "reset" {
                Command::new(sub.name)
                    .about(sub.about)
                    .arg(
                        Arg::new("yes")
                            .long("yes")
                            .action(ArgAction::SetTrue)
                            .help("Confirm destructive database recreation"),
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
            NestError::command("missing db subcommand")
        })?;

        match subcommand {
            "init" => db::init(ctx),
            "reset" => db::reset(ctx, sub_matches),
            "schema" => db::schema(ctx),
            "migrate" => db::migrate(ctx),
            other => Err(NestError::command(format!("unknown db subcommand: {other}"))),
        }
    }
}
