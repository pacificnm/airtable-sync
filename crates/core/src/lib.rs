//! Shared Airtable Sync application logic.
//!
//! Product code for [airtable-sync](https://github.com/pacificnm/airtable-sync).
//!
//! **Execution model:** all behavior is implemented as CLI commands in this crate.
//! The CLI binary and the future GUI host both dispatch here — the GUI does not
//! duplicate business logic; it runs the same commands (see `docs/architecture.md`).

#![deny(missing_docs)]

pub mod commands;
pub mod config;

mod app;

pub use app::{cli_app, cli_help_text, group_help_text};
pub use commands::{CommandGroupSpec, SubcommandSpec, COMMAND_GROUPS};
