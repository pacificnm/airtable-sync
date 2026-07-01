//! Limited CSV row reading for preview.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_file_csv::normalize_header;

/// Result of reading a limited CSV preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvPreviewReadResult {
    /// Normalized header names in file order.
    pub headers: Vec<String>,
    /// Data rows as cell values aligned to `headers`.
    pub rows: Vec<Vec<String>>,
    /// Number of data rows returned (at most `limit`).
    pub rows_returned: usize,
    /// Whether the file contains more data rows than `limit`.
    pub truncated: bool,
    /// Non-fatal warnings (e.g. duplicate headers).
    pub warnings: Vec<String>,
}

/// Reads up to `limit` data rows from a CSV file without loading the entire file.
pub fn read_csv_preview(
    files: &FileService,
    path: &Path,
    limit: usize,
) -> NestResult<CsvPreviewReadResult> {
    let content = files.read_text(path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(content.as_bytes()));

    let raw_headers = reader.headers().map_err(|error| {
        NestError::validation(format!(
            "failed to read CSV headers from {}: {error}",
            path.display()
        ))
    })?;

    let mut headers = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_normalized = HashSet::new();

    for raw in raw_headers.iter() {
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

        headers.push(normalized);
    }

    let mut rows = Vec::new();
    let mut truncated = false;

    for (index, result) in reader.records().enumerate() {
        if index == limit {
            truncated = true;
            break;
        }

        let record = result.map_err(|error| {
            NestError::validation(format!(
                "failed to parse CSV row {} in {}: {error}",
                index + 1,
                path.display()
            ))
        })?;

        let mut values = Vec::with_capacity(headers.len());
        for column_index in 0..headers.len() {
            values.push(record.get(column_index).unwrap_or("").to_string());
        }
        rows.push(values);
    }

    let rows_returned = rows.len();
    Ok(CsvPreviewReadResult {
        headers,
        rows,
        rows_returned,
        truncated,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_file::FileService;

    #[test]
    fn read_csv_preview_trims_and_normalizes_headers() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.csv");
        std::fs::write(&path, " ID , Name \n1,Alice\n").unwrap();

        let result = read_csv_preview(&files, &path, 5).unwrap();
        assert_eq!(result.headers, vec!["id", "name"]);
        assert_eq!(result.rows, vec![vec!["1".to_string(), "Alice".to_string()]]);
        assert!(!result.truncated);
    }

    #[test]
    fn read_csv_preview_respects_limit_and_sets_truncated() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("many.csv");
        std::fs::write(&path, "id\n1\n2\n3\n").unwrap();

        let result = read_csv_preview(&files, &path, 2).unwrap();
        assert_eq!(result.rows_returned, 2);
        assert_eq!(result.rows, vec![vec!["1"], vec!["2"]]);
        assert!(result.truncated);
    }

    #[test]
    fn read_csv_preview_warns_on_duplicate_headers() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.csv");
        std::fs::write(&path, "id,ID\n1,2\n").unwrap();

        let result = read_csv_preview(&files, &path, 5).unwrap();
        assert_eq!(result.headers, vec!["id"]);
        assert_eq!(result.rows, vec![vec!["1".to_string()]]);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("duplicate header"));
    }
}
