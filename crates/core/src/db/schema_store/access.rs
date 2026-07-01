//! Database access guards for the Airtable schema cache.

use std::path::Path;

use nest_error::{NestError, NestResult};
use rusqlite::Connection;

use crate::db::MIGRATION_ID;

/// Ensures the SQLite database exists and contains the schema cache tables.
pub fn ensure_schema_cache(database_path: &Path) -> NestResult<()> {
    if !database_path.is_file() {
        return Err(NestError::data(format!(
            "database file not found: {}",
            database_path.display()
        ))
        .with_help("Run `db init` to create the SQLite database first."));
    }

    let conn = Connection::open(database_path).map_err(|error| {
        NestError::data(format!(
            "failed to open database {}: {error}",
            database_path.display()
        ))
    })?;
    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'airtable_tables'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| NestError::data(format!("failed to inspect database schema: {error}")))?;
    if exists == 0 {
        return Err(NestError::data(format!(
            "database is missing airtable_tables (migration {MIGRATION_ID} not applied)"
        ))
        .with_help("Run `db init` or `db migrate` to apply the schema."));
    }
    Ok(())
}
