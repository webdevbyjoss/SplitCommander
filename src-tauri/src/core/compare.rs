use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::model::*;
use crate::core::scan::ScanResult;

pub struct CompareResult {
    pub diffs: Vec<DiffItem>,
    pub summary: CompareSummary,
}

/// Compares two scan results, producing a diff list and summary.
pub fn compare(
    left: &ScanResult,
    right: &ScanResult,
    mode: CompareMode,
    cancel_flag: &AtomicBool,
) -> Result<CompareResult, String> {
    let mut diffs = Vec::new();
    let mut summary = CompareSummary::default();

    summary.total_left = left.entries.len();
    summary.total_right = right.entries.len();

    let all_keys: HashSet<&String> = left.entries.keys().chain(right.entries.keys()).collect();

    for key in &all_keys {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("Compare cancelled".to_string());
        }

        let left_entry = left.entries.get(*key);
        let right_entry = right.entries.get(*key);

        // Prefer left original path, fall back to right
        let original_path = left
            .originals
            .get(*key)
            .or_else(|| right.originals.get(*key))
            .cloned()
            .unwrap_or_else(|| key.to_string());

        let diff = match (left_entry, right_entry) {
            (Some(l), None) => {
                summary.only_left += 1;
                DiffItem {
                    rel_path: original_path,
                    diff_kind: DiffKind::OnlyLeft,
                    left: Some(l.clone()),
                    right: None,
                    error_message: None,
                }
            }
            (None, Some(r)) => {
                summary.only_right += 1;
                DiffItem {
                    rel_path: original_path,
                    diff_kind: DiffKind::OnlyRight,
                    left: None,
                    right: Some(r.clone()),
                    error_message: None,
                }
            }
            (Some(l), Some(r)) => classify_pair(&original_path, l, r, mode, &mut summary),
            (None, None) => unreachable!(),
        };

        diffs.push(diff);
    }

    // Sort diffs by path for consistent output
    diffs.sort_by(|a, b| a.rel_path.to_lowercase().cmp(&b.rel_path.to_lowercase()));

    Ok(CompareResult { diffs, summary })
}

fn classify_pair(
    rel_path: &str,
    left: &EntryMeta,
    right: &EntryMeta,
    mode: CompareMode,
    summary: &mut CompareSummary,
) -> DiffItem {
    // Type mismatch (applies in all modes)
    if left.kind != right.kind {
        summary.type_mismatch += 1;
        return DiffItem {
            rel_path: rel_path.to_string(),
            diff_kind: DiffKind::TypeMismatch,
            left: Some(left.clone()),
            right: Some(right.clone()),
            error_message: None,
        };
    }

    match mode {
        CompareMode::Structure => {
            // Structure mode: same kind = same
            summary.same += 1;
            DiffItem {
                rel_path: rel_path.to_string(),
                diff_kind: DiffKind::Same,
                left: Some(left.clone()),
                right: Some(right.clone()),
                error_message: None,
            }
        }
        CompareMode::Smart => {
            // Directories: always Same in smart mode (size/mtime not meaningful)
            if left.kind == EntryKind::Dir {
                summary.same += 1;
                return DiffItem {
                    rel_path: rel_path.to_string(),
                    diff_kind: DiffKind::Same,
                    left: Some(left.clone()),
                    right: Some(right.clone()),
                    error_message: None,
                };
            }

            let size_match = left.size == right.size;
            let symlink_match = left.symlink_target == right.symlink_target;

            if size_match && symlink_match {
                summary.same += 1;
                DiffItem {
                    rel_path: rel_path.to_string(),
                    diff_kind: DiffKind::Same,
                    left: Some(left.clone()),
                    right: Some(right.clone()),
                    error_message: None,
                }
            } else {
                summary.meta_diff += 1;
                DiffItem {
                    rel_path: rel_path.to_string(),
                    diff_kind: DiffKind::MetaDiff,
                    left: Some(left.clone()),
                    right: Some(right.clone()),
                    error_message: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_scan(entries: Vec<(&str, EntryMeta)>) -> ScanResult {
        let mut map = HashMap::new();
        let mut originals = HashMap::new();
        for (path, meta) in &entries {
            let key = path.to_lowercase();
            originals.insert(key.clone(), path.to_string());
            map.insert(key, meta.clone());
        }
        ScanResult {
            count: map.len(),
            entries: map,
            originals,
            errors: vec![],
        }
    }

    fn file_meta(size: u64, mtime: u64) -> EntryMeta {
        EntryMeta {
            kind: EntryKind::File,
            size,
            modified: Some(mtime),
            symlink_target: None,
        }
    }

    fn dir_meta() -> EntryMeta {
        EntryMeta {
            kind: EntryKind::Dir,
            size: 0,
            modified: Some(1000),
            symlink_target: None,
        }
    }

    fn no_cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn test_identical_files() {
        let left = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let right = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.same, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::Same);
    }

    #[test]
    fn test_only_left() {
        let left = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let right = make_scan(vec![]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.only_left, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::OnlyLeft);
    }

    #[test]
    fn test_only_right() {
        let left = make_scan(vec![]);
        let right = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.only_right, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::OnlyRight);
    }

    #[test]
    fn test_type_mismatch() {
        let left = make_scan(vec![("item", file_meta(100, 1000))]);
        let right = make_scan(vec![("item", dir_meta())]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.type_mismatch, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::TypeMismatch);
    }

    #[test]
    fn test_meta_diff_size() {
        let left = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let right = make_scan(vec![("file.txt", file_meta(200, 1000))]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.meta_diff, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::MetaDiff);
    }

    #[test]
    fn test_same_size_different_mtime_is_same() {
        let left = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let right = make_scan(vec![("file.txt", file_meta(100, 2000))]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.same, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::Same);
    }

    #[test]
    fn test_structure_mode_ignores_metadata() {
        let left = make_scan(vec![("file.txt", file_meta(100, 1000))]);
        let right = make_scan(vec![("file.txt", file_meta(200, 2000))]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Structure, &cancel).unwrap();
        assert_eq!(result.summary.same, 1);
        assert_eq!(result.diffs[0].diff_kind, DiffKind::Same);
    }

    #[test]
    fn test_dirs_always_same_in_smart() {
        let left = make_scan(vec![("mydir", dir_meta())]);
        let right_dir = EntryMeta {
            kind: EntryKind::Dir,
            size: 4096,
            modified: Some(9999),
            symlink_target: None,
        };
        let right = make_scan(vec![("mydir", right_dir)]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.same, 1);
    }

    #[test]
    fn test_summary_counts() {
        let left = make_scan(vec![
            ("same.txt", file_meta(100, 1000)),
            ("left_only.txt", file_meta(50, 500)),
            ("changed.txt", file_meta(100, 1000)),
        ]);
        let right = make_scan(vec![
            ("same.txt", file_meta(100, 1000)),
            ("right_only.txt", file_meta(75, 750)),
            ("changed.txt", file_meta(200, 2000)),
        ]);
        let cancel = no_cancel();

        let result = compare(&left, &right, CompareMode::Smart, &cancel).unwrap();
        assert_eq!(result.summary.same, 1);
        assert_eq!(result.summary.only_left, 1);
        assert_eq!(result.summary.only_right, 1);
        assert_eq!(result.summary.meta_diff, 1);
        assert_eq!(result.summary.total_left, 3);
        assert_eq!(result.summary.total_right, 3);
    }
}
