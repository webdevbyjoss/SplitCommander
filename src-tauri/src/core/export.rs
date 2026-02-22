use serde::Serialize;

use crate::core::model::{CompareMode, CompareSummary, DiffItem};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub version: String,
    pub left_root: String,
    pub right_root: String,
    pub mode: CompareMode,
    pub summary: CompareSummary,
    pub diffs: Vec<DiffItem>,
    pub generated_at: String,
}

pub fn generate_json_report(
    left_root: &str,
    right_root: &str,
    mode: CompareMode,
    summary: CompareSummary,
    diffs: Vec<DiffItem>,
) -> Result<String, String> {
    let report = ExportReport {
        version: "0.1.0".to_string(),
        left_root: left_root.to_string(),
        right_root: right_root.to_string(),
        mode,
        summary,
        diffs,
        generated_at: chrono::Utc::now().to_rfc3339(),
    };
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::*;

    #[test]
    fn test_generate_json_report() {
        let summary = CompareSummary {
            total_left: 1,
            total_right: 1,
            same: 1,
            ..Default::default()
        };
        let diffs = vec![DiffItem {
            rel_path: "test.txt".to_string(),
            diff_kind: DiffKind::Same,
            left: Some(EntryMeta {
                kind: EntryKind::File,
                size: 10,
                modified: Some(1000),
                symlink_target: None,
            }),
            right: Some(EntryMeta {
                kind: EntryKind::File,
                size: 10,
                modified: Some(1000),
                symlink_target: None,
            }),
            error_message: None,
        }];

        let json = generate_json_report("/left", "/right", CompareMode::Smart, summary, diffs);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains("\"version\": \"0.1.0\""));
        assert!(json.contains("\"leftRoot\": \"/left\""));
        assert!(json.contains("generatedAt"));
    }
}
