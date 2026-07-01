//! Full-file CSV structural validation.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use nest_error::{NestError, NestResult};
use nest_file::FileService;
use nest_file_csv::normalize_header;

/// Result of validating one CSV file's structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvValidateReadResult {
    /// Normalized header names kept in file order (duplicates omitted).
    pub headers: Vec<String>,
    /// Number of physical columns in the header row.
    pub column_count: usize,
    /// Number of data rows scanned.
    pub row_count: usize,
    /// Non-fatal issues (empty or duplicate headers).
    pub warnings: Vec<String>,
    /// Fatal structural issues (ragged rows).
    pub errors: Vec<String>,
}

impl CsvValidateReadResult {
    /// Returns whether the file has no fatal structural errors.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validates CSV structure for one file (headers + row widths).
pub fn validate_csv_file(
    files: &FileService,
    path: &Path,
) -> NestResult<CsvValidateReadResult> {
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

    let column_count = raw_headers.len();
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

    let mut errors = Vec::new();
    let mut row_count = 0usize;

    for (index, result) in reader.records().enumerate() {
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                errors.push(format!(
                    "failed to parse row {} in {}: {error}",
                    index + 1,
                    path.display()
                ));
                continue;
            }
        };

        row_count += 1;
        if record.len() != column_count {
            errors.push(format!(
                "row {} has {} columns, expected {}",
                index + 1,
                record.len(),
                column_count
            ));
        }
    }

    Ok(CsvValidateReadResult {
        headers,
        column_count,
        row_count,
        warnings,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nest_file::FileService;

    #[test]
    fn validate_csv_file_accepts_well_formed_file() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("good.csv");
        std::fs::write(&path, "id,name\n1,Alice\n2,Bob\n").unwrap();

        let result = validate_csv_file(&files, &path).unwrap();
        assert!(result.is_valid());
        assert_eq!(result.column_count, 2);
        assert_eq!(result.row_count, 2);
        assert_eq!(result.headers, vec!["id", "name"]);
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn validate_csv_file_warns_on_duplicate_headers() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.csv");
        std::fs::write(&path, "id,ID\n1,2\n").unwrap();

        let result = validate_csv_file(&files, &path).unwrap();
        assert!(result.is_valid());
        assert_eq!(result.headers, vec!["id"]);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("duplicate header"));
    }

    #[test]
    fn validate_csv_file_errors_on_ragged_rows() {
        let files = FileService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ragged.csv");
        std::fs::write(&path, "id,name\n1\n2,Bob,extra\n").unwrap();

        let result = validate_csv_file(&files, &path).unwrap();
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors[0].contains("row 1"));
        assert!(result.errors[1].contains("row 2"));
    }
}
