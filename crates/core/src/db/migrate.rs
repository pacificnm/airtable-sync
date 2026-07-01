//! Database migration for `db migrate`.

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};

use super::common::{apply_pending_migrations, DbMigrateResult};

/// Applies pending database migrations.
pub fn migrate(ctx: &AppContext) -> NestResult<()> {
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

    let result = apply_pending_migrations(&database_path, &schema_path, true)?;

    if json {
        let payload = serde_json::to_string_pretty(&result).map_err(|error| {
            NestError::data(format!("failed to serialize db migrate result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!("{}", format_migrate_human(&result));
    }

    Ok(())
}

fn format_migrate_human(result: &DbMigrateResult) -> String {
    if result.applied.is_empty() {
        let count = result.all_applied.len();
        let label = if count == 1 { "migration" } else { "migrations" };
        return format!("Database up to date ({count} {label} applied)");
    }

    let count = result.applied.len();
    let label = if count == 1 { "migration" } else { "migrations" };
    let ids = result.applied.join(", ");

    if result.database_created {
        format!("Created database and applied {count} {label}: {ids}")
    } else {
        format!("Applied {count} {label}: {ids}")
    }
}
