//! Show configured Airtable tables.

use clap::Command;
use nest_airtable::AirtableClient;
use nest_cli::{CliCommand, CliGlobals};
use nest_core::AppContext;
use nest_error::NestResult;

/// Prints logical table names from configuration.
pub struct TablesCommand;

impl CliCommand for TablesCommand {
    fn name(&self) -> &'static str {
        "tables"
    }

    fn about(&self) -> &'static str {
        "List configured Airtable table names"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd
    }

    fn run(&self, ctx: &AppContext, _matches: &clap::ArgMatches) -> NestResult<()> {
        let quiet = ctx
            .service::<CliGlobals>()
            .map(|globals| globals.quiet)
            .unwrap_or(false);
        if quiet {
            return Ok(());
        }

        let airtable = ctx.service::<AirtableClient>()?;
        let mut names: Vec<_> = airtable.config().tables.keys().cloned().collect();
        names.sort();

        if names.is_empty() {
            println!("No tables configured. Add [airtable.tables.<name>] sections to config.");
            return Ok(());
        }

        for name in names {
            let table = airtable.config().table(&name)?;
            let pk = table
                .primary_key_field
                .as_deref()
                .unwrap_or("-");
            println!("{name}\ttable_id={}\tprimary_key={pk}", table.table_id);
        }

        Ok(())
    }
}
