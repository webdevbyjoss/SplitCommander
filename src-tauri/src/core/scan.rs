use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use jwalk::WalkDir;

use crate::core::ignore::IgnoreRules;
use crate::core::model::{EntryKind, EntryMeta};

#[derive(Debug)]
pub struct ScanResult {
    /// Lowercased relative path → metadata
    pub entries: HashMap<String, EntryMeta>,
    /// Lowercased relative path → original-case relative path
    pub originals: HashMap<String, String>,
    pub count: usize,
    pub errors: Vec<ScanError>,
}

#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

/// Scans a directory root in parallel using jwalk.
/// Returns a map of relative paths to metadata.
///
/// - `cancel_flag`: set to true to abort scan
/// - `progress_callback`: called every 1000 entries with current count
pub fn scan_directory(
    root: &Path,
    ignore_rules: &IgnoreRules,
    cancel_flag: &AtomicBool,
    progress_callback: &dyn Fn(usize),
) -> Result<ScanResult, String> {
    let mut entries = HashMap::new();
    let mut originals = HashMap::new();
    let mut errors = Vec::new();
    let mut count: usize = 0;

    let walker = WalkDir::new(root)
        .skip_hidden(false)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(num_cpus()));

    for entry_result in walker {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("Scan cancelled".to_string());
        }

        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                let rel_path = match path.strip_prefix(root) {
                    Ok(r) => r.to_string_lossy().to_string(),
                    Err(_) => continue,
                };

                if rel_path.is_empty() {
                    continue;
                }

                if ignore_rules.is_ignored(&rel_path) {
                    continue;
                }

                let file_type = entry.file_type();
                let kind = if file_type.is_dir() {
                    EntryKind::Dir
                } else if file_type.is_symlink() {
                    EntryKind::Symlink
                } else {
                    EntryKind::File
                };

                let (size, modified) = match entry.metadata() {
                    Ok(meta) => {
                        let size = meta.len();
                        let modified = meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_millis() as u64);
                        (size, modified)
                    }
                    Err(e) => {
                        errors.push(ScanError {
                            path: rel_path.clone(),
                            message: e.to_string(),
                        });
                        (0, None)
                    }
                };

                let symlink_target = if kind == EntryKind::Symlink {
                    std::fs::read_link(&path)
                        .ok()
                        .map(|t| t.to_string_lossy().to_string())
                } else {
                    None
                };

                let meta = EntryMeta {
                    kind,
                    size,
                    modified,
                    symlink_target,
                };

                let key = rel_path.to_lowercase();
                originals.insert(key.clone(), rel_path);
                entries.insert(key, meta);

                count += 1;
                if count % 1000 == 0 {
                    progress_callback(count);
                }
            }
            Err(e) => {
                errors.push(ScanError {
                    path: "unknown".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    progress_callback(count);

    Ok(ScanResult {
        entries,
        originals,
        count,
        errors,
    })
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn no_cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn test_scan_empty_dir() {
        let dir = std::env::temp_dir().join("sc_scan_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let rules = IgnoreRules::new(&[]);
        let cancel = no_cancel();
        let result = scan_directory(&dir, &rules, &cancel, &|_| {}).unwrap();

        assert_eq!(result.count, 0);
        assert!(result.entries.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_with_files() {
        let dir = std::env::temp_dir().join("sc_scan_files");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("file1.txt"), "hello").unwrap();
        fs::write(dir.join("subdir/file2.txt"), "world").unwrap();

        let rules = IgnoreRules::new(&[]);
        let cancel = no_cancel();
        let result = scan_directory(&dir, &rules, &cancel, &|_| {}).unwrap();

        assert!(result.entries.contains_key("file1.txt"));
        assert!(result.entries.contains_key("subdir/file2.txt"));
        assert!(result.entries.contains_key("subdir"));
        assert_eq!(result.entries["file1.txt"].kind, EntryKind::File);
        assert_eq!(result.entries["file1.txt"].size, 5);
        assert_eq!(result.entries["subdir"].kind, EntryKind::Dir);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_ignore_rules() {
        let dir = std::env::temp_dir().join("sc_scan_ignore");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".DS_Store"), "").unwrap();
        fs::write(dir.join("keep.txt"), "data").unwrap();

        let rules = IgnoreRules::new(&[]);
        let cancel = no_cancel();
        let result = scan_directory(&dir, &rules, &cancel, &|_| {}).unwrap();

        assert!(!result.entries.contains_key(".ds_store"));
        assert!(result.entries.contains_key("keep.txt"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_cancellation() {
        let dir = std::env::temp_dir().join("sc_scan_cancel");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for i in 0..10 {
            fs::write(dir.join(format!("file{}.txt", i)), "data").unwrap();
        }

        let rules = IgnoreRules::new(&[]);
        // Pre-set cancel flag
        let cancel = AtomicBool::new(true);
        let result = scan_directory(&dir, &rules, &cancel, &|_| {});

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cancelled"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_case_insensitive_keys() {
        let dir = std::env::temp_dir().join("sc_scan_case");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("README.md"), "hello").unwrap();

        let rules = IgnoreRules::new(&[]);
        let cancel = no_cancel();
        let result = scan_directory(&dir, &rules, &cancel, &|_| {}).unwrap();

        // Key should be lowercased
        assert!(result.entries.contains_key("readme.md"));
        // Original should preserve case
        assert_eq!(result.originals["readme.md"], "README.md");

        let _ = fs::remove_dir_all(&dir);
    }
}
