//! CSV vs Airtable record comparison.

use std::collections::{HashMap, HashSet};

use nest_airtable::AirtableRecord;
use serde::Serialize;
use serde_json::Value;

use super::csv_index::CsvIndex;
use super::value::{normalize_airtable_value, normalize_csv_value};

/// One mapped field participating in compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareField {
    /// Airtable field name.
    pub field_name: String,
    /// Normalized CSV column name.
    pub csv_column: String,
}

/// One field value mismatch for a matched primary key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareFieldDiff {
    /// Airtable field name.
    pub field_name: String,
    /// CSV column name.
    pub csv_column: String,
    /// Value from CSV.
    pub csv_value: String,
    /// Value from Airtable.
    pub airtable_value: String,
}

/// One record with at least one differing field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareDifferingRecord {
    /// Primary key value.
    pub key: String,
    /// Airtable record id (`rec…`), when known.
    pub airtable_record_id: Option<String>,
    /// Field-level differences.
    pub differences: Vec<CompareFieldDiff>,
}

/// Compare summary counts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct CompareSummary {
    /// Indexed CSV rows with a non-empty primary key.
    pub csv_rows: usize,
    /// Airtable records with a non-empty primary key.
    pub airtable_rows: usize,
    /// Records present in both with identical compared fields.
    pub matched: usize,
    /// Records present in both with at least one difference.
    pub differing: usize,
    /// Primary keys present only in CSV.
    pub csv_only: usize,
    /// Primary keys present only in Airtable.
    pub airtable_only: usize,
}

/// Full compare diff result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompareDiffResult {
    /// Compare summary counts.
    pub summary: CompareSummary,
    /// Records with field differences.
    pub differing_records: Vec<CompareDifferingRecord>,
    /// Primary keys present only in CSV.
    pub csv_only_keys: Vec<String>,
    /// Primary keys present only in Airtable.
    pub airtable_only_keys: Vec<String>,
}

/// Compares indexed CSV rows to Airtable records by primary key.
pub fn compare_records(
    csv_index: &CsvIndex,
    airtable_records: &[AirtableRecord],
    primary_key_field: &str,
    fields: &[CompareField],
) -> CompareDiffResult {
    let airtable_by_key = index_airtable_records(airtable_records, primary_key_field);
    let csv_keys: HashSet<_> = csv_index.rows.keys().cloned().collect();
    let airtable_keys: HashSet<_> = airtable_by_key.keys().cloned().collect();

    let mut differing_records = Vec::new();
    let mut matched = 0usize;

    for key in csv_keys.intersection(&airtable_keys) {
        let csv_row = &csv_index.rows[key];
        let (record_id, airtable_values) = &airtable_by_key[key];
        let differences = diff_row(csv_row, airtable_values, fields);
        if differences.is_empty() {
            matched += 1;
        } else {
            differing_records.push(CompareDifferingRecord {
                key: key.clone(),
                airtable_record_id: Some(record_id.clone()),
                differences,
            });
        }
    }

    let mut csv_only_keys: Vec<_> = csv_keys
        .difference(&airtable_keys)
        .cloned()
        .collect();
    csv_only_keys.sort();

    let mut airtable_only_keys: Vec<_> = airtable_keys
        .difference(&csv_keys)
        .cloned()
        .collect();
    airtable_only_keys.sort();

    differing_records.sort_by(|left, right| left.key.cmp(&right.key));

    let summary = CompareSummary {
        csv_rows: csv_index.rows.len(),
        airtable_rows: airtable_by_key.len(),
        matched,
        differing: differing_records.len(),
        csv_only: csv_only_keys.len(),
        airtable_only: airtable_only_keys.len(),
    };

    CompareDiffResult {
        summary,
        differing_records,
        csv_only_keys,
        airtable_only_keys,
    }
}

fn index_airtable_records(
    records: &[AirtableRecord],
    primary_key_field: &str,
) -> HashMap<String, (String, HashMap<String, Value>)> {
    let mut indexed = HashMap::new();

    for record in records {
        let Some(value) = record.fields.0.get(primary_key_field) else {
            continue;
        };
        let key = normalize_airtable_value(value);
        if key.is_empty() {
            continue;
        }
        indexed.insert(
            key,
            (record.id.clone(), record.fields.0.clone()),
        );
    }

    indexed
}

fn diff_row(
    csv_row: &super::csv_index::CsvIndexedRow,
    airtable_values: &HashMap<String, Value>,
    fields: &[CompareField],
) -> Vec<CompareFieldDiff> {
    let mut differences = Vec::new();

    for field in fields {
        let csv_value = normalize_csv_value(
            csv_row
                .values
                .get(&field.csv_column)
                .map(String::as_str)
                .unwrap_or(""),
        );
        let airtable_value = airtable_values
            .get(&field.field_name)
            .map(normalize_airtable_value)
            .unwrap_or_default();

        if csv_value != airtable_value {
            differences.push(CompareFieldDiff {
                field_name: field.field_name.clone(),
                csv_column: field.csv_column.clone(),
                csv_value,
                airtable_value,
            });
        }
    }

    differences
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_airtable::AirtableFields;
    use serde_json::json;

    fn sample_csv_index() -> CsvIndex {
        let mut rows = HashMap::new();
        rows.insert(
            "1".to_string(),
            super::super::csv_index::CsvIndexedRow {
                key: "1".to_string(),
                values: HashMap::from([
                    ("id".to_string(), "1".to_string()),
                    ("name".to_string(), "Alice".to_string()),
                ]),
            },
        );
        rows.insert(
            "2".to_string(),
            super::super::csv_index::CsvIndexedRow {
                key: "2".to_string(),
                values: HashMap::from([
                    ("id".to_string(), "2".to_string()),
                    ("name".to_string(), "Bob".to_string()),
                ]),
            },
        );
        CsvIndex {
            rows,
            row_count: 2,
            skipped_empty_keys: 0,
            warnings: Vec::new(),
        }
    }

    fn sample_fields() -> Vec<CompareField> {
        vec![CompareField {
            field_name: "Name".to_string(),
            csv_column: "name".to_string(),
        }]
    }

    #[test]
    fn compare_records_detects_match_and_difference() {
        let csv_index = sample_csv_index();
        let records = vec![
            AirtableRecord {
                id: "recA".to_string(),
                created_time: None,
                fields: AirtableFields(HashMap::from([
                    ("ID".to_string(), json!("1")),
                    ("Name".to_string(), json!("Alice")),
                ])),
            },
            AirtableRecord {
                id: "recB".to_string(),
                created_time: None,
                fields: AirtableFields(HashMap::from([
                    ("ID".to_string(), json!("2")),
                    ("Name".to_string(), json!("Robert")),
                ])),
            },
            AirtableRecord {
                id: "recC".to_string(),
                created_time: None,
                fields: AirtableFields(HashMap::from([
                    ("ID".to_string(), json!("3")),
                    ("Name".to_string(), json!("Carol")),
                ])),
            },
        ];

        let result = compare_records(&csv_index, &records, "ID", &sample_fields());
        assert_eq!(result.summary.matched, 1);
        assert_eq!(result.summary.differing, 1);
        assert_eq!(result.summary.csv_only, 0);
        assert_eq!(result.summary.airtable_only, 1);
        assert_eq!(result.differing_records[0].key, "2");
        assert_eq!(result.airtable_only_keys, vec!["3".to_string()]);
    }
}
