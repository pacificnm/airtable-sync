//! Compare CSV data to Airtable records.

mod csv_index;
mod diff;
mod table;
mod value;

pub use table::compare_table;
