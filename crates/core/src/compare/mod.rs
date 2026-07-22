//! Compare CSV data to Airtable records.

mod all;
mod csv_index;
pub(crate) mod diff;
pub(crate) mod engine;
pub(crate) mod table;
mod value;

pub use all::compare_all;
pub use table::compare_table;

pub(crate) use diff::{merge_compare_summaries, CompareSummary};
pub(crate) use engine::compare_single_table;
pub(crate) use table::CompareTableResult;
