//! `mapping set` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::csv::CsvFileRole;
use crate::db::{
    absolute_path, ensure_csv_cache, ensure_schema_cache, open_database, AirtableTableRow,
    FieldMappingUpdate, SchemaStore, CsvStore,
};
use crate::mapping::resolve::resolve_csv_column;

/// Cached table metadata in mapping set output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingSetTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
}

/// JSON response for `mapping set` with `--json`.
#[derive(Debug, Serialize)]
pub struct MappingSetResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Cached table metadata.
    pub table: MappingSetTableView,
    /// Airtable field name updated.
    pub field_name: String,
    /// Normalized CSV column stored in `csv_field`.
    pub csv_field: String,
    /// Source CSV file basename for the resolved column.
    pub csv_file: String,
    /// Whether field sync is enabled after the update.
    pub sync_enabled: bool,
    /// Whether a row was updated in SQLite.
    pub updated: bool,
}

/// Creates or updates one field mapping in SQLite.
pub fn set_mapping(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let table_name = matches
        .get_one::<String>("table")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing table name"))?;
    let field_name = matches
        .get_one::<String>("field")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing field name"))?;
    let csv_column = matches
        .get_one::<String>("csv_column")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing CSV column name"))?;

    let enable = matches.get_flag("enable");
    let disable = matches.get_flag("disable");
    if enable && disable {
        return Err(NestError::command(
            "`--enable` and `--disable` cannot be used together",
        ));
    }

    let csv_file = matches
        .get_one::<String>("csv-file")
        .map(|value| CsvFileRole::parse(value.as_str()))
        .transpose()?;

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
    ensure_csv_cache(&database_path)?;

    let db = open_database(&database_path)?;
    let schema_store = SchemaStore::new(db.clone());
    let csv_store = CsvStore::new(db);

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

    let resolved = resolve_csv_column(
        &csv_store,
        &validated.config,
        &validated.app,
        csv_column,
        csv_file,
    )?;

    let sync_enabled = match (enable, disable) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    };

    let updated = schema_store
        .set_field_mapping(
            &table.table_id,
            field_name,
            &FieldMappingUpdate {
                csv_field: resolved.normalized_name.clone(),
                csv_filename: resolved.filename.clone(),
                sync_enabled,
            },
        )
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

    let result = MappingSetResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        table: MappingSetTableView::from(table),
        field_name: field.field_name.clone(),
        csv_field: field.csv_field.clone().unwrap_or(resolved.normalized_name),
        csv_file: field
            .csv_filename
            .clone()
            .unwrap_or(resolved.filename),
        sync_enabled: field.sync_enabled,
        updated,
    };

    print_mapping_set_success(&result, json, quiet)
}

impl From<AirtableTableRow> for MappingSetTableView {
    fn from(table: AirtableTableRow) -> Self {
        Self {
            name: table.name,
            table_id: table.table_id,
        }
    }
}

fn print_mapping_set_success(result: &MappingSetResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize mapping set result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    println!(
        "Mapped {}.{} -> {} ({})",
        result.table.name,
        result.field_name,
        result.csv_field,
        sync_status(result.sync_enabled)
    );

    Ok(())
}

fn sync_status(sync_enabled: bool) -> &'static str {
    if sync_enabled {
        "sync enabled"
    } else {
        "sync disabled"
    }
}
