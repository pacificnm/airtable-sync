//! Database initialization for `db init`.

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};

use super::common::{create_database, DbInitResult, MIGRATION_ID};

/// Creates the SQLite database and applies the initial schema migration.
pub fn init(ctx: &AppContext) -> NestResult<()> {
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

    if database_path.is_file() {
        return Err(NestError::data(format!(
            "database file already exists: {}",
            database_path.display()
        ))
        .with_help("Use `db reset --yes` to recreate the database."));
    }

    let result = create_database(&validated.config, &validated.app, &database_path, &schema_path)?;
    print_init_success(&result, json, quiet)?;

    Ok(())
}

fn print_init_success(result: &DbInitResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize db init result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!(
            "Created database: {} (migration: {MIGRATION_ID})",
            result.database_path.display()
        );
    }
    Ok(())
}
