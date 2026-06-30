//! List records from a configured Airtable table.

use clap::{Arg, ArgAction, Command};
use nest_airtable::{AirtableClient, AirtableListParams};
use nest_cli::{AsyncCliCommand, CliGlobals};
use nest_core::AppContext;
use nest_error::NestResult;
use tracing::info;

/// Lists records from an Airtable table.
pub struct ListCommand;

#[async_trait::async_trait]
impl AsyncCliCommand for ListCommand {
    fn name(&self) -> &'static str {
        "list"
    }

    fn about(&self) -> &'static str {
        "List records from a configured Airtable table"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd.arg(
            Arg::new("table")
                .required(true)
                .help("Logical table name from [airtable.tables.<name>] in config"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Emit records as JSON array"),
        )
    }

    async fn run_async(&self, ctx: &AppContext, matches: &clap::ArgMatches) -> NestResult<()> {
        let table = matches
            .get_one::<String>("table")
            .map(String::as_str)
            .expect("table required");
        let as_json = matches.get_flag("json");
        let quiet = ctx
            .service::<CliGlobals>()
            .map(|globals| globals.quiet)
            .unwrap_or(false);

        let airtable = ctx.service::<AirtableClient>()?;
        info!(table, "listing airtable records");
        let records = airtable
            .list_all_records(table, AirtableListParams::default())
            .await?;

        if as_json {
            let body = serde_json::to_string_pretty(&records).map_err(|error| {
                nest_error::NestError::validation(format!("failed to encode JSON: {error}"))
            })?;
            if !quiet {
                println!("{body}");
            }
        } else if !quiet {
            println!("{table}: {} record(s)", records.len());
        }

        Ok(())
    }
}
