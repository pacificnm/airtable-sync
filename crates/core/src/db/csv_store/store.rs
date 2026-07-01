//! Upsert repository for `csv_fields`.

use nest_data::{DataError, DataResult, Transactional};
use nest_data_sqlite::SqliteConnection;

fn sqlite_result<T>(result: rusqlite::Result<T>) -> DataResult<T> {
    result.map_err(|error| DataError::query(error.to_string()))
}

/// One row in `csv_fields`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvFieldRow {
    /// Source CSV file name (basename).
    pub filename: String,
    /// Trimmed original CSV header text.
    pub name: String,
    /// Lowercase normalized header for mapping.
    pub normalized_name: String,
}

/// SQLite store for imported CSV column headers.
pub struct CsvStore {
    db: SqliteConnection,
}

impl CsvStore {
    /// Creates a store over an open SQLite connection.
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    /// Replaces all cached CSV fields in a single transaction.
    pub fn replace_fields(&self, fields: &[CsvFieldRow]) -> DataResult<usize> {
        let transaction = self.db.begin()?;
        self.db.with_connection(|conn| {
            sqlite_result(conn.execute("DELETE FROM csv_fields", []))?;
            Ok(())
        })?;

        for field in fields {
            self.db.with_connection(|conn| {
                sqlite_result(conn.execute(
                    "INSERT INTO csv_fields (filename, name, normalized_name) VALUES (?1, ?2, ?3)",
                    rusqlite::params![field.filename, field.name, field.normalized_name],
                ))?;
                Ok(())
            })?;
        }

        transaction.commit()?;
        Ok(fields.len())
    }

    /// Returns cached CSV fields ordered by filename then normalized name.
    pub fn list_fields(&self) -> DataResult<Vec<CsvFieldRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT filename, name, normalized_name
                 FROM csv_fields
                 ORDER BY filename, normalized_name",
            ))?;
            let rows = sqlite_result(stmt.query_map([], |row| {
                Ok(CsvFieldRow {
                    filename: row.get(0)?,
                    name: row.get(1)?,
                    normalized_name: row.get(2)?,
                })
            }))?;
            let mut fields = Vec::new();
            for row in rows {
                fields.push(sqlite_result(row)?);
            }
            Ok(fields)
        })
    }

    /// Returns cached CSV fields with the given normalized name.
    pub fn find_by_normalized_name(
        &self,
        normalized_name: &str,
    ) -> DataResult<Vec<CsvFieldRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT filename, name, normalized_name
                 FROM csv_fields
                 WHERE normalized_name = ?1
                 ORDER BY filename",
            ))?;
            let rows = sqlite_result(stmt.query_map([normalized_name], |row| {
                Ok(CsvFieldRow {
                    filename: row.get(0)?,
                    name: row.get(1)?,
                    normalized_name: row.get(2)?,
                })
            }))?;
            let mut fields = Vec::new();
            for row in rows {
                fields.push(sqlite_result(row)?);
            }
            Ok(fields)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_data_sqlite::SqliteConfig;

    const SCHEMA_SQL: &str = include_str!("../../../../../schema/airtable-sync.sql");

    fn memory_store() -> CsvStore {
        let db = SqliteConnection::open(&SqliteConfig::memory()).unwrap();
        db.with_connection(|conn| sqlite_result(conn.execute_batch(SCHEMA_SQL)))
            .unwrap();
        CsvStore::new(db)
    }

    #[test]
    fn replace_fields_clears_previous_rows() {
        let store = memory_store();
        store
            .replace_fields(&[
                CsvFieldRow {
                    filename: "location.csv".to_string(),
                    name: "ID".to_string(),
                    normalized_name: "id".to_string(),
                },
                CsvFieldRow {
                    filename: "location.csv".to_string(),
                    name: "Name".to_string(),
                    normalized_name: "name".to_string(),
                },
            ])
            .unwrap();

        store
            .replace_fields(&[CsvFieldRow {
                filename: "space.csv".to_string(),
                name: "space_name".to_string(),
                normalized_name: "space_name".to_string(),
            }])
            .unwrap();

        let fields = store.list_fields().unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].filename, "space.csv");
    }

    #[test]
    fn list_fields_allows_same_normalized_name_in_different_files() {
        let store = memory_store();
        store
            .replace_fields(&[
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
            ])
            .unwrap();

        let fields = store.list_fields().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].filename, "location.csv");
        assert_eq!(fields[1].filename, "space.csv");
    }

    #[test]
    fn find_by_normalized_name_returns_rows_from_multiple_files() {
        let store = memory_store();
        store
            .replace_fields(&[
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
            ])
            .unwrap();

        let matches = store.find_by_normalized_name("id").unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn list_fields_orders_by_filename_then_normalized_name() {
        let store = memory_store();
        store
            .replace_fields(&[
                CsvFieldRow {
                    filename: "space.csv".to_string(),
                    name: "Zebra".to_string(),
                    normalized_name: "zebra".to_string(),
                },
                CsvFieldRow {
                    filename: "location.csv".to_string(),
                    name: "Alpha".to_string(),
                    normalized_name: "alpha".to_string(),
                },
            ])
            .unwrap();

        let fields = store.list_fields().unwrap();
        assert_eq!(fields[0].filename, "location.csv");
        assert_eq!(fields[1].filename, "space.csv");
    }
}
