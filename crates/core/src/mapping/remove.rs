//! `mapping remove` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{
    absolute_path, ensure_schema_cache, open_database, AirtableTableRow, SchemaStore,
};

/// Cached table metadata in mapping remove output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingRemoveTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
}

/// JSON response for `mapping remove` with `--json`.
#[derive(Debug, Serialize)]
pub struct MappingRemoveResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Cached table metadata.
    pub table: MappingRemoveTableView,
    /// Airtable field name updated.
    pub field_name: String,
    /// Whether a mapping or sync flag was cleared.
    pub removed: bool,
}

/// Removes a field mapping from SQLite.
pub fn remove_mapping(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let table_name = matches
        .get_one::<String>("table")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing table name"))?;
    let field_name = matches
        .get_one::<String>("field")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing field name"))?;

    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);
    ensure_schema_cache(&database_path)?;

    let db = open_database(&database_path)?;
    let schema_store = SchemaStore::new(db);

    let Some(table) = schema_store
        .find_table_by_name(table_name)
        .map_err(NestError::from)?
    else {
        return Err(NestError::data(format!(
            "table `{table_name}` not in cache — run `airtable pull-schema` or `airtable list-tables`"
        )));
    };

    let Some(field) = schema_store
        .find_field_by_name(&table.table_id, field_name)
        .map_err(NestError::from)?
    else {
        return Err(NestError::data(format!(
            "field `{field_name}` not in cache for table `{table_name}` — run `airtable pull-schema`"
        )));
    };

    if field.is_computed {
        return Err(NestError::validation(format!(
            "field `{field_name}` is computed and cannot be mapped"
        )));
    }

    let had_mapping = field.csv_field.is_some()
        || field.csv_filename.is_some()
        || field.sync_enabled;

    let updated = schema_store
        .clear_field_mapping(&table.table_id, field_name)
        .map_err(NestError::from)?;

    if !updated {
        return Err(NestError::data(format!(
            "field `{field_name}` could not be updated — run `airtable pull-schema`"
        )));
    }

    let result = MappingRemoveResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        table: MappingRemoveTableView::from(table),
        field_name: field.field_name,
        removed: had_mapping,
    };

    print_mapping_remove_success(&result, json, quiet)
}

impl From<AirtableTableRow> for MappingRemoveTableView {
    fn from(table: AirtableTableRow) -> Self {
        Self {
            name: table.name,
            table_id: table.table_id,
        }
    }
}

fn print_mapping_remove_success(
    result: &MappingRemoveResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping remove result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.removed {
        println!(
            "Removed mapping for {}.{}",
            result.table.name, result.field_name
        );
    } else {
        println!(
            "No mapping on {}.{}",
            result.table.name, result.field_name
        );
    }

    Ok(())
}
