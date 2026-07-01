//! `airtable list-fields` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{
    absolute_path, ensure_schema_cache, open_database, AirtableFieldRow, AirtableTableRow,
    SchemaStore,
};

/// Cached table metadata included in list-fields output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListFieldsTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Whether sync is enabled for this table.
    pub enabled: bool,
    /// Whether creates are allowed during sync.
    pub allow_create: bool,
    /// Whether updates are allowed during sync.
    pub allow_update: bool,
}

/// One field in the schema cache list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListFieldsFieldView {
    /// Airtable field id (`fld…`), when known.
    pub field_id: Option<String>,
    /// Field display name.
    pub field_name: String,
    /// Airtable field type.
    pub field_type: Option<String>,
    /// Whether the field is computed or read-only.
    pub is_computed: bool,
    /// Whether this field is the table primary key.
    pub is_key: bool,
    /// Whether field sync is enabled (mapping layer).
    pub sync_enabled: bool,
    /// Mapped CSV column name, if any.
    pub csv_field: Option<String>,
    /// Source CSV file basename for the mapped column, if any.
    pub csv_file: Option<String>,
}

/// JSON response for `airtable list-fields` with `--json`.
#[derive(Debug, Serialize)]
pub struct ListFieldsResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Cached table metadata.
    pub table: ListFieldsTableView,
    /// Cached fields ordered by name.
    pub fields: Vec<ListFieldsFieldView>,
}

/// Lists cached Airtable fields for one table from SQLite.
pub fn list_fields(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let table_name = matches
        .get_one::<String>("table")
        .map(String::as_str)
        .ok_or_else(|| NestError::command("missing table name"))?;

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
    let store = SchemaStore::new(db);

    let Some(table) = store
        .find_table_by_name(table_name)
        .map_err(NestError::from)?
    else {
        return Err(NestError::data(format!(
            "table `{table_name}` not in cache — run `airtable pull-schema` or `airtable list-tables`"
        )));
    };

    let fields = store
        .list_fields(&table.table_id)
        .map_err(NestError::from)?;

    let result = ListFieldsResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        table: ListFieldsTableView::from(table),
        fields: fields.into_iter().map(ListFieldsFieldView::from).collect(),
    };

    print_list_fields_success(&result, json, quiet)
}

impl From<AirtableTableRow> for ListFieldsTableView {
    fn from(table: AirtableTableRow) -> Self {
        Self {
            name: table.name,
            table_id: table.table_id,
            enabled: table.enabled,
            allow_create: table.allow_create,
            allow_update: table.allow_update,
        }
    }
}

impl From<AirtableFieldRow> for ListFieldsFieldView {
    fn from(field: AirtableFieldRow) -> Self {
        Self {
            field_id: field.field_id,
            field_name: field.field_name,
            field_type: field.field_type,
            is_computed: field.is_computed,
            is_key: field.is_key,
            sync_enabled: field.sync_enabled,
            csv_field: field.csv_field,
            csv_file: field.csv_filename,
        }
    }
}

fn print_list_fields_success(
    result: &ListFieldsResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize list-fields result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.fields.is_empty() {
        println!(
            "No fields in cache for table `{}` — run `airtable pull-schema` first.",
            result.table.name
        );
        return Ok(());
    }

    println!(
        "Cached Airtable fields for table `{}` ({}) in base {}:",
        result.table.name,
        result.table.table_id,
        result.base_id
    );
    println!(
        "{:<20} {:<16} {:<18} {:<5} {:<9} {:<5} {:<12} {}",
        "field_name", "field_id", "type", "key", "computed", "sync", "csv_field", "csv_file"
    );
    for field in &result.fields {
        println!(
            "{:<20} {:<16} {:<18} {:<5} {:<9} {:<5} {:<12} {}",
            field.field_name,
            field.field_id.as_deref().unwrap_or("-"),
            field.field_type.as_deref().unwrap_or("-"),
            yes_no(field.is_key),
            yes_no(field.is_computed),
            yes_no(field.sync_enabled),
            field.csv_field.as_deref().unwrap_or("-"),
            field.csv_file.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
