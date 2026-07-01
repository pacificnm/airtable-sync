//! `compare table` command handler.

use std::path::PathBuf;

use clap::ArgMatches;
use nest_airtable::{AirtableClient, AirtableListParams, AirtableModule};
use nest_cli::CliGlobals;
use nest_config::ConfigService;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_http_client::HttpClientModule;
use serde::Serialize;

use crate::airtable::{block_on_async, to_airtable_config};
use crate::compare::csv_index::index_csv_by_key;
use crate::compare::diff::{compare_records, CompareDiffResult, CompareField};
use crate::config::{ensure_valid_config, print_warning, resolve_config_path, AppConfig};
use crate::csv::resolve_csv_path_by_filename;
use crate::db::{
    absolute_path, ensure_schema_cache, open_database, FieldMappingRow, SchemaStore,
};

/// Cached table metadata in compare output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareTableView {
    /// Logical table name from config.
    pub name: String,
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Whether sync is enabled for this table.
    pub enabled: bool,
}

/// JSON response for `compare table` with `--json`.
#[derive(Debug, Serialize)]
pub struct CompareTableResult {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Airtable base id from config.
    pub base_id: String,
    /// Compared table metadata.
    pub table: CompareTableView,
    /// Airtable primary key field name.
    pub primary_key_field: String,
    /// Normalized CSV column used as the primary key.
    pub primary_key_csv_column: String,
    /// Source CSV file basename.
    pub csv_file: String,
    /// Absolute path to the source CSV file.
    pub csv_path: PathBuf,
    /// Compared fields (sync enabled, mapped, same CSV file).
    pub compared_fields: Vec<String>,
    /// Compare summary and diffs.
    pub compare: CompareDiffResult,
}

/// Compares one configured table's CSV rows against live Airtable records.
pub fn compare_table(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
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

    let Some(table_entry) = validated.app.airtable.tables.get(table_name) else {
        return Err(NestError::data(format!(
            "table `{table_name}` is not configured — see `airtable list-tables`"
        )));
    };

    let primary_key_field = table_entry.primary_key_field.as_deref().ok_or_else(|| {
        NestError::validation(format!(
            "table `{table_name}` has no primary_key_field configured"
        ))
        .with_help("Set `primary_key_field` under [airtable.tables.<name>] in config.toml.")
    })?;

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
        .list_mappable_fields(&table.table_id)
        .map_err(NestError::from)?;

    let prepared = prepare_compare_fields(
        &validated.config,
        &validated.app,
        table_name,
        primary_key_field,
        &fields,
    )?;

    if !quiet {
        for warning in &prepared.warnings {
            println!("warning: {warning}");
        }
    }

    let files = FileService::new()?;
    let csv_index = index_csv_by_key(
        &files,
        &prepared.csv_path,
        &prepared.primary_key_csv_column,
    )?;

    if !quiet {
        for warning in &csv_index.warnings {
            println!("warning: {warning}");
        }
        if csv_index.skipped_empty_keys > 0 {
            println!(
                "warning: skipped {} CSV row(s) with empty primary key",
                csv_index.skipped_empty_keys
            );
        }
    }

    let airtable_config = to_airtable_config(&validated.app)?;
    let fetch_table = table_name.to_string();
    let fetch_fields = prepared.airtable_field_names.clone();

    let airtable_records = block_on_async(async move {
        let built = AppBuilder::new()
            .module(HttpClientModule::default())
            .module(AirtableModule::with_config(airtable_config))
            .build()?;
        let client = built.context.service::<AirtableClient>()?.clone();
        let mut params = AirtableListParams::default();
        params.fields = Some(fetch_fields);
        client.list_all_records(&fetch_table, params).await
    })?;

    let compare = compare_records(
        &csv_index,
        &airtable_records,
        primary_key_field,
        &prepared.compare_fields,
    );

    let result = CompareTableResult {
        database_path: absolute_path(&database_path),
        base_id: validated.app.airtable.base_id.clone(),
        table: CompareTableView {
            name: table.name,
            table_id: table.table_id,
            enabled: table.enabled,
        },
        primary_key_field: primary_key_field.to_string(),
        primary_key_csv_column: prepared.primary_key_csv_column.clone(),
        csv_file: prepared.csv_filename.clone(),
        csv_path: prepared.csv_path.clone(),
        compared_fields: prepared
            .compare_fields
            .iter()
            .map(|field| field.field_name.clone())
            .collect(),
        compare,
    };

    print_compare_table_success(&result, json, quiet)
}

struct PreparedCompareFields {
    csv_filename: String,
    csv_path: PathBuf,
    primary_key_csv_column: String,
    compare_fields: Vec<CompareField>,
    airtable_field_names: Vec<String>,
    warnings: Vec<String>,
}

fn prepare_compare_fields(
    config: &ConfigService,
    app: &AppConfig,
    table_name: &str,
    primary_key_field: &str,
    fields: &[FieldMappingRow],
) -> NestResult<PreparedCompareFields> {
    let pk_field = fields
        .iter()
        .find(|field| field.field_name == primary_key_field)
        .ok_or_else(|| {
            NestError::validation(format!(
                "primary key field `{primary_key_field}` not found in schema cache for table `{table_name}`"
            ))
            .with_help("Run `airtable pull-schema` after changing Airtable fields.")
        })?;

    let pk_csv_column = pk_field.csv_field.as_deref().ok_or_else(|| {
        NestError::validation(format!(
            "primary key field `{primary_key_field}` is not mapped to a CSV column"
        ))
        .with_help(format!(
            "Run `mapping set {table_name} {primary_key_field} <csv_column>` first."
        ))
    })?;

    let csv_filename = pk_field.csv_filename.as_deref().ok_or_else(|| {
        NestError::data(format!(
            "primary key field `{primary_key_field}` is missing csv_filename in cache"
        ))
    })?;

    let csv_path = resolve_csv_path_by_filename(config, app, csv_filename)?;
    let mut warnings = Vec::new();
    let mut compare_fields = Vec::new();
    let mut airtable_field_names = vec![primary_key_field.to_string()];

    for field in fields {
        if field.field_name == primary_key_field {
            continue;
        }
        if !field.sync_enabled {
            continue;
        }
        let Some(csv_column) = field.csv_field.as_deref() else {
            warnings.push(format!(
                "skipped sync-enabled field `{}` with no CSV mapping",
                field.field_name
            ));
            continue;
        };
        let Some(filename) = field.csv_filename.as_deref() else {
            warnings.push(format!(
                "skipped sync-enabled field `{}` with no csv_filename",
                field.field_name
            ));
            continue;
        };
        if filename != csv_filename {
            warnings.push(format!(
                "skipped sync-enabled field `{}` mapped to {filename} (primary key uses {csv_filename})",
                field.field_name
            ));
            continue;
        }

        compare_fields.push(CompareField {
            field_name: field.field_name.clone(),
            csv_column: csv_column.to_string(),
        });
        airtable_field_names.push(field.field_name.clone());
    }

    if compare_fields.is_empty() {
        warnings.push(
            "no sync-enabled mapped fields to compare besides the primary key".to_string(),
        );
    }

    compare_fields.sort_by(|left, right| left.field_name.cmp(&right.field_name));
    airtable_field_names.sort();
    airtable_field_names.dedup();

    Ok(PreparedCompareFields {
        csv_filename: csv_filename.to_string(),
        csv_path,
        primary_key_csv_column: pk_csv_column.to_string(),
        compare_fields,
        airtable_field_names,
        warnings,
    })
}

fn print_compare_table_success(
    result: &CompareTableResult,
    json: bool,
    quiet: bool,
) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize compare table result: {error}"))
        })?;
        println!("{payload}");
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    print_compare_summary_human(result);
    Ok(())
}

fn print_compare_summary_human(result: &CompareTableResult) {
    let summary = &result.compare.summary;
    println!(
        "Compare `{}` ({}) using primary key `{}` from {}:",
        result.table.name,
        result.table.table_id,
        result.primary_key_field,
        result.csv_file
    );
    println!(
        "Summary: {} CSV row(s), {} Airtable row(s), {} matched, {} differing, {} CSV-only, {} Airtable-only",
        summary.csv_rows,
        summary.airtable_rows,
        summary.matched,
        summary.differing,
        summary.csv_only,
        summary.airtable_only
    );

    if result.compared_fields.is_empty() {
        println!("Compared fields: none (primary key only)");
    } else {
        println!("Compared fields: {}", result.compared_fields.join(", "));
    }

    if !result.compare.differing_records.is_empty() {
        println!();
        println!("Differing records:");
        for record in &result.compare.differing_records {
            println!("  key `{}`:", record.key);
            for diff in &record.differences {
                println!(
                    "    - {} (CSV `{}`): {:?} != {:?}",
                    diff.field_name, diff.csv_column, diff.csv_value, diff.airtable_value
                );
            }
        }
    }

    if !result.compare.csv_only_keys.is_empty() {
        println!();
        println!(
            "CSV-only keys ({}): {}",
            result.compare.csv_only_keys.len(),
            result.compare.csv_only_keys.join(", ")
        );
    }

    if !result.compare.airtable_only_keys.is_empty() {
        println!();
        println!(
            "Airtable-only keys ({}): {}",
            result.compare.airtable_only_keys.len(),
            result.compare.airtable_only_keys.join(", ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_config::{ConfigDocument, ConfigService, ConfigSource, LoadedConfig};

    fn sample_config_service() -> ConfigService {
        ConfigService::new(LoadedConfig {
            document: ConfigDocument::empty(),
            source: ConfigSource::Memory(ConfigDocument::empty()),
            path: Some(std::path::PathBuf::from("/tmp/config.toml")),
        })
    }

    fn app_config() -> AppConfig {
        AppConfig {
            airtable: crate::config::AirtableSection {
                api_url: None,
                meta_api_url: None,
                token: Some("pat-test".to_string()),
                token_env: None,
                base_id: "appTEST".to_string(),
                tables: std::collections::HashMap::from([(
                    "assets".to_string(),
                    crate::config::AirtableTableEntry {
                        table_id: "tblTEST".to_string(),
                        sync: true,
                        primary_key_field: Some("ID".to_string()),
                    },
                )]),
            },
            sync: crate::config::SyncSection {
                dry_run: true,
                continue_on_error: true,
                max_parallel_tables: 2,
                max_parallel_updates: 5,
                create_change_plan: true,
            },
            csv: crate::config::CsvSection {
                location_data_file: "location.csv".into(),
                space_data_file: "space.csv".into(),
            },
            database: crate::config::DatabaseSection {
                provider: "sqlite".to_string(),
                database_path: "data/app.db".into(),
                schema: "schema.sql".into(),
            },
            logging: crate::config::LoggingSection {
                level: "info".to_string(),
                directory: "logs".into(),
            },
        }
    }

    #[test]
    fn prepare_compare_fields_selects_sync_enabled_same_csv_fields() {
        let fields = vec![
            FieldMappingRow {
                field_id: Some("fld1".to_string()),
                field_name: "ID".to_string(),
                field_type: Some("singleLineText".to_string()),
                is_key: true,
                csv_field: Some("id".to_string()),
                csv_filename: Some("location.csv".to_string()),
                sync_enabled: true,
            },
            FieldMappingRow {
                field_id: Some("fld2".to_string()),
                field_name: "Name".to_string(),
                field_type: Some("singleLineText".to_string()),
                is_key: false,
                csv_field: Some("name".to_string()),
                csv_filename: Some("location.csv".to_string()),
                sync_enabled: true,
            },
            FieldMappingRow {
                field_id: Some("fld3".to_string()),
                field_name: "Status".to_string(),
                field_type: Some("singleSelect".to_string()),
                is_key: false,
                csv_field: Some("status".to_string()),
                csv_filename: Some("space.csv".to_string()),
                sync_enabled: true,
            },
            FieldMappingRow {
                field_id: Some("fld4".to_string()),
                field_name: "Notes".to_string(),
                field_type: Some("multilineText".to_string()),
                is_key: false,
                csv_field: Some("notes".to_string()),
                csv_filename: Some("location.csv".to_string()),
                sync_enabled: false,
            },
        ];

        let prepared = prepare_compare_fields(
            &sample_config_service(),
            &app_config(),
            "assets",
            "ID",
            &fields,
        )
        .unwrap();

        assert_eq!(prepared.primary_key_csv_column, "id");
        assert_eq!(prepared.csv_filename, "location.csv");
        assert_eq!(prepared.compare_fields.len(), 1);
        assert_eq!(prepared.compare_fields[0].field_name, "Name");
        assert!(prepared.warnings.iter().any(|warning| warning.contains("Status")));
    }
}
