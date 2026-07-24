//! `config set` command: format-preserving edits to `config.toml`.
//!
//! Only an explicit allow-list of keys can be set (see [`FLAT_KEYS`] and
//! [`TABLE_FIELDS`]) — this is a targeted settings editor, not a generic TOML
//! poke. Table entries (`airtable.tables.<name>.<field>`) must already exist
//! in the file; this command edits fields, it does not add or remove tables.

use std::fs;

use nest_cli::CliGlobals;
use nest_config::ConfigService;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;
use toml_edit::{value, DocumentMut, Item, TableLike};

/// JSON response for `config set --json`.
#[derive(Debug, Serialize)]
pub struct ConfigSetResult {
    /// The dotted key that was set (e.g. `airtable.base_id`).
    pub key: String,
    /// The value it was set to, as passed on the command line.
    pub value: String,
}

enum FieldKind {
    /// Must be non-empty; an empty value is rejected.
    RequiredString,
    /// An empty value removes the key from the TOML (falls back to its default).
    OptionalString,
    /// Accepts exactly `"true"`/`"false"` (case-insensitive).
    Bool,
}

/// Flat (non-table) editable keys: `(["section", "field"], kind)`.
const FLAT_KEYS: &[(&[&str], FieldKind)] = &[
    (&["airtable", "api_url"], FieldKind::OptionalString),
    (&["airtable", "token_env"], FieldKind::OptionalString),
    (&["airtable", "base_id"], FieldKind::RequiredString),
    (&["csv", "location_data_file"], FieldKind::RequiredString),
    (&["csv", "space_data_file"], FieldKind::RequiredString),
    (&["database", "provider"], FieldKind::RequiredString),
    (&["database", "database_path"], FieldKind::RequiredString),
    (&["database", "schema"], FieldKind::RequiredString),
    (&["logging", "level"], FieldKind::RequiredString),
    (&["logging", "directory"], FieldKind::RequiredString),
];

/// Editable fields on an `[airtable.tables.<name>]` entry: `(field, kind)`.
const TABLE_FIELDS: &[(&str, FieldKind)] = &[
    ("table_id", FieldKind::RequiredString),
    ("sync", FieldKind::Bool),
    ("primary_key_field", FieldKind::OptionalString),
];

/// Sets one allow-listed `config.toml` key in place, preserving the rest of
/// the file's formatting and comments.
pub fn set(ctx: &AppContext, key: &str, raw_value: &str) -> NestResult<()> {
    let config = ctx.service::<ConfigService>()?;
    let path = config
        .path()
        .ok_or_else(|| {
            NestError::validation(
                "config set requires a config.toml on disk (no --config path resolved)",
            )
        })?
        .to_path_buf();

    let raw = fs::read_to_string(&path)
        .map_err(|error| NestError::io(format!("failed to read {}: {error}", path.display())))?;
    let mut doc = raw
        .parse::<DocumentMut>()
        .map_err(|error| NestError::data(format!("failed to parse {}: {error}", path.display())))?;

    apply_set(&mut doc, key, raw_value)?;

    fs::write(&path, doc.to_string())
        .map_err(|error| NestError::io(format!("failed to write {}: {error}", path.display())))?;

    let globals = ctx.service::<CliGlobals>().ok();
    let json = globals.as_ref().is_some_and(|globals| globals.json);
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let result = ConfigSetResult {
        key: key.to_string(),
        value: raw_value.to_string(),
    };

    if json {
        let payload = serde_json::to_string_pretty(&result).map_err(|error| {
            NestError::data(format!("failed to serialize config set result: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!("Set {} = {}", result.key, result.value);
    }

    Ok(())
}

fn apply_set(doc: &mut DocumentMut, key: &str, raw_value: &str) -> NestResult<()> {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 4 && parts[0] == "airtable" && parts[1] == "tables" {
        return set_table_field(doc, parts[2], parts[3], raw_value, key);
    }

    let (path, kind) = FLAT_KEYS
        .iter()
        .find(|(path, _)| path.len() == parts.len() && path.iter().eq(parts.iter()))
        .ok_or_else(|| unknown_key_error(key))?;

    let section = path[0];
    let field = path[1];

    let table = doc
        .entry(section)
        .or_insert_with(|| Item::Table(Default::default()))
        .as_table_like_mut()
        .ok_or_else(|| NestError::data(format!("[{section}] is not a table in config.toml")))?;

    set_field(table, field, raw_value, kind, key)
}

fn set_table_field(
    doc: &mut DocumentMut,
    table_name: &str,
    field: &str,
    raw_value: &str,
    full_key: &str,
) -> NestResult<()> {
    let kind = TABLE_FIELDS
        .iter()
        .find(|(name, _)| *name == field)
        .map(|(_, kind)| kind)
        .ok_or_else(|| unknown_key_error(full_key))?;

    let airtable = doc
        .entry("airtable")
        .or_insert_with(|| Item::Table(Default::default()))
        .as_table_like_mut()
        .ok_or_else(|| NestError::data("[airtable] is not a table in config.toml".to_string()))?;

    let tables = airtable
        .entry("tables")
        .or_insert_with(|| Item::Table(Default::default()))
        .as_table_like_mut()
        .ok_or_else(|| {
            NestError::data("[airtable.tables] is not a table in config.toml".to_string())
        })?;

    let entry = tables.get_mut(table_name).ok_or_else(|| {
        NestError::validation(format!(
            "table \"{table_name}\" is not configured — config set only edits existing [airtable.tables.<name>] entries"
        ))
    })?;

    let entry_table = entry.as_table_like_mut().ok_or_else(|| {
        NestError::data(format!(
            "[airtable.tables.{table_name}] is not a table in config.toml"
        ))
    })?;

    set_field(entry_table, field, raw_value, kind, full_key)
}

fn set_field(
    table: &mut dyn TableLike,
    field: &str,
    raw_value: &str,
    kind: &FieldKind,
    full_key: &str,
) -> NestResult<()> {
    match kind {
        FieldKind::RequiredString => {
            if raw_value.trim().is_empty() {
                return Err(NestError::validation(format!("{full_key} cannot be empty")));
            }
            set_value_preserving_decor(table, field, value(raw_value));
        }
        FieldKind::OptionalString => {
            if raw_value.is_empty() {
                table.remove(field);
            } else {
                set_value_preserving_decor(table, field, value(raw_value));
            }
        }
        FieldKind::Bool => {
            let parsed = match raw_value.to_ascii_lowercase().as_str() {
                "true" => true,
                "false" => false,
                other => {
                    return Err(NestError::validation(format!(
                        "{full_key} must be \"true\" or \"false\", got \"{other}\""
                    )))
                }
            };
            set_value_preserving_decor(table, field, value(parsed));
        }
    }
    Ok(())
}

/// Sets `field` to `new_item`, keeping the previous value's decor (inline
/// comment, leading whitespace) if it had one — `toml_edit::value(...)`
/// otherwise produces a bare item with no decor, dropping any trailing
/// comment on the line being edited even though the rest of the file is
/// untouched.
fn set_value_preserving_decor(table: &mut dyn TableLike, field: &str, new_item: Item) {
    let decor = table
        .get(field)
        .and_then(Item::as_value)
        .map(|value| value.decor().clone());

    table.insert(field, new_item);

    if let Some(decor) = decor {
        if let Some(new_value) = table.get_mut(field).and_then(Item::as_value_mut) {
            *new_value.decor_mut() = decor;
        }
    }
}

fn unknown_key_error(key: &str) -> NestError {
    NestError::validation(format!("\"{key}\" is not an editable config key"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> DocumentMut {
        toml.parse::<DocumentMut>().unwrap()
    }

    #[test]
    fn sets_a_required_flat_string() {
        let mut doc = parse("[airtable]\nbase_id = \"appOLD\"\n");
        apply_set(&mut doc, "airtable.base_id", "appNEW").unwrap();
        assert_eq!(doc["airtable"]["base_id"].as_str(), Some("appNEW"));
    }

    #[test]
    fn rejects_empty_required_string() {
        let mut doc = parse("[airtable]\nbase_id = \"appOLD\"\n");
        let error = apply_set(&mut doc, "airtable.base_id", "").unwrap_err();
        assert!(error.message().contains("cannot be empty"));
        assert_eq!(doc["airtable"]["base_id"].as_str(), Some("appOLD"));
    }

    #[test]
    fn empty_optional_string_unsets_the_key() {
        let mut doc = parse("[airtable]\nbase_id = \"appX\"\ntoken_env = \"MY_TOKEN\"\n");
        apply_set(&mut doc, "airtable.token_env", "").unwrap();
        assert!(doc["airtable"].get("token_env").is_none());
    }

    #[test]
    fn creates_missing_section_for_flat_key() {
        let mut doc = parse("[airtable]\nbase_id = \"appX\"\n");
        apply_set(&mut doc, "logging.level", "debug").unwrap();
        assert_eq!(doc["logging"]["level"].as_str(), Some("debug"));
    }

    #[test]
    fn sets_table_id_on_existing_table() {
        let mut doc = parse(
            "[airtable.tables.assets]\ntable_id = \"tblOLD\"\nsync = false\n",
        );
        apply_set(&mut doc, "airtable.tables.assets.table_id", "tblNEW").unwrap();
        assert_eq!(
            doc["airtable"]["tables"]["assets"]["table_id"].as_str(),
            Some("tblNEW")
        );
    }

    #[test]
    fn toggles_sync_bool() {
        let mut doc = parse(
            "[airtable.tables.assets]\ntable_id = \"tblX\"\nsync = false\n",
        );
        apply_set(&mut doc, "airtable.tables.assets.sync", "true").unwrap();
        assert_eq!(
            doc["airtable"]["tables"]["assets"]["sync"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn rejects_invalid_bool_value() {
        let mut doc = parse(
            "[airtable.tables.assets]\ntable_id = \"tblX\"\nsync = false\n",
        );
        let error = apply_set(&mut doc, "airtable.tables.assets.sync", "yes").unwrap_err();
        assert!(error.message().contains("must be \"true\" or \"false\""));
    }

    #[test]
    fn unsets_primary_key_field_on_empty() {
        let mut doc = parse(
            "[airtable.tables.assets]\ntable_id = \"tblX\"\nsync = true\nprimary_key_field = \"Name\"\n",
        );
        apply_set(&mut doc, "airtable.tables.assets.primary_key_field", "").unwrap();
        assert!(doc["airtable"]["tables"]["assets"]
            .get("primary_key_field")
            .is_none());
    }

    #[test]
    fn rejects_editing_a_table_that_does_not_exist() {
        let mut doc = parse("[airtable.tables.assets]\ntable_id = \"tblX\"\n");
        let error = apply_set(&mut doc, "airtable.tables.ghost.table_id", "tblY").unwrap_err();
        assert!(error.message().contains("not configured"));
    }

    #[test]
    fn rejects_unknown_key() {
        let mut doc = parse("[airtable]\nbase_id = \"appX\"\n");
        let error = apply_set(&mut doc, "airtable.token", "pat-secret").unwrap_err();
        assert!(error.message().contains("not an editable config key"));
    }

    #[test]
    fn rejects_unknown_table_field() {
        let mut doc = parse("[airtable.tables.assets]\ntable_id = \"tblX\"\n");
        let error = apply_set(&mut doc, "airtable.tables.assets.enabled", "true").unwrap_err();
        assert!(error.message().contains("not an editable config key"));
    }

    #[test]
    fn preserves_unrelated_content_and_comments() {
        let mut doc = parse(
            "# top comment\n[airtable]\nbase_id = \"appOLD\" # inline comment\ntoken_env = \"X\"\n\n[csv]\nlocation_data_file = \"a.csv\"\n",
        );
        apply_set(&mut doc, "airtable.base_id", "appNEW").unwrap();
        let rendered = doc.to_string();
        assert!(rendered.contains("# top comment"));
        assert!(rendered.contains("# inline comment"));
        assert!(rendered.contains("token_env = \"X\""));
        assert!(rendered.contains("location_data_file = \"a.csv\""));
        assert!(rendered.contains("appNEW"));
    }
}
