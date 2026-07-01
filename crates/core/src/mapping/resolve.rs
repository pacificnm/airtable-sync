//! CSV column resolution for mapping commands.

use std::collections::HashSet;

use nest_config::ConfigService;
use nest_error::{NestError, NestResult};
use nest_file_csv::normalize_header;

use crate::config::AppConfig;
use crate::csv::{csv_filename, resolve_csv_path, CsvFileRole};
use crate::db::CsvStore;

/// Resolved CSV column for a field mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCsvColumn {
    /// Normalized CSV column name stored in `csv_field`.
    pub normalized_name: String,
    /// Source CSV file basename.
    pub filename: String,
}

/// Resolves and validates a CSV column against imported `csv_fields`.
pub fn resolve_csv_column(
    csv_store: &CsvStore,
    config: &ConfigService,
    app: &AppConfig,
    csv_column: &str,
    csv_file: Option<CsvFileRole>,
) -> NestResult<ResolvedCsvColumn> {
    let normalized_name = normalize_header(csv_column, true, true);
    if normalized_name.is_empty() {
        return Err(NestError::validation("csv column name must not be empty"));
    }

    let matches = csv_store
        .find_by_normalized_name(&normalized_name)
        .map_err(NestError::from)?;

    if matches.is_empty() {
        return Err(NestError::validation(format!(
            "CSV column `{csv_column}` (normalized `{normalized_name}`) not found in cache"
        ))
        .with_help("Run `csv import-headers` to refresh imported CSV columns."));
    }

    let unique_filenames: HashSet<_> = matches.iter().map(|row| row.filename.as_str()).collect();
    if unique_filenames.len() == 1 {
        return Ok(ResolvedCsvColumn {
            normalized_name,
            filename: matches[0].filename.clone(),
        });
    }

    let Some(role) = csv_file else {
        return Err(NestError::validation(format!(
            "CSV column `{normalized_name}` exists in multiple files — pass --csv-file location or --csv-file space"
        )));
    };

    let expected_filename = csv_filename(&resolve_csv_path(config, app, role));
    let Some(selected) = matches
        .iter()
        .find(|row| row.filename == expected_filename)
    else {
        return Err(NestError::validation(format!(
            "CSV column `{normalized_name}` was not found in {} CSV ({expected_filename})",
            role.as_str()
        )));
    };

    Ok(ResolvedCsvColumn {
        normalized_name,
        filename: selected.filename.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use nest_config::{ConfigDocument, ConfigService, LoadedConfig, ConfigSource};
    use nest_data_sqlite::SqliteConfig;

    use crate::db::CsvFieldRow;

    const SCHEMA_SQL: &str = include_str!("../../../../schema/airtable-sync.sql");

    fn csv_store_with(rows: &[CsvFieldRow]) -> CsvStore {
        let db = nest_data_sqlite::SqliteConnection::open(&SqliteConfig::memory()).unwrap();
        db.with_connection(|conn| {
            conn.execute_batch(SCHEMA_SQL).unwrap();
            Ok(())
        })
        .unwrap();
        let store = CsvStore::new(db);
        store.replace_fields(rows).unwrap();
        store
    }

    fn app_config() -> AppConfig {
        AppConfig {
            airtable: crate::config::AirtableSection {
                api_url: None,
                meta_api_url: None,
                token: Some("pat-test".to_string()),
                token_env: None,
                base_id: "appTEST".to_string(),
                tables: Default::default(),
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

    fn sample_config_service() -> ConfigService {
        ConfigService::new(LoadedConfig {
            document: ConfigDocument::empty(),
            source: ConfigSource::Memory(ConfigDocument::empty()),
            path: Some(PathBuf::from("/tmp/config.toml")),
        })
    }

    #[test]
    fn resolve_csv_column_returns_unique_match() {
        let store = csv_store_with(&[CsvFieldRow {
            filename: "location.csv".to_string(),
            name: "Name".to_string(),
            normalized_name: "name".to_string(),
        }]);
        let app = app_config();
        let config = sample_config_service();

        let resolved = resolve_csv_column(&store, &config, &app, " Name ", None).unwrap();
        assert_eq!(resolved.normalized_name, "name");
        assert_eq!(resolved.filename, "location.csv");
    }

    #[test]
    fn resolve_csv_column_requires_csv_file_when_ambiguous() {
        let store = csv_store_with(&[
            CsvFieldRow {
                filename: "location.csv".to_string(),
                name: "id".to_string(),
                normalized_name: "id".to_string(),
            },
            CsvFieldRow {
                filename: "space.csv".to_string(),
                name: "ID".to_string(),
                normalized_name: "id".to_string(),
            },
        ]);
        let app = app_config();
        let config = sample_config_service();

        let error = resolve_csv_column(&store, &config, &app, "id", None).unwrap_err();
        assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);

        let resolved =
            resolve_csv_column(&store, &config, &app, "id", Some(CsvFileRole::Space)).unwrap();
        assert_eq!(resolved.filename, "space.csv");
    }
}
