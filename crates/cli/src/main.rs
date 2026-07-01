//! Airtable Sync command-line entry point.

fn main() {
    airtable_sync_core::cli_app_with_version(env!("CARGO_PKG_VERSION")).run();
}
