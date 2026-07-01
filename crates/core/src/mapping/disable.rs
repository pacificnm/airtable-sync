//! `mapping disable` command handler.

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

/// Cached table metadata in mapping disable output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingDisableTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
}

/// JSON response for `mapping disable` with `--json`.
#[derive(Debug, Serialize)]
pub struct MappingDisableResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Cached table metadata.
    pub table: MappingDisableTableView,
    /// Airtable field name updated.
    pub field_name: String,
    /// Mapped CSV column name, if any.
    pub csv_field: Option<String>,
    /// Source CSV file basename for the mapped column, if any.
    pub csv_file: Option<String>,
    /// Whether field sync is enabled after the update.
    pub sync_enabled: bool,
    /// Whether sync was turned off (false when already disabled).
    pub disabled: bool,
}

/// Disables field synchronization for one mapped field.
pub fn disable_mapping(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
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
            "field `{field_name}` is computed and cannot be synced"
        )));
    }

    if field.csv_field.is_none() {
        return Err(NestError::validation(format!(
            "field `{field_name}` has no CSV mapping — run `mapping set` first"
        )));
    }

    let was_enabled = field.sync_enabled;

    let updated = schema_store
        .set_field_sync_enabled(&table.table_id, field_name, false)
        .map_err(NestError::from)?;

    if !updated {
        return Err(NestError::data(format!(
            "field `{field_name}` could not be updated — run `airtable pull-schema`"
        )));
    }

    let field = schema_store
        .find_field_by_name(&table.table_id, field_name)
        .map_err(NestError::from)?
        .ok_or_else(|| {
            NestError::data(format!(
                "field `{field_name}` disappeared after update — run `airtable pull-schema`"
            ))
        })?;

    let result = MappingDisableResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        table: MappingDisableTableView::from(table),
        field_name: field.field_name.clone(),
        csv_field: field.csv_field.clone(),
        csv_file: field.csv_filename.clone(),
        sync_enabled: field.sync_enabled,
        disabled: was_enabled,
    };

    print_mapping_disable_success(&result, json, quiet)
}

impl From<AirtableTableRow> for MappingDisableTableView {
    fn from(table: AirtableTableRow) -> Self {
        Self {
            name: table.name,
            table_id: table.table_id,
        }
    }
}

fn print_mapping_disable_success(
    result: &MappingDisableResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping disable result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.disabled {
        println!(
            "Disabled sync for {}.{} -> {}",
            result.table.name,
            result.field_name,
            result.csv_field.as_deref().unwrap_or("-")
        );
    } else {
        println!(
            "Sync already disabled for {}.{}",
            result.table.name, result.field_name
        );
    }

    Ok(())
}
