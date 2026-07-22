//! SQLite database management commands.

mod change_plan_store;
mod common;
mod csv_store;
mod init;
mod migrate;
mod reset;
mod schema;
mod schema_store;

pub use csv_store::{ensure_csv_cache, CsvFieldRow, CsvStore};
pub use init::init;
pub use migrate::migrate;
pub use reset::reset;
pub use schema::{format_schema_human, introspect_database, schema, DbSchemaView};
pub use schema_store::{
    ensure_schema_cache, AirtableFieldRow, AirtableTableRow, AirtableTableSummary,
    FieldMappingRow, FieldMappingUpdate, SchemaPullStats, SchemaStore,
};

pub use change_plan_store::{
    ensure_change_plan_schema, ChangePlanDetail, ChangePlanFieldChange, ChangePlanHeader,
    ChangePlanOperation, ChangePlanOperationView, ChangePlanStatusCounts, ChangePlanStore,
    OPERATION_STATUS_APPROVED, OPERATION_STATUS_DENIED, OPERATION_STATUS_FAILED,
    OPERATION_STATUS_PENDING, OPERATION_STATUS_APPLIED, PLAN_STATUS_APPLIED, PLAN_STATUS_DRAFT,
    PLAN_STATUS_SUPERSEDED,
};
pub use common::{
    apply_pending_migrations, open_database, registered_migrations, DbMigrateResult,
    CHANGE_PLANS_MIGRATION_ID, MIGRATION_ID,
};
pub(crate) use common::absolute_path;
