//! Shared Airtable Sync application logic.
//!
//! Product code for [airtable-sync](https://github.com/pacificnm/airtable-sync).
//! CLI and future GUI hosts compose [`cli_app`] with domain commands from this crate.

#![deny(missing_docs)]

pub mod commands;

mod app;

pub use app::cli_app;
