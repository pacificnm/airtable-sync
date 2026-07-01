//! Maps product [`AppConfig`] to [`nest_airtable::AirtableConfig`].

use nest_airtable::{looks_like_secret, resolve_airtable_token, AirtableConfig};
use nest_error::{NestError, NestResult};

use crate::config::AppConfig;

/// Builds a resolved [`AirtableConfig`] from the product configuration.
pub fn to_airtable_config(app: &AppConfig) -> NestResult<AirtableConfig> {
    let section = &app.airtable;
    let token = resolve_airtable_token(section.token.as_deref(), section.token_env.as_deref())?;

    let mut builder = AirtableConfig::builder(&section.base_id, token);
    if let Some(url) = &section.api_url {
        builder = builder.api_url(url);
    }
    if let Some(url) = &section.meta_api_url {
        builder = builder.meta_api_url(url);
    }
    if let Some(token_env) = &section.token_env {
        if !looks_like_secret(token_env) {
            builder = builder.token_env(token_env);
        }
    }

    for (name, table) in &section.tables {
        builder = builder.table(name, &table.table_id, table.primary_key_field.clone());
    }

    builder.build()
}

/// Returns the logical table name used for connectivity probes.
///
/// Prefers the first table with `sync = true`, otherwise the first configured table.
pub fn test_table_name(app: &AppConfig) -> NestResult<&str> {
    app.airtable
        .tables
        .iter()
        .find(|(_, table)| table.sync)
        .map(|(name, _)| name.as_str())
        .or_else(|| app.airtable.tables.keys().next().map(String::as_str))
        .ok_or_else(|| {
            NestError::config("no Airtable tables configured")
                .with_module("airtable-sync")
                .with_help("Add at least one [airtable.tables.<name>] section with table_id.")
        })
}
