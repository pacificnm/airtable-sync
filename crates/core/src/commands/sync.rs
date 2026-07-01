//! `sync` command group with nested subcommands.

use clap::{Arg, Command};
use nest_cli::CliCommand;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::commands::spec::COMMAND_GROUPS;
use crate::sync;

fn plan_id_arg() -> Arg {
    Arg::new("plan-id")
        .long("plan-id")
        .value_parser(clap::value_parser!(i64))
        .help("Change plan id (default: latest draft for this base)")
}

fn operation_lookup_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("operation_id")
            .value_parser(clap::value_parser!(i64))
            .help("Operation id from `sync review`"),
    )
    .arg(plan_id_arg())
    .arg(
        Arg::new("table")
            .long("table")
            .help("Logical table name (use with --key instead of operation id)"),
    )
    .arg(
        Arg::new("key")
            .long("key")
            .help("Primary key value (use with --table instead of operation id)"),
    )
}

/// Synchronize CSV to Airtable command group.
pub struct SyncCommand;

impl CliCommand for SyncCommand {
    fn name(&self) -> &'static str {
        "sync"
    }

    fn about(&self) -> &'static str {
        "Synchronize Airtable"
    }

    fn configure(&self, cmd: Command) -> Command {
        let spec = COMMAND_GROUPS
            .iter()
            .find(|group| group.name == "sync")
            .expect("sync command group must exist in COMMAND_GROUPS");

        let mut cmd = cmd
            .subcommand_required(true)
            .arg_required_else_help(true);

        for sub in spec.subcommands {
            let sub_cmd = match sub.name {
                "review" | "apply" => Command::new(sub.name).about(sub.about).arg(plan_id_arg()),
                "approve" => operation_lookup_args(Command::new(sub.name).about(sub.about)),
                "deny" => operation_lookup_args(Command::new(sub.name).about(sub.about)),
                "approve-all" | "deny-all" => {
                    Command::new(sub.name).about(sub.about).arg(plan_id_arg())
                }
                _ => Command::new(sub.name).about(sub.about),
            };
            cmd = cmd.subcommand(sub_cmd);
        }

        cmd
    }

    fn run(&self, ctx: &AppContext, matches: &clap::ArgMatches) -> NestResult<()> {
        let (subcommand, sub_matches) = matches.subcommand().ok_or_else(|| {
            NestError::command("missing sync subcommand")
        })?;

        match subcommand {
            "dry-run" => sync::sync_dry_run(ctx),
            "review" => sync::sync_review(ctx, sub_matches),
            "approve" => sync::sync_approve(ctx, sub_matches),
            "deny" => sync::sync_deny(ctx, sub_matches),
            "approve-all" => sync::sync_approve_all(ctx, sub_matches),
            "deny-all" => sync::sync_deny_all(ctx, sub_matches),
            "apply" => sync::sync_apply(ctx, sub_matches),
            other => Err(NestError::command(format!(
                "sync subcommand `{other}` is not yet implemented"
            ))),
        }
    }
}
