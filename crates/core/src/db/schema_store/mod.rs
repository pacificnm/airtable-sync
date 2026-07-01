//! SQLite persistence for Airtable table and field schema cache.

mod access;
mod store;

pub use access::ensure_schema_cache;
pub use store::{
    AirtableFieldRow, AirtableTableRow, AirtableTableSummary, FieldMappingRow,
    FieldMappingUpdate, SchemaPullStats, SchemaStore,
};
