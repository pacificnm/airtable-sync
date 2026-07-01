//! CSV row indexing by primary key column.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;

use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_file_csv::normalize_header;

/// One indexed CSV row keyed by a primary key value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvIndexedRow {
    /// Primary key value after normalization.
    pub key: String,
    /// Cell values keyed by normalized header name.
    pub values: HashMap<String, String>,
}

/// Result of indexing a CSV file by primary key column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvIndex {
    /// Rows keyed by normalized primary key value.
    pub rows: HashMap<String, CsvIndexedRow>,
    /// Number of data rows scanned.
    pub row_count: usize,
    /// Rows skipped because the primary key was empty.
    pub skipped_empty_keys: usize,
    /// Non-fatal warnings (duplicate headers, etc.).
    pub warnings: Vec<String>,
}

/// Reads a CSV file and indexes rows by the given normalized primary key column.
pub fn index_csv_by_key(
    files: &FileService,
    path: &Path,
    primary_key_column: &str,
) -> NestResult<CsvIndex> {
    let content = files.read_text(path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(content.as_bytes()));

    let (header_indexes, column_count, pk_index, warnings) = {
        let raw_headers = reader.headers().map_err(|error| {
            NestError::validation(format!(
                "failed to read CSV headers from {}: {error}",
                path.display()
            ))
        })?;

        let mut header_indexes: HashMap<String, usize> = HashMap::new();
        let mut warnings = Vec::new();
        let mut seen_normalized = HashSet::new();

        for (index, raw) in raw_headers.iter().enumerate() {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                warnings.push(format!(
                    "skipped empty header column in {}",
                    path.display()
                ));
                continue;
            }

            let normalized = normalize_header(trimmed, true, true);
            if !seen_normalized.insert(normalized.clone()) {
                warnings.push(format!(
                    "duplicate header `{trimmed}` (normalized `{normalized}`) in {}",
                    path.display()
                ));
                continue;
            }

            header_indexes.insert(normalized, index);
        }

        let column_count = raw_headers.len();
        let Some(&pk_index) = header_indexes.get(primary_key_column) else {
            return Err(NestError::validation(format!(
                "primary key CSV column `{primary_key_column}` not found in {}",
                path.display()
            )));
        };

        (header_indexes, column_count, pk_index, warnings)
    };

    let mut rows = HashMap::new();
    let mut row_count = 0usize;
    let mut skipped_empty_keys = 0usize;

    for (index, result) in reader.records().enumerate() {
        let record = result.map_err(|error| {
            NestError::validation(format!(
                "failed to parse CSV row {} in {}: {error}",
                index + 1,
                path.display()
            ))
        })?;

        row_count += 1;
        if record.len() != column_count {
            return Err(NestError::validation(format!(
                "row {} has {} columns, expected {}",
                index + 1,
                record.len(),
                column_count
            )));
        }

        let key = record.get(pk_index).unwrap_or("").trim().to_string();
        if key.is_empty() {
            skipped_empty_keys += 1;
            continue;
        }

        if rows.contains_key(&key) {
            return Err(NestError::validation(format!(
                "duplicate primary key `{key}` in {} at row {}",
                path.display(),
                index + 1
            )));
        }

        let mut values = HashMap::new();
        for (column, column_index) in &header_indexes {
            let cell = record.get(*column_index).unwrap_or("");
            values.insert(column.clone(), cell.to_string());
        }

        rows.insert(
            key.clone(),
            CsvIndexedRow {
                key,
                values,
            },
        );
    }

    Ok(CsvIndex {
        rows,
        row_count,
        skipped_empty_keys,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_file::FileService;

    #[test]
    fn index_csv_by_key_indexes_rows() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rows.csv");
        std::fs::write(&path, "id,name\n1,Alice\n2,Bob\n").unwrap();

        let index = index_csv_by_key(&files, &path, "id").unwrap();
        assert_eq!(index.row_count, 2);
        assert_eq!(index.rows.len(), 2);
        assert_eq!(index.rows["1"].values["name"], "Alice");
    }

    #[test]
    fn index_csv_by_key_rejects_duplicate_keys() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.csv");
        std::fs::write(&path, "id\n1\n1\n").unwrap();

        let error = index_csv_by_key(&files, &path, "id").unwrap_err();
        assert_eq!(error.kind(), nest_error::NestErrorKind::Validation);
    }
}
