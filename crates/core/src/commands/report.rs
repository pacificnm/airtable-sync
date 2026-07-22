//! `report` command group with nested subcommands.

use clap::{Arg, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::report;

fn plan_id_arg() -> Arg {
    Arg::new("plan-id")
        .long("plan-id")
        .value_parser(clap::value_parser!(i64))
        .help("Change plan id (default: latest draft, else most recent plan)")
}

fn status_arg() -> Arg {
    Arg::new("status")
        .long("status")
        .help("Filter operations by status (pending, approved, denied, applied, failed)")
}

/// Generate reports command group.
pub struct ReportCommand;

impl CliCommand for ReportCommand {
    fn name(&self) -> &'static str {
        "report"
    }

    fn about(&self) -> &'static str {
        "Generate reports"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "report")
            .expect("report command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = match sub.name {
                "changes" => Command::new(sub.name)
                    .about(sub.about)
                    .arg(plan_id_arg())
                    .arg(status_arg()),
                _ => Command::new(sub.name).about(sub.about),
            };
            cmd = cmd.subcommand(sub_cmd);
        }

        cmd
    }

    fn run(&self, ctx: &AppContext, matches: &clap::ArgMatches) -> NestResult<()> {
        let (subcommand, sub_matches) = matches.subcommand().ok_or_else(|| {
            NestError::command("missing report subcommand")
        })?;

        match subcommand {
            "changes" => report::report_changes(ctx, sub_matches),
            "validation" => report::report_validation(ctx),
            "summary" => report::report_summary(ctx),
            other => Err(NestError::command(format!(
                "report subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
