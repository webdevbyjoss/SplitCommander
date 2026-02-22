use serde::Serialize;

use crate::core::model::{CompareSummary, CompareStatus};

pub const EVENT_SCAN_PROGRESS: &str = "scan-progress";
pub const EVENT_COMPARE_DONE: &str = "compare-done";
pub const EVENT_COMPARE_ERROR: &str = "compare-error";
pub const EVENT_DIR_STATUS_RESOLVED: &str = "dir-status-resolved";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressPayload {
    pub side: String,
    pub entries_scanned: usize,
    pub phase: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareDonePayload {
    pub summary: CompareSummary,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareErrorPayload {
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirStatusResolvedPayload {
    pub name: String,
    pub status: CompareStatus,
    pub left_path: String,
    pub right_path: String,
    pub total_size: u64,
}
