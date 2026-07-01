//! CSV header parsing helpers.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_file_csv::normalize_header;

/// One parsed CSV column header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvHeader {
    /// Trimmed original header text.
    pub name: String,
    /// Lowercase normalized header for mapping.
    pub normalized_name: String,
}

/// Result of reading headers from one CSV file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvHeaderReadResult {
    /// Parsed headers in file order.
    pub headers: Vec<CsvHeader>,
    /// Non-fatal warnings (e.g. duplicate columns).
    pub warnings: Vec<String>,
}

/// Reads and normalizes the header row from a CSV file without loading data rows.
pub fn read_csv_headers(
    files: &FileService,
    path: &Path,
) -> NestResult<CsvHeaderReadResult> {
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

        headers.push(CsvHeader {
            name: trimmed.to_string(),
            normalized_name: normalized,
        });
    }

    Ok(CsvHeaderReadResult { headers, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_file::FileService;

    #[test]
    fn read_csv_headers_trims_and_normalizes() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.csv");
        std::fs::write(&path, " ID , Name \n1,Alice\n").unwrap();

        let result = read_csv_headers(&files, &path).unwrap();
        assert_eq!(result.warnings, Vec::<String>::new());
        assert_eq!(
            result.headers,
            vec![
                CsvHeader {
                    name: "ID".to_string(),
                    normalized_name: "id".to_string(),
                },
                CsvHeader {
                    name: "Name".to_string(),
                    normalized_name: "name".to_string(),
                },
            ]
        );
    }

    #[test]
    fn read_csv_headers_warns_on_duplicate_normalized_columns() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.csv");
        std::fs::write(&path, "id,ID\n1,2\n").unwrap();

        let result = read_csv_headers(&files, &path).unwrap();
        assert_eq!(result.headers.len(), 1);
        assert_eq!(result.headers[0].name, "id");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("duplicate header"));
    }
}
