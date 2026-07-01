//! Normalizes CSV and Airtable values for comparison.

use serde_json::Value;

/// Normalizes a CSV cell for comparison.
pub fn normalize_csv_value(value: &str) -> String {
    value.trim().to_string()
}

/// Normalizes an Airtable field value for comparison.
pub fn normalize_airtable_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.trim().to_string(),
        Value::Array(items) => items
            .iter()
            .map(normalize_airtable_value)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(_) => value
            .as_str()
            .map(|text| text.trim().to_string())
            .unwrap_or_else(|| value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_csv_value_trims_whitespace() {
        assert_eq!(normalize_csv_value("  hello  "), "hello");
    }

    #[test]
    fn normalize_airtable_value_handles_scalars() {
        assert_eq!(normalize_airtable_value(&json!(null)), "");
        assert_eq!(normalize_airtable_value(&json!(true)), "true");
        assert_eq!(normalize_airtable_value(&json!(42)), "42");
        assert_eq!(normalize_airtable_value(&json!("  text ")), "text");
    }

    #[test]
    fn normalize_airtable_value_joins_arrays() {
        assert_eq!(
            normalize_airtable_value(&json!(["a", "b"])),
            "a, b"
        );
    }
}
