//! Airtable API command handlers.

mod bridge;
mod list_fields;
mod list_tables;
mod pull_schema;
mod runtime;
mod test;

pub use list_fields::list_fields;
pub use list_tables::list_tables;
pub use pull_schema::{pull_schema, PullSchemaResult};
pub use test::{test, AirtableTestResult};

pub(crate) use bridge::to_airtable_config;
pub(crate) use runtime::block_on_async;
