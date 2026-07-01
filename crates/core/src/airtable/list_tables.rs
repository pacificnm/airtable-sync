//! `airtable list-tables` command handler.

use std::path::PathBuf;

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{absolute_path, ensure_schema_cache, open_database, AirtableTableSummary, SchemaStore};

/// One table in the schema cache list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListTablesTableView {
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
    /// Number of cached fields for this table.
    pub field_count: usize,
}

/// JSON response for `airtable list-tables` with `--json`.
#[derive(Debug, Serialize)]
pub struct ListTablesResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Cached tables ordered by name.
    pub tables: Vec<ListTablesTableView>,
}

/// Lists cached Airtable tables from SQLite.
pub fn list_tables(ctx: &AppContext) -> NestResult<()> {
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
    let tables = SchemaStore::new(db)
        .list_tables_summary()
        .map_err(NestError::from)?;

    let result = ListTablesResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        tables: tables.into_iter().map(ListTablesTableView::from).collect(),
    };

    print_list_tables_success(&result, json, quiet)
}

impl From<AirtableTableSummary> for ListTablesTableView {
    fn from(table: AirtableTableSummary) -> Self {
        Self {
            name: table.name,
            table_id: table.table_id,
            enabled: table.enabled,
            allow_create: table.allow_create,
            allow_update: table.allow_update,
            field_count: table.field_count,
        }
    }
}

fn print_list_tables_success(result: &ListTablesResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize list-tables result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if result.tables.is_empty() {
        println!("No tables in cache — run `airtable pull-schema` first.");
        return Ok(());
    }

    println!(
        "Cached Airtable tables for base {} ({}):",
        result.base_id,
        result.database_path.display()
    );
    println!("{:<20} {:<16} {:<8} {}", "name", "table_id", "enabled", "fields");
    for table in &result.tables {
        println!(
            "{:<20} {:<16} {:<8} {}",
            table.name,
            table.table_id,
            yes_no(table.enabled),
            table.field_count
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
