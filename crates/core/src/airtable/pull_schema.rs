//! `airtable pull-schema` command handler.

use std::collections::HashMap;

use nest_airtable::{is_computed_field_type, AirtableClient, AirtableModule, AirtableTableSchema};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_http_client::HttpClientModule;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};
use crate::db::{ensure_schema_cache, AirtableFieldRow, AirtableTableRow, SchemaStore};
use crate::db::open_database;

use super::bridge::to_airtable_config;
use super::runtime::block_on_async;

/// JSON response for successful `airtable pull-schema` with `--json`.
#[derive(Debug, Serialize)]
pub struct PullSchemaResult {
    /// Airtable base id.
    pub base_id: String,
    /// Number of table rows upserted.
    pub tables_updated: usize,
    /// Number of field rows upserted.
    pub fields_upserted: usize,
    /// Non-fatal warnings encountered during the pull.
    pub warnings: Vec<String>,
}

/// Downloads Airtable schema metadata into SQLite.
pub fn pull_schema(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    let mut warnings = Vec::new();

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);
    ensure_schema_cache(&database_path)?;

    let (tables, fields, pull_warnings) =
        map_configured_schema(&validated.app, fetch_base_schema(&validated.app)?)?;
    warnings.extend(pull_warnings);

    let db = open_database(&database_path)?;
    let stats = SchemaStore::new(db)
        .replace_schema_for_pull(&tables, &fields)
        .map_err(NestError::from)?;

    let result = PullSchemaResult {
        base_id: validated.app.airtable.base_id.clone(),
        tables_updated: stats.tables_updated,
        fields_upserted: stats.fields_upserted,
        warnings,
    };

    print_pull_schema_success(&result, json, quiet)
}

fn fetch_base_schema(
    app: &crate::config::AppConfig,
) -> NestResult<nest_airtable::AirtableBaseSchema> {
    let client_config = to_airtable_config(app)?;
    block_on_async(async move {
        let built = AppBuilder::new()
            .module(HttpClientModule::default())
            .module(AirtableModule::with_config(client_config))
            .build()?;
        let client = built.context.service::<AirtableClient>()?.clone();
        client.get_base_schema().await
    })
}

fn map_configured_schema(
    app: &crate::config::AppConfig,
    base_schema: nest_airtable::AirtableBaseSchema,
) -> NestResult<(Vec<AirtableTableRow>, Vec<AirtableFieldRow>, Vec<String>)> {
    let meta_by_id: HashMap<&str, &AirtableTableSchema> = base_schema
        .tables
        .iter()
        .map(|table| (table.id.as_str(), table))
        .collect();

    let mut tables = Vec::new();
    let mut fields = Vec::new();
    let mut warnings = Vec::new();

    for (logical_name, table_cfg) in &app.airtable.tables {
        let Some(meta_table) = meta_by_id.get(table_cfg.table_id.as_str()) else {
            warnings.push(format!(
                "airtable.tables.{logical_name}: table_id {} not found in Airtable base {}",
                table_cfg.table_id, app.airtable.base_id
            ));
            continue;
        };

        tables.push(AirtableTableRow {
            name: logical_name.clone(),
            table_id: table_cfg.table_id.clone(),
            enabled: table_cfg.sync,
            allow_create: false,
            allow_update: true,
        });

        if meta_table.fields.is_empty() {
            warnings.push(format!(
                "airtable.tables.{logical_name}: Airtable returned no fields for table {}",
                table_cfg.table_id
            ));
        }

        for field in &meta_table.fields {
            fields.push(AirtableFieldRow {
                table_id: table_cfg.table_id.clone(),
                field_id: Some(field.id.clone()),
                field_name: field.name.clone(),
                field_type: Some(field.field_type.clone()),
                is_computed: is_computed_field_type(&field.field_type),
                is_key: field.id == meta_table.primary_field_id,
                sync_enabled: false,
                csv_field: None,
                csv_filename: None,
            });
        }
    }

    Ok((tables, fields, warnings))
}

fn print_pull_schema_success(result: &PullSchemaResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize pull-schema result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!(
            "Pulled Airtable schema for base {}: {} table(s), {} field(s)",
            result.base_id, result.tables_updated, result.fields_upserted
        );
        for warning in &result.warnings {
            println!("warning: {warning}");
        }
    }
    Ok(())
}
