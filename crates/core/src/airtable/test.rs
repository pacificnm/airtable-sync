//! `airtable test` connectivity probe.

use nest_airtable::{AirtableClient, AirtableListParams, AirtableModule};
use nest_cli::CliGlobals;
use nest_core::{AppBuilder, AppContext};
use nest_error::{NestError, NestResult};
use nest_http_client::HttpClientModule;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning};

use super::bridge::{test_table_name, to_airtable_config};
use super::runtime::block_on_async;

/// JSON response for successful `airtable test` with `--json`.
#[derive(Debug, Serialize)]
pub struct AirtableTestResult {
    /// Airtable base id probed.
    pub base_id: String,
    /// Logical table name probed.
    pub table: String,
    /// Airtable table id probed.
    pub table_id: String,
    /// Number of records returned in the probe page.
    pub records_returned: usize,
    /// API base URL used for the request.
    pub api_url: String,
}

/// Tests Airtable connectivity by listing one record page from a configured table.
pub fn test(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let config = to_airtable_config(&validated.app)?;
    let table_name = test_table_name(&validated.app)?;
    let table_name = table_name.to_string();
    let table_id = config.table(&table_name)?.table_id.clone();
    let base_id = config.base_id.clone();
    let api_url = config.api_url.clone();
    let client_config = config;
    let probe_table = table_name.clone();

    let page = block_on_async(async move {
        let built = AppBuilder::new()
            .module(HttpClientModule::default())
            .module(AirtableModule::with_config(client_config))
            .build()?;
        let client = built.context.service::<AirtableClient>()?.clone();
        let mut params = AirtableListParams::default();
        params.page_size = Some(1);
        client.list_records_page(&probe_table, &params).await
    })?;

    let result = AirtableTestResult {
        base_id,
        table: table_name,
        table_id,
        records_returned: page.records.len(),
        api_url,
    };

    print_test_success(&result, json, quiet)
}

fn print_test_success(result: &AirtableTestResult, json: bool, quiet: bool) -> NestResult<()> {
    if json {
        let payload = serde_json::to_string_pretty(result).map_err(|error| {
            NestError::data(format!("failed to serialize airtable test result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!(
            "Airtable connection OK: base {} table {} ({}) — {} record(s) in probe page",
            result.base_id, result.table, result.table_id, result.records_returned
        );
    }
    Ok(())
}
