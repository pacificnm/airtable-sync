//! Field mapping command handlers.

mod auto;
mod disable;
mod enable;
mod list;
mod remove;
mod report;
mod resolve;
mod set;

pub use auto::{auto_map, AutoMapResult};
pub use disable::disable_mapping;
pub use enable::enable_mapping;
pub use list::list_mappings;
pub use report::mapping_report;

pub(crate) use auto::compute_auto_map;
pub(crate) use report::{
    build_mapping_table_reports, MappingReportSummary, MappingReportTableView,
};
pub use remove::remove_mapping;
pub use set::set_mapping;
