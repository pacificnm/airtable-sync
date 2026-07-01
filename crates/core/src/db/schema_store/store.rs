//! Upsert repositories for `airtable_tables` and `airtable_fields`.

use nest_data::{DataError, DataResult, Transactional};
use nest_data_sqlite::SqliteConnection;

fn sqlite_result<T>(result: rusqlite::Result<T>) -> DataResult<T> {
    result.map_err(|error| DataError::query(error.to_string()))
}

/// One row in `airtable_tables`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirtableTableRow {
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

/// One cached table row with a field count for list commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirtableTableSummary {
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

/// One row in `airtable_fields`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirtableFieldRow {
    /// Airtable table id (`tbl…`).
    pub table_id: String,
    /// Airtable field id (`fld…`).
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
    pub csv_filename: Option<String>,
}

/// Counts returned after a schema pull transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaPullStats {
    /// Number of table rows upserted.
    pub tables_updated: usize,
    /// Number of field rows upserted.
    pub fields_upserted: usize,
}

/// One mappable field row for mapping list output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMappingRow {
    /// Airtable field id (`fld…`), when known.
    pub field_id: Option<String>,
    /// Field display name.
    pub field_name: String,
    /// Airtable field type.
    pub field_type: Option<String>,
    /// Whether this field is the table primary key.
    pub is_key: bool,
    /// Mapped CSV column name, if any.
    pub csv_field: Option<String>,
    /// Source CSV file basename for the mapped column, if any.
    pub csv_filename: Option<String>,
    /// Whether field sync is enabled.
    pub sync_enabled: bool,
}

/// Mapping columns to update on one Airtable field row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMappingUpdate {
    /// Normalized CSV column name to store in `csv_field`.
    pub csv_field: String,
    /// Source CSV file basename to store in `csv_filename`.
    pub csv_filename: String,
    /// When set, updates `sync_enabled`; when `None`, leaves it unchanged.
    pub sync_enabled: Option<bool>,
}

/// SQLite store for cached Airtable schema metadata.
pub struct SchemaStore {
    db: SqliteConnection,
}

impl SchemaStore {
    /// Creates a store over an open SQLite connection.
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    /// Upserts all tables and fields from a pull in a single transaction.
    pub fn replace_schema_for_pull(
        &self,
        tables: &[AirtableTableRow],
        fields: &[AirtableFieldRow],
    ) -> DataResult<SchemaPullStats> {
        let transaction = self.db.begin()?;
        let mut tables_updated = 0usize;
        let mut fields_upserted = 0usize;

        for table in tables {
            self.upsert_table(table)?;
            tables_updated += 1;
        }
        for field in fields {
            self.upsert_field(field)?;
            fields_upserted += 1;
        }

        transaction.commit()?;
        Ok(SchemaPullStats {
            tables_updated,
            fields_upserted,
        })
    }

    /// Inserts or updates one table row by `table_id`.
    pub fn upsert_table(&self, row: &AirtableTableRow) -> DataResult<()> {
        self.db.with_connection(|conn| {
            sqlite_result(conn.execute(
                "INSERT INTO airtable_tables (name, table_id, enabled, allow_create, allow_update)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(table_id) DO UPDATE SET
                   name = excluded.name,
                   enabled = excluded.enabled",
                rusqlite::params![
                    row.name,
                    row.table_id,
                    row.enabled as i32,
                    row.allow_create as i32,
                    row.allow_update as i32,
                ],
            ))?;
            Ok(())
        })
    }

    /// Inserts or updates one field row, preserving mapping columns on conflict.
    pub fn upsert_field(&self, row: &AirtableFieldRow) -> DataResult<()> {
        self.db.with_connection(|conn| {
            sqlite_result(conn.execute(
                "INSERT INTO airtable_fields (
                   table_id, field_id, field_name, field_type, is_computed, is_key,
                   sync_enabled, csv_field, csv_filename
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(table_id, field_name) DO UPDATE SET
                   field_id = excluded.field_id,
                   field_type = excluded.field_type,
                   is_computed = excluded.is_computed,
                   is_key = excluded.is_key",
                rusqlite::params![
                    row.table_id,
                    row.field_id,
                    row.field_name,
                    row.field_type,
                    row.is_computed as i32,
                    row.is_key as i32,
                    row.sync_enabled as i32,
                    row.csv_field,
                    row.csv_filename,
                ],
            ))?;
            Ok(())
        })
    }

    /// Returns all cached table rows ordered by name.
    pub fn list_tables(&self) -> DataResult<Vec<AirtableTableRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT name, table_id, enabled, allow_create, allow_update
                 FROM airtable_tables
                 ORDER BY name",
            ))?;
            let rows = sqlite_result(stmt.query_map([], |row| {
                Ok(AirtableTableRow {
                    name: row.get(0)?,
                    table_id: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    allow_create: row.get::<_, i32>(3)? != 0,
                    allow_update: row.get::<_, i32>(4)? != 0,
                })
            }))?;
            let mut tables = Vec::new();
            for row in rows {
                tables.push(sqlite_result(row)?);
            }
            Ok(tables)
        })
    }

    /// Returns cached tables with field counts ordered by name.
    pub fn list_tables_summary(&self) -> DataResult<Vec<AirtableTableSummary>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT t.name, t.table_id, t.enabled, t.allow_create, t.allow_update,
                        COUNT(f.id) AS field_count
                 FROM airtable_tables t
                 LEFT JOIN airtable_fields f ON f.table_id = t.table_id
                 GROUP BY t.id
                 ORDER BY t.name",
            ))?;
            let rows = sqlite_result(stmt.query_map([], |row| {
                Ok(AirtableTableSummary {
                    name: row.get(0)?,
                    table_id: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    allow_create: row.get::<_, i32>(3)? != 0,
                    allow_update: row.get::<_, i32>(4)? != 0,
                    field_count: row.get::<_, i64>(5)? as usize,
                })
            }))?;
            let mut tables = Vec::new();
            for row in rows {
                tables.push(sqlite_result(row)?);
            }
            Ok(tables)
        })
    }

    /// Returns one cached table row by logical name, if present.
    pub fn find_table_by_name(&self, name: &str) -> DataResult<Option<AirtableTableRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT name, table_id, enabled, allow_create, allow_update
                 FROM airtable_tables
                 WHERE name = ?1",
            ))?;
            let mut rows = sqlite_result(stmt.query_map([name], |row| {
                Ok(AirtableTableRow {
                    name: row.get(0)?,
                    table_id: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    allow_create: row.get::<_, i32>(3)? != 0,
                    allow_update: row.get::<_, i32>(4)? != 0,
                })
            }))?;
            match rows.next() {
                Some(row) => Ok(Some(sqlite_result(row)?)),
                None => Ok(None),
            }
        })
    }

    /// Returns cached field rows for a table id ordered by field name.
    pub fn list_fields(&self, table_id: &str) -> DataResult<Vec<AirtableFieldRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT table_id, field_id, field_name, field_type, is_computed, is_key,
                        sync_enabled, csv_field, csv_filename
                 FROM airtable_fields
                 WHERE table_id = ?1
                 ORDER BY field_name",
            ))?;
            let rows = sqlite_result(stmt.query_map([table_id], |row| {
                Ok(AirtableFieldRow {
                    table_id: row.get(0)?,
                    field_id: row.get(1)?,
                    field_name: row.get(2)?,
                    field_type: row.get(3)?,
                    is_computed: row.get::<_, i32>(4)? != 0,
                    is_key: row.get::<_, i32>(5)? != 0,
                    sync_enabled: row.get::<_, i32>(6)? != 0,
                    csv_field: row.get(7)?,
                    csv_filename: row.get(8)?,
                })
            }))?;
            let mut fields = Vec::new();
            for row in rows {
                fields.push(sqlite_result(row)?);
            }
            Ok(fields)
        })
    }

    /// Returns non-computed cached fields for mapping display.
    pub fn list_mappable_fields(&self, table_id: &str) -> DataResult<Vec<FieldMappingRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT field_id, field_name, field_type, is_key, csv_field, csv_filename,
                        sync_enabled
                 FROM airtable_fields
                 WHERE table_id = ?1 AND is_computed = 0
                 ORDER BY field_name",
            ))?;
            let rows = sqlite_result(stmt.query_map([table_id], |row| {
                Ok(FieldMappingRow {
                    field_id: row.get(0)?,
                    field_name: row.get(1)?,
                    field_type: row.get(2)?,
                    is_key: row.get::<_, i32>(3)? != 0,
                    csv_field: row.get(4)?,
                    csv_filename: row.get(5)?,
                    sync_enabled: row.get::<_, i32>(6)? != 0,
                })
            }))?;
            let mut fields = Vec::new();
            for row in rows {
                fields.push(sqlite_result(row)?);
            }
            Ok(fields)
        })
    }

    /// Returns one cached field row by table id and field name.
    pub fn find_field_by_name(
        &self,
        table_id: &str,
        field_name: &str,
    ) -> DataResult<Option<AirtableFieldRow>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT table_id, field_id, field_name, field_type, is_computed, is_key,
                        sync_enabled, csv_field, csv_filename
                 FROM airtable_fields
                 WHERE table_id = ?1 AND field_name = ?2",
            ))?;
            let mut rows = sqlite_result(stmt.query_map((table_id, field_name), |row| {
                Ok(AirtableFieldRow {
                    table_id: row.get(0)?,
                    field_id: row.get(1)?,
                    field_name: row.get(2)?,
                    field_type: row.get(3)?,
                    is_computed: row.get::<_, i32>(4)? != 0,
                    is_key: row.get::<_, i32>(5)? != 0,
                    sync_enabled: row.get::<_, i32>(6)? != 0,
                    csv_field: row.get(7)?,
                    csv_filename: row.get(8)?,
                })
            }))?;
            match rows.next() {
                Some(row) => Ok(Some(sqlite_result(row)?)),
                None => Ok(None),
            }
        })
    }

    /// Updates mapping columns on one non-computed field row.
    pub fn set_field_mapping(
        &self,
        table_id: &str,
        field_name: &str,
        update: &FieldMappingUpdate,
    ) -> DataResult<bool> {
        self.db.with_connection(|conn| {
            let rows_affected = match update.sync_enabled {
                Some(sync_enabled) => sqlite_result(conn.execute(
                    "UPDATE airtable_fields
                     SET csv_field = ?1, csv_filename = ?2, sync_enabled = ?3
                     WHERE table_id = ?4 AND field_name = ?5 AND is_computed = 0",
                    rusqlite::params![
                        update.csv_field,
                        update.csv_filename,
                        sync_enabled as i32,
                        table_id,
                        field_name,
                    ],
                ))?,
                None => sqlite_result(conn.execute(
                    "UPDATE airtable_fields
                     SET csv_field = ?1, csv_filename = ?2
                     WHERE table_id = ?3 AND field_name = ?4 AND is_computed = 0",
                    rusqlite::params![
                        update.csv_field,
                        update.csv_filename,
                        table_id,
                        field_name,
                    ],
                ))?,
            };
            Ok(rows_affected > 0)
        })
    }

    /// Clears mapping columns on one non-computed field row.
    pub fn clear_field_mapping(
        &self,
        table_id: &str,
        field_name: &str,
    ) -> DataResult<bool> {
        self.db.with_connection(|conn| {
            let rows_affected = sqlite_result(conn.execute(
                "UPDATE airtable_fields
                 SET csv_field = NULL, csv_filename = NULL, sync_enabled = 0
                 WHERE table_id = ?1 AND field_name = ?2 AND is_computed = 0",
                rusqlite::params![table_id, field_name],
            ))?;
            Ok(rows_affected > 0)
        })
    }

    /// Updates `sync_enabled` on one non-computed field row without changing mapping columns.
    pub fn set_field_sync_enabled(
        &self,
        table_id: &str,
        field_name: &str,
        sync_enabled: bool,
    ) -> DataResult<bool> {
        self.db.with_connection(|conn| {
            let rows_affected = sqlite_result(conn.execute(
                "UPDATE airtable_fields
                 SET sync_enabled = ?1
                 WHERE table_id = ?2 AND field_name = ?3 AND is_computed = 0",
                rusqlite::params![sync_enabled as i32, table_id, field_name],
            ))?;
            Ok(rows_affected > 0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_data_sqlite::SqliteConfig;

    const SCHEMA_SQL: &str = include_str!("../../../../../schema/airtable-sync.sql");

    fn memory_store() -> SchemaStore {
        let db = SqliteConnection::open(&SqliteConfig::memory()).unwrap();
        db.with_connection(|conn| sqlite_result(conn.execute_batch(SCHEMA_SQL)))
            .unwrap();
        SchemaStore::new(db)
    }

    #[test]
    fn upsert_field_preserves_csv_field_on_repull() {
        let store = memory_store();
        store
            .upsert_table(&AirtableTableRow {
                name: "assets".to_string(),
                table_id: "tblTEST".to_string(),
                enabled: true,
                allow_create: false,
                allow_update: true,
            })
            .unwrap();
        store
            .upsert_field(&AirtableFieldRow {
                table_id: "tblTEST".to_string(),
                field_id: Some("fldOLD".to_string()),
                field_name: "Name".to_string(),
                field_type: Some("singleLineText".to_string()),
                is_computed: false,
                is_key: true,
                sync_enabled: true,
                csv_field: Some("name".to_string()),
                csv_filename: Some("location.csv".to_string()),
            })
            .unwrap();

        store
            .upsert_field(&AirtableFieldRow {
                table_id: "tblTEST".to_string(),
                field_id: Some("fldNEW".to_string()),
                field_name: "Name".to_string(),
                field_type: Some("email".to_string()),
                is_computed: false,
                is_key: true,
                sync_enabled: false,
                csv_field: None,
                csv_filename: None,
            })
            .unwrap();

        let fields = store.list_fields("tblTEST").unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_id.as_deref(), Some("fldNEW"));
        assert_eq!(fields[0].field_type.as_deref(), Some("email"));
        assert_eq!(fields[0].csv_field.as_deref(), Some("name"));
        assert_eq!(fields[0].csv_filename.as_deref(), Some("location.csv"));
        assert!(fields[0].sync_enabled);
    }

    #[test]
    fn find_table_by_name_returns_cached_row() {
        let store = memory_store();
        store
            .upsert_table(&AirtableTableRow {
                name: "assets".to_string(),
                table_id: "tblTEST".to_string(),
                enabled: true,
                allow_create: false,
                allow_update: true,
            })
            .unwrap();

        let table = store.find_table_by_name("assets").unwrap();
        assert_eq!(
            table,
            Some(AirtableTableRow {
                name: "assets".to_string(),
                table_id: "tblTEST".to_string(),
                enabled: true,
                allow_create: false,
                allow_update: true,
            })
        );
        assert_eq!(store.find_table_by_name("missing").unwrap(), None);
    }

    #[test]
    fn list_tables_summary_includes_field_count() {
        let store = memory_store();
        store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[
                    AirtableFieldRow {
                        table_id: "tblTEST".to_string(),
                        field_id: Some("fld1".to_string()),
                        field_name: "Name".to_string(),
                        field_type: Some("singleLineText".to_string()),
                        is_computed: false,
                        is_key: true,
                        sync_enabled: false,
                        csv_field: None,
                        csv_filename: None,
                    },
                    AirtableFieldRow {
                        table_id: "tblTEST".to_string(),
                        field_id: Some("fld2".to_string()),
                        field_name: "Total".to_string(),
                        field_type: Some("formula".to_string()),
                        is_computed: true,
                        is_key: false,
                        sync_enabled: false,
                        csv_field: None,
                        csv_filename: None,
                    },
                ],
            )
            .unwrap();

        let summary = store.list_tables_summary().unwrap();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].name, "assets");
        assert_eq!(summary[0].field_count, 2);
    }

    #[test]
    fn list_mappable_fields_excludes_computed_and_includes_unmapped() {
        let store = memory_store();
        store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[
                    AirtableFieldRow {
                        table_id: "tblTEST".to_string(),
                        field_id: Some("fld1".to_string()),
                        field_name: "Name".to_string(),
                        field_type: Some("singleLineText".to_string()),
                        is_computed: false,
                        is_key: true,
                        sync_enabled: true,
                        csv_field: Some("name".to_string()),
                        csv_filename: Some("location.csv".to_string()),
                    },
                    AirtableFieldRow {
                        table_id: "tblTEST".to_string(),
                        field_id: Some("fld2".to_string()),
                        field_name: "Status".to_string(),
                        field_type: Some("singleSelect".to_string()),
                        is_computed: false,
                        is_key: false,
                        sync_enabled: false,
                        csv_field: None,
                        csv_filename: None,
                    },
                    AirtableFieldRow {
                        table_id: "tblTEST".to_string(),
                        field_id: Some("fld3".to_string()),
                        field_name: "Total".to_string(),
                        field_type: Some("formula".to_string()),
                        is_computed: true,
                        is_key: false,
                        sync_enabled: false,
                        csv_field: None,
                        csv_filename: None,
                    },
                ],
            )
            .unwrap();

        let fields = store.list_mappable_fields("tblTEST").unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].field_name, "Name");
        assert_eq!(fields[0].csv_field.as_deref(), Some("name"));
        assert_eq!(fields[0].csv_filename.as_deref(), Some("location.csv"));
        assert_eq!(fields[1].field_name, "Status");
        assert!(fields[1].csv_field.is_none());
    }

    #[test]
    fn set_field_mapping_updates_csv_field_and_preserves_sync_by_default() {
        let store = memory_store();
        store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[AirtableFieldRow {
                    table_id: "tblTEST".to_string(),
                    field_id: Some("fld1".to_string()),
                    field_name: "Name".to_string(),
                    field_type: Some("singleLineText".to_string()),
                    is_computed: false,
                    is_key: true,
                    sync_enabled: true,
                    csv_field: None,
                    csv_filename: None,
                }],
            )
            .unwrap();

        let updated = store
            .set_field_mapping(
                "tblTEST",
                "Name",
                &FieldMappingUpdate {
                    csv_field: "name".to_string(),
                    csv_filename: "location.csv".to_string(),
                    sync_enabled: None,
                },
            )
            .unwrap();
        assert!(updated);

        let field = store.find_field_by_name("tblTEST", "Name").unwrap().unwrap();
        assert_eq!(field.csv_field.as_deref(), Some("name"));
        assert_eq!(field.csv_filename.as_deref(), Some("location.csv"));
        assert!(field.sync_enabled);
    }

    #[test]
    fn set_field_mapping_can_toggle_sync_enabled() {
        let store = memory_store();
        store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[AirtableFieldRow {
                    table_id: "tblTEST".to_string(),
                    field_id: Some("fld1".to_string()),
                    field_name: "Name".to_string(),
                    field_type: Some("singleLineText".to_string()),
                    is_computed: false,
                    is_key: true,
                    sync_enabled: false,
                    csv_field: Some("name".to_string()),
                    csv_filename: Some("location.csv".to_string()),
                }],
            )
            .unwrap();

        store
            .set_field_mapping(
                "tblTEST",
                "Name",
                &FieldMappingUpdate {
                    csv_field: "name".to_string(),
                    csv_filename: "location.csv".to_string(),
                    sync_enabled: Some(true),
                },
            )
            .unwrap();

        let field = store.find_field_by_name("tblTEST", "Name").unwrap().unwrap();
        assert!(field.sync_enabled);
        assert_eq!(field.csv_filename.as_deref(), Some("location.csv"));
    }

    #[test]
    fn replace_schema_for_pull_is_atomic() {
        let store = memory_store();
        let stats = store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[AirtableFieldRow {
                    table_id: "tblTEST".to_string(),
                    field_id: Some("fld1".to_string()),
                    field_name: "Name".to_string(),
                    field_type: Some("singleLineText".to_string()),
                    is_computed: false,
                    is_key: true,
                    sync_enabled: false,
                    csv_field: None,
                    csv_filename: None,
                }],
            )
            .unwrap();
        assert_eq!(stats.tables_updated, 1);
        assert_eq!(stats.fields_upserted, 1);
        assert_eq!(store.list_tables().unwrap().len(), 1);
    }

    #[test]
    fn clear_field_mapping_clears_csv_columns_and_disables_sync() {
        let store = memory_store();
        store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[AirtableFieldRow {
                    table_id: "tblTEST".to_string(),
                    field_id: Some("fld1".to_string()),
                    field_name: "Name".to_string(),
                    field_type: Some("singleLineText".to_string()),
                    is_computed: false,
                    is_key: true,
                    sync_enabled: true,
                    csv_field: Some("name".to_string()),
                    csv_filename: Some("location.csv".to_string()),
                }],
            )
            .unwrap();

        assert!(store.clear_field_mapping("tblTEST", "Name").unwrap());

        let field = store.find_field_by_name("tblTEST", "Name").unwrap().unwrap();
        assert!(field.csv_field.is_none());
        assert!(field.csv_filename.is_none());
        assert!(!field.sync_enabled);
    }

    #[test]
    fn set_field_sync_enabled_preserves_csv_mapping_columns() {
        let store = memory_store();
        store
            .replace_schema_for_pull(
                &[AirtableTableRow {
                    name: "assets".to_string(),
                    table_id: "tblTEST".to_string(),
                    enabled: true,
                    allow_create: false,
                    allow_update: true,
                }],
                &[AirtableFieldRow {
                    table_id: "tblTEST".to_string(),
                    field_id: Some("fld1".to_string()),
                    field_name: "Name".to_string(),
                    field_type: Some("singleLineText".to_string()),
                    is_computed: false,
                    is_key: true,
                    sync_enabled: false,
                    csv_field: Some("name".to_string()),
                    csv_filename: Some("location.csv".to_string()),
                }],
            )
            .unwrap();

        assert!(store.set_field_sync_enabled("tblTEST", "Name", true).unwrap());

        let field = store.find_field_by_name("tblTEST", "Name").unwrap().unwrap();
        assert!(field.sync_enabled);
        assert_eq!(field.csv_field.as_deref(), Some("name"));
        assert_eq!(field.csv_filename.as_deref(), Some("location.csv"));
    }
}
