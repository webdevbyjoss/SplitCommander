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
