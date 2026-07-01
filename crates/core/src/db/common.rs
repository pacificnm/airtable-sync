//! Shared database helpers for `db init`, `db reset`, and `db migrate`.

use std::fs;
use std::path::{Path, PathBuf};

use nest_config::ConfigService;
use nest_data::{Migration, MigrationRunner, SqlMigration};
use nest_data_sqlite::{SqliteConfig, SqliteConnection, SqliteMigrationRunner};
use nest_error::{NestError, NestResult};
use serde::Serialize;

use crate::config::AppConfig;

/// Migration id applied on init and reset.
pub const MIGRATION_ID: &str = "001_initial_schema";

/// JSON response for successful `db init` with `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DbInitResult {
    /// Absolute path to the created database file.
    pub database_path: PathBuf,
    /// Absolute path to the schema file that was applied.
    pub schema: PathBuf,
    /// Migration id recorded in `_nest_migrations`.
    pub migration: String,
}

/// JSON response for successful `db reset` with `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DbResetResult {
    /// Absolute path to the recreated database file.
    pub database_path: PathBuf,
    /// Absolute path to the schema file that was applied.
    pub schema: PathBuf,
    /// Migration id recorded in `_nest_migrations`.
    pub migration: String,
    /// Whether a database file existed before reset.
    pub previous_existed: bool,
}

/// JSON response for successful `db migrate` with `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DbMigrateResult {
    /// Absolute path to the database file.
    pub database_path: PathBuf,
    /// Configured schema SQL file path.
    pub schema_file: PathBuf,
    /// Migration ids applied during this run.
    pub applied: Vec<String>,
    /// Migration ids that were pending before this run.
    pub pending_before: Vec<String>,
    /// All migration ids recorded after this run.
    pub all_applied: Vec<String>,
    /// Whether the database file was created during this run.
    pub database_created: bool,
}

/// Returns the database file and SQLite sidecar paths.
pub fn database_sidecar_paths(database_path: &Path) -> [PathBuf; 3] {
    let display = database_path.display().to_string();
    [
        database_path.to_path_buf(),
        PathBuf::from(format!("{display}-wal")),
        PathBuf::from(format!("{display}-shm")),
    ]
}

/// Deletes the database file and SQLite sidecar files, ignoring not-found errors.
pub fn remove_database_files(database_path: &Path) -> NestResult<()> {
    for path in database_sidecar_paths(database_path) {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(NestError::io(format!(
                    "failed to remove {}: {error}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

/// Opens an existing SQLite database file.
pub fn open_database(database_path: &Path) -> NestResult<SqliteConnection> {
    SqliteConnection::open(&SqliteConfig::file(database_path)).map_err(NestError::from)
}

/// Returns the product migration registry for the configured schema file.
pub fn registered_migrations(schema_path: &Path) -> NestResult<Vec<Box<dyn Migration>>> {
    let schema_sql = fs::read_to_string(schema_path).map_err(|error| {
        NestError::io(format!(
            "failed to read schema file {}: {error}",
            schema_path.display()
        ))
    })?;

    Ok(vec![Box::new(SqlMigration::new(
        MIGRATION_ID,
        schema_sql,
        initial_schema_down_sql(),
    ))])
}

/// Applies pending migrations to the database.
pub fn apply_pending_migrations(
    database_path: &Path,
    schema_path: &Path,
    create_parent: bool,
) -> NestResult<DbMigrateResult> {
    let database_created = !database_path.is_file();

    if create_parent && database_created {
        if let Some(parent) = database_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                NestError::io(format!(
                    "failed to create directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
    }

    let migrations = registered_migrations(schema_path)?;
    let registered_ids: Vec<String> = migrations.iter().map(|m| m.id().to_string()).collect();
    let conn = open_database(database_path)?;
    let runner = SqliteMigrationRunner::new(conn, migrations);

    let pending_before = runner.pending().map_err(NestError::from)?;
    let applied = pending_before.clone();
    runner.apply_all().map_err(NestError::from)?;
    let pending_after = runner.pending().map_err(NestError::from)?;

    let all_applied: Vec<String> = registered_ids
        .into_iter()
        .filter(|id| !pending_after.iter().any(|pending| pending == id))
        .collect();

    Ok(DbMigrateResult {
        database_path: absolute_path(database_path),
        schema_file: absolute_path(schema_path),
        applied,
        pending_before,
        all_applied,
        database_created,
    })
}

/// Creates the SQLite database and applies the initial schema migration.
pub fn create_database(
    _config: &ConfigService,
    _app: &AppConfig,
    database_path: &Path,
    schema_path: &Path,
) -> NestResult<DbInitResult> {
    let result = apply_pending_migrations(database_path, schema_path, true)?;

    Ok(DbInitResult {
        database_path: result.database_path,
        schema: result.schema_file,
        migration: MIGRATION_ID.to_string(),
    })
}

fn initial_schema_down_sql() -> String {
    [
        "DROP TABLE IF EXISTS csv_fields;",
        "DROP TABLE IF EXISTS airtable_fields;",
        "DROP TABLE IF EXISTS airtable_tables;",
    ]
    .join("\n")
}

pub(crate) fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const SCHEMA_SQL: &str = "CREATE TABLE notes (id INTEGER PRIMARY KEY, title TEXT NOT NULL);";

    #[test]
    fn apply_pending_migrations_applies_registered_migration() {
        let dir = tempdir().unwrap();
        let schema_path = dir.path().join("schema.sql");
        let db_path = dir.path().join("app.db");
        fs::write(&schema_path, SCHEMA_SQL).unwrap();

        let result = apply_pending_migrations(&db_path, &schema_path, true).unwrap();

        assert!(db_path.is_file());
        assert_eq!(result.applied, vec![MIGRATION_ID.to_string()]);
        assert_eq!(result.all_applied, vec![MIGRATION_ID.to_string()]);
        assert!(result.database_created);
    }

    #[test]
    fn apply_pending_migrations_is_noop_when_up_to_date() {
        let dir = tempdir().unwrap();
        let schema_path = dir.path().join("schema.sql");
        let db_path = dir.path().join("app.db");
        fs::write(&schema_path, SCHEMA_SQL).unwrap();

        apply_pending_migrations(&db_path, &schema_path, true).unwrap();
        let result = apply_pending_migrations(&db_path, &schema_path, false).unwrap();

        assert!(result.applied.is_empty());
        assert_eq!(result.all_applied, vec![MIGRATION_ID.to_string()]);
        assert!(!result.database_created);
    }
}
