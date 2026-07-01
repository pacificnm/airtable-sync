//! Database reset for `db reset`.

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};

use super::common::{create_database, remove_database_files, DbResetResult, MIGRATION_ID};

/// Deletes and recreates the SQLite database after explicit confirmation.
pub fn reset(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    if !matches.get_flag("yes") {
        return Err(NestError::command("refusing to reset database without --yes")
            .with_help(
                "This permanently deletes all local sync data. Re-run with --yes to confirm.",
            ));
    }

    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let database_path = resolve_config_path(&validated.config, &validated.app.database.database_path);
    let schema_path = resolve_config_path(&validated.config, &validated.app.database.schema);
    let previous_existed = database_path.is_file();

    remove_database_files(&database_path)?;

    let created = create_database(&validated.config, &validated.app, &database_path, &schema_path)?;
    let result = DbResetResult {
        database_path: created.database_path,
        schema: created.schema,
        migration: created.migration,
        previous_existed,
    };

    if json {
        let payload = serde_json::to_string_pretty(&result).map_err(|error| {
            NestError::data(format!("failed to serialize db reset result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!(
            "Recreated database: {} (migration: {MIGRATION_ID})",
            result.database_path.display()
        );
    }

    Ok(())
}
