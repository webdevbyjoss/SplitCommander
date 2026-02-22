use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMeta {
    pub kind: EntryKind,
    pub size: u64,
    /// Epoch milliseconds for JS interop
    pub modified: Option<u64>,
    pub symlink_target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffKind {
    OnlyLeft,
    OnlyRight,
    TypeMismatch,
    Same,
    MetaDiff,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffItem {
    pub rel_path: String,
    pub diff_kind: DiffKind,
    pub left: Option<EntryMeta>,
    pub right: Option<EntryMeta>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompareMode {
    Structure,
    Smart,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSummary {
    pub total_left: usize,
    pub total_right: usize,
    pub only_left: usize,
    pub only_right: usize,
    pub type_mismatch: usize,
    pub same: usize,
    pub meta_diff: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompareStatus {
    Same,
    Modified,
    OnlyLeft,
    OnlyRight,
    TypeMismatch,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareEntry {
    pub name: String,
    pub kind: EntryKind,
    pub status: CompareStatus,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
    pub left_modified: Option<u64>,
    pub right_modified: Option<u64>,
    pub dir_info: Option<DirResolveInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirResolveInfo {
    pub total_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_meta_serialization_roundtrip() {
        let meta = EntryMeta {
            kind: EntryKind::File,
            size: 1024,
            modified: Some(1700000000000),
            symlink_target: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: EntryMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, EntryKind::File);
        assert_eq!(deserialized.size, 1024);
        assert_eq!(deserialized.modified, Some(1700000000000));
    }

    #[test]
    fn test_diff_item_serialization() {
        let item = DiffItem {
            rel_path: "src/main.rs".to_string(),
            diff_kind: DiffKind::MetaDiff,
            left: Some(EntryMeta {
                kind: EntryKind::File,
                size: 100,
                modified: Some(1000),
                symlink_target: None,
            }),
            right: Some(EntryMeta {
                kind: EntryKind::File,
                size: 200,
                modified: Some(2000),
                symlink_target: None,
            }),
            error_message: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("metaDiff"));
        assert!(json.contains("relPath"));
    }

    #[test]
    fn test_compare_summary_default() {
        let summary = CompareSummary::default();
        assert_eq!(summary.total_left, 0);
        assert_eq!(summary.same, 0);
    }
}
