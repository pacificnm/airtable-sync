//! Build sync change plans from compare results.

use serde::Serialize;

use crate::compare::CompareTableResult;
use crate::db::{ChangePlanFieldChange, ChangePlanOperation};

/// Per-table sync plan summary.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct SyncTablePlanSummary {
    /// Records with identical compared fields.
    pub matched: usize,
    /// Planned update operations.
    pub updates: usize,
    /// Differing records skipped because updates are disabled.
    pub skipped_no_permission: usize,
}

/// Planned sync operations for one table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncTablePlan {
    /// Non-fatal planning warnings.
    pub warnings: Vec<String>,
    /// Planned operations for this table.
    pub operations: Vec<ChangePlanOperation>,
    /// Planning summary counts.
    pub summary: SyncTablePlanSummary,
}

/// Builds an update-only sync plan from a compare result.
pub fn build_table_plan(compare: &CompareTableResult, allow_update: bool) -> SyncTablePlan {
    let compare_summary = &compare.compare.summary;
    let mut warnings = Vec::new();
    let mut operations = Vec::new();
    let mut skipped_no_permission = 0usize;

    if !allow_update && !compare.compare.differing_records.is_empty() {
        skipped_no_permission = compare.compare.differing_records.len();
        warnings.push(format!(
            "skipped {} differing record(s) in `{}` because allow_update is false",
            skipped_no_permission, compare.table.name
        ));
    } else {
        for record in &compare.compare.differing_records {
            operations.push(ChangePlanOperation {
                table_name: compare.table.name.clone(),
                operation: "update".to_string(),
                record_key: record.key.clone(),
                airtable_record_id: record.airtable_record_id.clone(),
                field_changes: record
                    .differences
                    .iter()
                    .map(|diff| ChangePlanFieldChange {
                        field_name: diff.field_name.clone(),
                        old_value: diff.airtable_value.clone(),
                        new_value: diff.csv_value.clone(),
                    })
                    .collect(),
            });
        }
    }

    SyncTablePlan {
        warnings,
        summary: SyncTablePlanSummary {
            matched: compare_summary.matched,
            updates: operations.len(),
            skipped_no_permission,
        },
        operations,
    }
}

/// Merges per-table plan summaries into rollup counts.
pub fn merge_plan_summaries(
    left: &SyncTablePlanSummary,
    right: &SyncTablePlanSummary,
) -> SyncTablePlanSummary {
    SyncTablePlanSummary {
        matched: left.matched + right.matched,
        updates: left.updates + right.updates,
        skipped_no_permission: left.skipped_no_permission + right.skipped_no_permission,
    }
}

/// Counts total field changes across planned operations.
pub fn count_field_changes(operations: &[ChangePlanOperation]) -> usize {
    operations
        .iter()
        .map(|operation| operation.field_changes.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::diff::{
        CompareDiffResult, CompareDifferingRecord, CompareFieldDiff, CompareSummary,
    };
    use crate::compare::table::CompareTableView;
    use crate::compare::CompareTableResult;
    use std::path::PathBuf;

    fn sample_compare_result() -> CompareTableResult {
        CompareTableResult {
            database_path: PathBuf::from("/tmp/app.db"),
            base_id: "appTEST".to_string(),
            table: CompareTableView {
                name: "assets".to_string(),
                table_id: "tblTEST".to_string(),
                enabled: true,
            },
            primary_key_field: "ID".to_string(),
            primary_key_csv_column: "id".to_string(),
            csv_file: "location.csv".to_string(),
            csv_path: PathBuf::from("/tmp/location.csv"),
            compared_fields: vec!["Name".to_string()],
            compare: CompareDiffResult {
                summary: CompareSummary {
                    csv_rows: 2,
                    airtable_rows: 2,
                    matched: 1,
                    differing: 1,
                    csv_only: 0,
                    airtable_only: 0,
                },
                differing_records: vec![CompareDifferingRecord {
                    key: "2".to_string(),
                    airtable_record_id: Some("recB".to_string()),
                    differences: vec![CompareFieldDiff {
                        field_name: "Name".to_string(),
                        csv_column: "name".to_string(),
                        csv_value: "Bob".to_string(),
                        airtable_value: "Robert".to_string(),
                    }],
                }],
                csv_only_keys: Vec::new(),
                airtable_only_keys: Vec::new(),
            },
        }
    }

    #[test]
    fn build_table_plan_converts_differing_records_to_updates() {
        let plan = build_table_plan(&sample_compare_result(), true);

        assert_eq!(plan.summary.updates, 1);
        assert_eq!(plan.summary.matched, 1);
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].record_key, "2");
        assert_eq!(plan.operations[0].field_changes[0].new_value, "Bob");
    }

    #[test]
    fn build_table_plan_skips_updates_when_not_allowed() {
        let plan = build_table_plan(&sample_compare_result(), false);

        assert!(plan.operations.is_empty());
        assert_eq!(plan.summary.skipped_no_permission, 1);
        assert!(plan.warnings.iter().any(|warning| warning.contains("allow_update")));
    }
}
