//! Read/write helpers for persisted sync change plans.

use std::collections::HashMap;
use std::path::Path;

use nest_data::{DataError, DataResult, Transactional};
use nest_data_sqlite::SqliteConnection;
use nest_error::{NestError, NestResult};
use rusqlite::Connection;
use serde::Serialize;

use crate::db::CHANGE_PLANS_MIGRATION_ID;

/// Plan status: active change plan from the latest dry-run.
pub const PLAN_STATUS_DRAFT: &str = "draft";
/// Plan status: replaced by a newer dry-run.
pub const PLAN_STATUS_SUPERSEDED: &str = "superseded";
/// Plan status: all operations applied or resolved.
pub const PLAN_STATUS_APPLIED: &str = "applied";

/// Operation status: awaiting review.
pub const OPERATION_STATUS_PENDING: &str = "pending";
/// Operation status: approved for apply.
pub const OPERATION_STATUS_APPROVED: &str = "approved";
/// Operation status: rejected by reviewer.
pub const OPERATION_STATUS_DENIED: &str = "denied";
/// Operation status: successfully pushed to Airtable.
pub const OPERATION_STATUS_APPLIED: &str = "applied";
/// Operation status: Airtable update failed.
pub const OPERATION_STATUS_FAILED: &str = "failed";

fn sqlite_result<T>(result: rusqlite::Result<T>) -> DataResult<T> {
    result.map_err(|error| DataError::query(error.to_string()))
}

/// One field change in a planned update operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangePlanFieldChange {
    /// Airtable field name.
    pub field_name: String,
    /// Current value in Airtable.
    pub old_value: String,
    /// Proposed value from CSV.
    pub new_value: String,
}

/// One planned sync operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangePlanOperation {
    /// Logical table name from config.
    pub table_name: String,
    /// Operation kind (`update` in v1).
    pub operation: String,
    /// Primary key value.
    pub record_key: String,
    /// Airtable record id (`rec…`) for updates.
    pub airtable_record_id: Option<String>,
    /// Field-level changes for this operation.
    pub field_changes: Vec<ChangePlanFieldChange>,
}

/// Operation counts grouped by review status.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ChangePlanStatusCounts {
    /// Operations awaiting review.
    pub pending: usize,
    /// Operations approved for apply.
    pub approved: usize,
    /// Operations denied by reviewer.
    pub denied: usize,
    /// Operations successfully applied to Airtable.
    pub applied: usize,
    /// Operations that failed during apply.
    pub failed: usize,
}

/// Change plan metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangePlanHeader {
    /// Plan id.
    pub id: i64,
    /// Creation timestamp (SQLite datetime).
    pub created_at: String,
    /// Airtable base id.
    pub base_id: String,
    /// Plan status (`draft` or `superseded`).
    pub status: String,
    /// Tables included when the plan was created.
    pub tables_planned: usize,
    /// Total operations when the plan was created.
    pub operations_total: usize,
    /// Current operation counts by review status.
    pub status_counts: ChangePlanStatusCounts,
}

/// One persisted operation with review status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangePlanOperationView {
    /// Operation row id.
    pub operation_id: i64,
    /// Logical table name from config.
    pub table_name: String,
    /// Operation kind (`update` in v1).
    pub operation: String,
    /// Primary key value.
    pub record_key: String,
    /// Airtable record id (`rec…`) for updates.
    pub airtable_record_id: Option<String>,
    /// Review status (`pending`, `approved`, `denied`).
    pub status: String,
    /// Field-level changes for this operation.
    pub field_changes: Vec<ChangePlanFieldChange>,
}

/// Full change plan with operations and field changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangePlanDetail {
    /// Plan metadata.
    pub plan: ChangePlanHeader,
    /// Operations ordered by id.
    pub operations: Vec<ChangePlanOperationView>,
}

/// SQLite store for persisted change plans.
pub struct ChangePlanStore {
    db: SqliteConnection,
}

impl ChangePlanStore {
    /// Creates a store over an open SQLite connection.
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    /// Marks all draft plans for a base as superseded.
    pub fn supersede_draft_plans(&self, base_id: &str) -> DataResult<usize> {
        self.db.with_connection(|conn| {
            let updated = sqlite_result(conn.execute(
                "UPDATE change_plans SET status = ?1 WHERE base_id = ?2 AND status = ?3",
                rusqlite::params![PLAN_STATUS_SUPERSEDED, base_id, PLAN_STATUS_DRAFT],
            ))?;
            Ok(updated)
        })
    }

    /// Returns the latest draft plan for a base, if any.
    pub fn find_latest_draft_plan(&self, base_id: &str) -> DataResult<Option<ChangePlanHeader>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT id, created_at, base_id, status, tables_planned, operations_total
                 FROM change_plans
                 WHERE base_id = ?1 AND status = ?2
                 ORDER BY id DESC
                 LIMIT 1",
            ))?;
            let mut rows = sqlite_result(stmt.query_map(
                rusqlite::params![base_id, PLAN_STATUS_DRAFT],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)? as usize,
                        row.get::<_, i64>(5)? as usize,
                    ))
                },
            ))?;

            let Some((id, created_at, base_id, status, tables_planned, operations_total)) =
                rows.next().map(|row| sqlite_result(row)).transpose()?
            else {
                return Ok(None);
            };

            let status_counts = count_operations_by_status_conn(conn, id)?;
            Ok(Some(ChangePlanHeader {
                id,
                created_at,
                base_id,
                status,
                tables_planned,
                operations_total,
                status_counts,
            }))
        })
    }

    /// Returns the most recent change plan for a base, regardless of status.
    pub fn find_latest_plan(&self, base_id: &str) -> DataResult<Option<ChangePlanHeader>> {
        self.db.with_connection(|conn| {
            let mut stmt = sqlite_result(conn.prepare(
                "SELECT id, created_at, base_id, status, tables_planned, operations_total
                 FROM change_plans
                 WHERE base_id = ?1
                 ORDER BY id DESC
                 LIMIT 1",
            ))?;
            let mut rows = sqlite_result(stmt.query_map([base_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)? as usize,
                    row.get::<_, i64>(5)? as usize,
                ))
            }))?;

            let Some((id, created_at, base_id, status, tables_planned, operations_total)) =
                rows.next().map(|row| sqlite_result(row)).transpose()?
            else {
                return Ok(None);
            };

            let status_counts = count_operations_by_status_conn(conn, id)?;
            Ok(Some(ChangePlanHeader {
                id,
                created_at,
                base_id,
                status,
                tables_planned,
                operations_total,
                status_counts,
            }))
        })
    }

    /// Loads a plan header by id.
    pub fn load_plan_header(&self, plan_id: i64) -> DataResult<Option<ChangePlanHeader>> {
        self.db.with_connection(|conn| load_plan_header_conn(conn, plan_id))
    }

    /// Loads a full plan with operations and field changes.
    pub fn load_plan(&self, plan_id: i64) -> DataResult<Option<ChangePlanDetail>> {
        self.db.with_connection(|conn| {
            let Some(plan) = load_plan_header_conn(conn, plan_id)? else {
                return Ok(None);
            };

            let operations = load_operations_for_plan(conn, plan_id, None)?;
            Ok(Some(ChangePlanDetail { plan, operations }))
        })
    }

    /// Finds one operation by id within a plan.
    pub fn find_operation(
        &self,
        plan_id: i64,
        operation_id: i64,
    ) -> DataResult<Option<ChangePlanOperationView>> {
        self.db.with_connection(|conn| {
            let operations = load_operations_for_plan(conn, plan_id, None)?;
            Ok(operations
                .into_iter()
                .find(|operation| operation.operation_id == operation_id))
        })
    }

    /// Finds one operation by table name and primary key within a plan.
    pub fn find_operation_by_key(
        &self,
        plan_id: i64,
        table_name: &str,
        record_key: &str,
    ) -> DataResult<Option<ChangePlanOperationView>> {
        self.db.with_connection(|conn| {
            let operations = load_operations_for_plan(conn, plan_id, None)?;
            Ok(operations.into_iter().find(|operation| {
                operation.table_name == table_name && operation.record_key == record_key
            }))
        })
    }

    /// Updates one operation's review status.
    pub fn set_operation_status(&self, operation_id: i64, status: &str) -> DataResult<()> {
        self.db.with_connection(|conn| {
            let current: String = sqlite_result(conn.query_row(
                "SELECT status FROM change_plan_operations WHERE id = ?1",
                [operation_id],
                |row| row.get(0),
            ))?;

            if current == status {
                return Ok(());
            }

            if current != OPERATION_STATUS_PENDING {
                return Err(DataError::query(format!(
                    "operation {operation_id} is `{current}` and cannot be set to `{status}` — run `sync dry-run` to create a new plan"
                )));
            }

            if status != OPERATION_STATUS_APPROVED && status != OPERATION_STATUS_DENIED {
                return Err(DataError::query(format!(
                    "invalid operation status `{status}`"
                )));
            }

            sqlite_result(conn.execute(
                "UPDATE change_plan_operations SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, operation_id],
            ))?;
            Ok(())
        })
    }

    /// Updates all pending operations in a plan to the given review status.
    pub fn set_pending_status_for_plan(&self, plan_id: i64, status: &str) -> DataResult<usize> {
        if status != OPERATION_STATUS_APPROVED && status != OPERATION_STATUS_DENIED {
            return Err(DataError::query(format!(
                "invalid operation status `{status}`"
            )));
        }

        self.db.with_connection(|conn| {
            let updated = sqlite_result(conn.execute(
                "UPDATE change_plan_operations SET status = ?1
                 WHERE plan_id = ?2 AND status = ?3",
                rusqlite::params![status, plan_id, OPERATION_STATUS_PENDING],
            ))?;
            Ok(updated)
        })
    }

    /// Counts operations in a plan grouped by review status.
    pub fn count_operations_by_status(&self, plan_id: i64) -> DataResult<ChangePlanStatusCounts> {
        self.db.with_connection(|conn| count_operations_by_status_conn(conn, plan_id))
    }

    /// Loads approved operations for a plan, ordered by id.
    pub fn load_approved_operations(
        &self,
        plan_id: i64,
    ) -> DataResult<Vec<ChangePlanOperationView>> {
        self.db.with_connection(|conn| {
            load_operations_for_plan(conn, plan_id, Some(OPERATION_STATUS_APPROVED))
        })
    }

    /// Marks an approved operation as applied.
    pub fn mark_operation_applied(&self, operation_id: i64) -> DataResult<()> {
        self.mark_operation_apply_status(operation_id, OPERATION_STATUS_APPLIED)
    }

    /// Marks an approved operation as failed.
    pub fn mark_operation_failed(&self, operation_id: i64) -> DataResult<()> {
        self.mark_operation_apply_status(operation_id, OPERATION_STATUS_FAILED)
    }

    /// Updates a plan's status.
    pub fn set_plan_status(&self, plan_id: i64, status: &str) -> DataResult<()> {
        self.db.with_connection(|conn| {
            sqlite_result(conn.execute(
                "UPDATE change_plans SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, plan_id],
            ))?;
            Ok(())
        })
    }

    /// Inserts a change plan and its operations in one transaction.
    pub fn insert_plan(
        &self,
        base_id: &str,
        tables_planned: usize,
        operations: &[ChangePlanOperation],
    ) -> DataResult<i64> {
        let transaction = self.db.begin()?;
        let operations_total = operations.len();

        let plan_id = self.db.with_connection(|conn| {
            sqlite_result(conn.execute(
                "INSERT INTO change_plans (created_at, base_id, status, tables_planned, operations_total)
                 VALUES (datetime('now'), ?1, 'draft', ?2, ?3)",
                rusqlite::params![base_id, tables_planned as i64, operations_total as i64],
            ))?;
            Ok(conn.last_insert_rowid())
        })?;

        for operation in operations {
            let operation_id = self.db.with_connection(|conn| {
                sqlite_result(conn.execute(
                    "INSERT INTO change_plan_operations (
                       plan_id, table_name, operation, record_key, airtable_record_id, status
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
                    rusqlite::params![
                        plan_id,
                        operation.table_name,
                        operation.operation,
                        operation.record_key,
                        operation.airtable_record_id,
                    ],
                ))?;
                Ok(conn.last_insert_rowid())
            })?;

            for field_change in &operation.field_changes {
                self.db.with_connection(|conn| {
                    sqlite_result(conn.execute(
                        "INSERT INTO change_plan_field_changes (
                           operation_id, field_name, old_value, new_value
                         ) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![
                            operation_id,
                            field_change.field_name,
                            field_change.old_value,
                            field_change.new_value,
                        ],
                    ))?;
                    Ok(())
                })?;
            }
        }

        transaction.commit()?;
        Ok(plan_id)
    }

    fn mark_operation_apply_status(&self, operation_id: i64, status: &str) -> DataResult<()> {
        if status != OPERATION_STATUS_APPLIED && status != OPERATION_STATUS_FAILED {
            return Err(DataError::query(format!(
                "invalid apply status `{status}`"
            )));
        }

        self.db.with_connection(|conn| {
            let current: String = sqlite_result(conn.query_row(
                "SELECT status FROM change_plan_operations WHERE id = ?1",
                [operation_id],
                |row| row.get(0),
            ))?;

            if current == status {
                return Ok(());
            }

            if current != OPERATION_STATUS_APPROVED {
                return Err(DataError::query(format!(
                    "operation {operation_id} is `{current}` and cannot be set to `{status}`"
                )));
            }

            sqlite_result(conn.execute(
                "UPDATE change_plan_operations SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, operation_id],
            ))?;
            Ok(())
        })
    }
}

fn load_plan_header_conn(
    conn: &Connection,
    plan_id: i64,
) -> DataResult<Option<ChangePlanHeader>> {
    let mut stmt = sqlite_result(conn.prepare(
        "SELECT id, created_at, base_id, status, tables_planned, operations_total
         FROM change_plans WHERE id = ?1",
    ))?;
    let mut rows = sqlite_result(stmt.query_map([plan_id], |row| {
        Ok(ChangePlanHeader {
            id: row.get(0)?,
            created_at: row.get(1)?,
            base_id: row.get(2)?,
            status: row.get(3)?,
            tables_planned: row.get::<_, i64>(4)? as usize,
            operations_total: row.get::<_, i64>(5)? as usize,
            status_counts: ChangePlanStatusCounts::default(),
        })
    }))?;

    let Some(mut plan) = rows
        .next()
        .map(|row| sqlite_result(row))
        .transpose()?
    else {
        return Ok(None);
    };

    plan.status_counts = count_operations_by_status_conn(conn, plan_id)?;
    Ok(Some(plan))
}

fn count_operations_by_status_conn(
    conn: &Connection,
    plan_id: i64,
) -> DataResult<ChangePlanStatusCounts> {
    let mut counts = ChangePlanStatusCounts::default();
    let mut stmt = sqlite_result(conn.prepare(
        "SELECT status, COUNT(*) FROM change_plan_operations
         WHERE plan_id = ?1 GROUP BY status",
    ))?;
    let rows = sqlite_result(stmt.query_map([plan_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
    }))?;

    for row in rows {
        let (status, count) = sqlite_result(row)?;
        match status.as_str() {
            OPERATION_STATUS_PENDING => counts.pending = count,
            OPERATION_STATUS_APPROVED => counts.approved = count,
            OPERATION_STATUS_DENIED => counts.denied = count,
            OPERATION_STATUS_APPLIED => counts.applied = count,
            OPERATION_STATUS_FAILED => counts.failed = count,
            _ => {}
        }
    }

    Ok(counts)
}

fn load_operations_for_plan(
    conn: &Connection,
    plan_id: i64,
    status_filter: Option<&str>,
) -> DataResult<Vec<ChangePlanOperationView>> {
    let mut operation_rows = Vec::new();

    if let Some(status) = status_filter {
        let mut stmt = sqlite_result(conn.prepare(
            "SELECT id, table_name, operation, record_key, airtable_record_id, status
             FROM change_plan_operations
             WHERE plan_id = ?1 AND status = ?2
             ORDER BY id ASC",
        ))?;
        let rows = sqlite_result(stmt.query_map(rusqlite::params![plan_id, status], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        }))?;
        for row in rows {
            operation_rows.push(sqlite_result(row)?);
        }
    } else {
        let mut stmt = sqlite_result(conn.prepare(
            "SELECT id, table_name, operation, record_key, airtable_record_id, status
             FROM change_plan_operations
             WHERE plan_id = ?1
             ORDER BY id ASC",
        ))?;
        let rows = sqlite_result(stmt.query_map([plan_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
        ))
    }))?;
        for row in rows {
            operation_rows.push(sqlite_result(row)?);
        }
    }

    if operation_rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut field_changes_by_operation: HashMap<i64, Vec<ChangePlanFieldChange>> = HashMap::new();
    let mut field_stmt = sqlite_result(conn.prepare(
        "SELECT operation_id, field_name, old_value, new_value
         FROM change_plan_field_changes
         WHERE operation_id IN (
           SELECT id FROM change_plan_operations WHERE plan_id = ?1
         )
         ORDER BY id ASC",
    ))?;
    let field_rows = sqlite_result(field_stmt.query_map([plan_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            ChangePlanFieldChange {
                field_name: row.get(1)?,
                old_value: row.get(2)?,
                new_value: row.get(3)?,
            },
        ))
    }))?;

    for row in field_rows {
        let (operation_id, field_change) = sqlite_result(row)?;
        field_changes_by_operation
            .entry(operation_id)
            .or_default()
            .push(field_change);
    }

    Ok(operation_rows
        .into_iter()
        .map(
            |(operation_id, table_name, operation, record_key, airtable_record_id, status)| {
                ChangePlanOperationView {
                    operation_id,
                    table_name,
                    operation,
                    record_key,
                    airtable_record_id,
                    status,
                    field_changes: field_changes_by_operation
                        .remove(&operation_id)
                        .unwrap_or_default(),
                }
            },
        )
        .collect())
}

/// Ensures the change plan tables exist when persistence is required.
pub fn ensure_change_plan_schema(database_path: &Path) -> NestResult<()> {
    let conn = Connection::open(database_path).map_err(|error| {
        NestError::data(format!(
            "failed to open database {}: {error}",
            database_path.display()
        ))
    })?;
    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'change_plans'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| NestError::data(format!("failed to inspect database schema: {error}")))?;
    if exists == 0 {
        return Err(NestError::data(format!(
            "database is missing change_plans (migration {CHANGE_PLANS_MIGRATION_ID} not applied)"
        ))
        .with_help("Run `db migrate` to apply pending migrations."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{apply_pending_migrations, open_database};
    use tempfile::tempdir;

    const SCHEMA_SQL: &str = "CREATE TABLE notes (id INTEGER PRIMARY KEY, title TEXT NOT NULL);";

    fn sample_operation(record_key: &str) -> ChangePlanOperation {
        ChangePlanOperation {
            table_name: "assets".to_string(),
            operation: "update".to_string(),
            record_key: record_key.to_string(),
            airtable_record_id: Some(format!("rec{record_key}")),
            field_changes: vec![ChangePlanFieldChange {
                field_name: "Name".to_string(),
                old_value: "Old".to_string(),
                new_value: "New".to_string(),
            }],
        }
    }

    fn setup_store() -> (tempfile::TempDir, ChangePlanStore) {
        let dir = tempdir().unwrap();
        let schema_path = dir.path().join("schema.sql");
        let db_path = dir.path().join("app.db");
        std::fs::write(&schema_path, SCHEMA_SQL).unwrap();
        apply_pending_migrations(&db_path, &schema_path, true).unwrap();
        let db = open_database(&db_path).unwrap();
        (dir, ChangePlanStore::new(db))
    }

    #[test]
    fn insert_plan_persists_operations_and_field_changes() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();

        let plan = store.load_plan(plan_id).unwrap().unwrap();
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].field_changes.len(), 1);
        assert_eq!(plan.plan.status_counts.pending, 1);
    }

    #[test]
    fn set_operation_status_updates_pending_operations() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();
        let operation_id = store.load_plan(plan_id).unwrap().unwrap().operations[0].operation_id;

        store
            .set_operation_status(operation_id, OPERATION_STATUS_APPROVED)
            .unwrap();
        let counts = store.count_operations_by_status(plan_id).unwrap();
        assert_eq!(counts.approved, 1);
        assert_eq!(counts.pending, 0);
    }

    #[test]
    fn set_pending_status_for_plan_updates_all_pending() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan(
                "appTEST",
                1,
                &[sample_operation("1"), sample_operation("2")],
            )
            .unwrap();

        let updated = store
            .set_pending_status_for_plan(plan_id, OPERATION_STATUS_DENIED)
            .unwrap();
        assert_eq!(updated, 2);
        let counts = store.count_operations_by_status(plan_id).unwrap();
        assert_eq!(counts.denied, 2);
    }

    #[test]
    fn supersede_draft_plans_leaves_only_newest_draft() {
        let (_dir, store) = setup_store();
        store.supersede_draft_plans("appTEST").unwrap();
        let first = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();
        store.supersede_draft_plans("appTEST").unwrap();
        let second = store
            .insert_plan("appTEST", 1, &[sample_operation("2")])
            .unwrap();

        let first_plan = store.load_plan_header(first).unwrap().unwrap();
        let second_plan = store.load_plan_header(second).unwrap().unwrap();
        assert_eq!(first_plan.status, PLAN_STATUS_SUPERSEDED);
        assert_eq!(second_plan.status, PLAN_STATUS_DRAFT);
        assert!(store.find_latest_draft_plan("appTEST").unwrap().is_some());
    }

    #[test]
    fn set_operation_status_rejects_transition_from_denied() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();
        let operation_id = store.load_plan(plan_id).unwrap().unwrap().operations[0].operation_id;

        store
            .set_operation_status(operation_id, OPERATION_STATUS_DENIED)
            .unwrap();
        let error = store
            .set_operation_status(operation_id, OPERATION_STATUS_APPROVED)
            .unwrap_err();
        assert!(error.to_string().contains("cannot be set"));
    }

    #[test]
    fn load_approved_operations_returns_only_approved() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan(
                "appTEST",
                1,
                &[sample_operation("1"), sample_operation("2")],
            )
            .unwrap();
        let operations = store.load_plan(plan_id).unwrap().unwrap().operations;
        store
            .set_operation_status(operations[0].operation_id, OPERATION_STATUS_APPROVED)
            .unwrap();
        store
            .set_operation_status(operations[1].operation_id, OPERATION_STATUS_DENIED)
            .unwrap();

        let approved = store.load_approved_operations(plan_id).unwrap();
        assert_eq!(approved.len(), 1);
        assert_eq!(approved[0].record_key, "1");
    }

    #[test]
    fn mark_operation_applied_transitions_from_approved() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();
        let operation_id = store.load_plan(plan_id).unwrap().unwrap().operations[0].operation_id;
        store
            .set_operation_status(operation_id, OPERATION_STATUS_APPROVED)
            .unwrap();

        store.mark_operation_applied(operation_id).unwrap();
        let counts = store.count_operations_by_status(plan_id).unwrap();
        assert_eq!(counts.applied, 1);
        assert_eq!(counts.approved, 0);
    }

    #[test]
    fn mark_operation_failed_rejects_non_approved() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();
        let operation_id = store.load_plan(plan_id).unwrap().unwrap().operations[0].operation_id;

        let error = store.mark_operation_failed(operation_id).unwrap_err();
        assert!(error.to_string().contains("cannot be set"));
    }

    #[test]
    fn set_plan_status_updates_plan_row() {
        let (_dir, store) = setup_store();
        let plan_id = store
            .insert_plan("appTEST", 1, &[sample_operation("1")])
            .unwrap();

        store
            .set_plan_status(plan_id, PLAN_STATUS_APPLIED)
            .unwrap();
        let plan = store.load_plan_header(plan_id).unwrap().unwrap();
        assert_eq!(plan.status, PLAN_STATUS_APPLIED);
    }
}
