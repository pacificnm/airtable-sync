//! Shared compare logic for single-table operations.

use std::path::PathBuf;

use nest_airtable::{AirtableClient, AirtableListParams};
use nest_config::ConfigService;
use nest_error::{NestError, NestResult};
use nest_file::FileService;

use crate::compare::csv_index::index_csv_by_key;
use crate::compare::diff::{compare_records, CompareField};
use crate::compare::table::{CompareTableResult, CompareTableView};
use crate::config::{resolve_config_path, AppConfig, ValidatedConfig};
use crate::csv::resolve_csv_path_by_filename;
use crate::db::{absolute_path, FieldMappingRow, SchemaStore};

struct PreparedCompareFields {
    csv_filename: String,
    csv_path: PathBuf,
    primary_key_csv_column: String,
    compare_fields: Vec<CompareField>,
    airtable_field_names: Vec<String>,
    warnings: Vec<String>,
}

/// Compares one table using a shared Airtable client.
pub async fn compare_single_table(
    validated: &ValidatedConfig,
    store: &SchemaStore,
    files: &FileService,
    client: &AirtableClient,
    table_name: &str,
    quiet: bool,
) -> NestResult<CompareTableResult> {
    let database_path =
        resolve_config_path(&validated.config, &validated.app.database.database_path);

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

    let csv_index = index_csv_by_key(
        files,
        &prepared.csv_path,
        &prepared.primary_key_csv_column,
    )?;

    if !quiet {
        for warning in &csv_index.warnings {
            println!("warning: {warning}");
        }
        if csv_index.skipped_empty_keys > 0 {
            println!(
                "warning: skipped {} CSV row(s) with empty primary key in {}",
                csv_index.skipped_empty_keys, table_name
            );
        }
    }

    let mut params = AirtableListParams::default();
    params.fields = Some(prepared.airtable_field_names.clone());
    let airtable_records = client.list_all_records(table_name, params).await?;

    let compare = compare_records(
        &csv_index,
        &airtable_records,
        primary_key_field,
        &prepared.compare_fields,
    );

    Ok(CompareTableResult {
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
    })
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
