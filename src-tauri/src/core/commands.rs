use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::compare;
use crate::core::events::*;
use crate::core::export;
use crate::core::fileops;
use crate::core::ignore::IgnoreRules;
use crate::core::model::*;
use crate::core::pty;
use crate::core::scan;

/// Cache key for resolved directory statuses: (left_path, right_path).
pub type DirCacheKey = (String, String);
/// Cache value: (status, total_size).
pub type DirCacheValue = (CompareStatus, u64);

/// Shared application state managed by Tauri.
pub struct AppState {
    pub left_root: Mutex<Option<PathBuf>>,
    pub right_root: Mutex<Option<PathBuf>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub dir_resolve_cancel: Arc<AtomicBool>,
    pub last_result: Mutex<Option<LastCompareResult>>,
    pub dir_resolve_cache: Arc<Mutex<HashMap<DirCacheKey, DirCacheValue>>>,
    pub pty_left: Mutex<Option<pty::PtyState>>,
    pub pty_right: Mutex<Option<pty::PtyState>>,
}

pub struct LastCompareResult {
    pub diffs: Vec<DiffItem>,
    pub summary: CompareSummary,
    pub left_root: String,
    pub right_root: String,
    pub mode: CompareMode,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            left_root: Mutex::new(None),
            right_root: Mutex::new(None),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            dir_resolve_cancel: Arc::new(AtomicBool::new(false)),
            last_result: Mutex::new(None),
            dir_resolve_cache: Arc::new(Mutex::new(HashMap::new())),
            pty_left: Mutex::new(None),
            pty_right: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub async fn set_root(side: String, path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    match side.as_str() {
        "left" => *state.left_root.lock().unwrap() = Some(path_buf),
        "right" => *state.right_root.lock().unwrap() = Some(path_buf),
        _ => return Err(format!("Invalid side: {}", side)),
    }
    Ok(())
}

#[tauri::command]
pub async fn start_compare(
    mode: CompareMode,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let left_root = state
        .left_root
        .lock()
        .unwrap()
        .clone()
        .ok_or("Left root not set")?;
    let right_root = state
        .right_root
        .lock()
        .unwrap()
        .clone()
        .ok_or("Right root not set")?;

    state.cancel_flag.store(false, Ordering::Relaxed);
    let cancel_flag = Arc::clone(&state.cancel_flag);

    let app_handle = app.clone();
    let left_str = left_root.to_string_lossy().to_string();
    let right_str = right_root.to_string_lossy().to_string();

    tokio::task::spawn_blocking(move || {
        let ignore_rules = IgnoreRules::new(&[]);
        let cancel = cancel_flag.as_ref();

        // Scan left
        let app_left = app_handle.clone();
        let left_result = match scan::scan_directory(&left_root, &ignore_rules, cancel, &|count| {
            let _ = app_left.emit(
                EVENT_SCAN_PROGRESS,
                ScanProgressPayload {
                    side: "left".to_string(),
                    entries_scanned: count,
                    phase: "scanning".to_string(),
                },
            );
        }) {
            Ok(r) => r,
            Err(e) => {
                let _ = app_handle.emit(
                    EVENT_COMPARE_ERROR,
                    CompareErrorPayload { message: e },
                );
                return;
            }
        };

        let _ = app_handle.emit(
            EVENT_SCAN_PROGRESS,
            ScanProgressPayload {
                side: "left".to_string(),
                entries_scanned: left_result.count,
                phase: "done".to_string(),
            },
        );

        // Scan right
        let app_right = app_handle.clone();
        let right_result = match scan::scan_directory(&right_root, &ignore_rules, cancel, &|count| {
            let _ = app_right.emit(
                EVENT_SCAN_PROGRESS,
                ScanProgressPayload {
                    side: "right".to_string(),
                    entries_scanned: count,
                    phase: "scanning".to_string(),
                },
            );
        }) {
            Ok(r) => r,
            Err(e) => {
                let _ = app_handle.emit(
                    EVENT_COMPARE_ERROR,
                    CompareErrorPayload { message: e },
                );
                return;
            }
        };

        let _ = app_handle.emit(
            EVENT_SCAN_PROGRESS,
            ScanProgressPayload {
                side: "right".to_string(),
                entries_scanned: right_result.count,
                phase: "done".to_string(),
            },
        );

        // Compare
        match compare::compare(&left_result, &right_result, mode, cancel) {
            Ok(result) => {
                let _ = app_handle.emit(
                    EVENT_COMPARE_DONE,
                    CompareDonePayload {
                        summary: result.summary.clone(),
                    },
                );

                // Store result for later retrieval
                if let Some(app_state) = app_handle.try_state::<AppState>() {
                    *app_state.last_result.lock().unwrap() = Some(LastCompareResult {
                        diffs: result.diffs,
                        summary: result.summary,
                        left_root: left_str,
                        right_root: right_str,
                        mode,
                    });
                }
            }
            Err(e) => {
                let _ = app_handle.emit(
                    EVENT_COMPARE_ERROR,
                    CompareErrorPayload { message: e },
                );
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn cancel_compare(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn get_diffs(state: State<'_, AppState>) -> Result<Vec<DiffItem>, String> {
    let result = state.last_result.lock().unwrap();
    match result.as_ref() {
        Some(r) => Ok(r.diffs.clone()),
        None => Err("No comparison result available".to_string()),
    }
}

#[tauri::command]
pub async fn get_summary(state: State<'_, AppState>) -> Result<CompareSummary, String> {
    let result = state.last_result.lock().unwrap();
    match result.as_ref() {
        Some(r) => Ok(r.summary.clone()),
        None => Err("No comparison result available".to_string()),
    }
}

#[tauri::command]
pub async fn export_report(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let result = state.last_result.lock().unwrap();
    match result.as_ref() {
        Some(r) => {
            let json = export::generate_json_report(
                &r.left_root,
                &r.right_root,
                r.mode,
                r.summary.clone(),
                r.diffs.clone(),
            )?;
            std::fs::write(&path, json).map_err(|e| e.to_string())
        }
        None => Err("No comparison result to export".to_string()),
    }
}

/// A single entry for directory browsing (not comparison).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseEntry {
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<u64>,
}

/// Result of init_browse: home path + initial directory listing in one IPC call.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitBrowseResult {
    pub home: String,
    pub entries: Vec<BrowseEntry>,
}

/// Returns home directory path + its listing in a single IPC call for fast startup.
#[tauri::command]
pub async fn init_browse() -> Result<InitBrowseResult, String> {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let entries = list_directory_impl(&home)?;
    Ok(InitBrowseResult { home, entries })
}

/// Lists the contents of a directory for browsing.
/// Returns entries sorted: directories first, then files, alphabetically.
#[tauri::command]
pub async fn list_directory(path: String) -> Result<Vec<BrowseEntry>, String> {
    list_directory_impl(&path)
}

/// Opens a file with the OS default application.
#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || {
        open::that(&p).map_err(|e| format!("Cannot open {}: {}", p, e))
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

/// Copies a file or directory from source to the destination directory.
#[tauri::command]
pub async fn copy_entry(source_path: String, dest_dir: String) -> Result<(), String> {
    let src = PathBuf::from(&source_path);
    let dst = PathBuf::from(&dest_dir);

    if !src.exists() {
        return Err(format!("Source does not exist: {}", source_path));
    }
    if !dst.is_dir() {
        return Err(format!("Destination is not a directory: {}", dest_dir));
    }

    tokio::task::spawn_blocking(move || fileops::copy_entry(&src, &dst))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
        .map(|_| ())
}

/// Moves a file or directory from source to the destination directory.
#[tauri::command]
pub async fn move_entry(source_path: String, dest_dir: String) -> Result<(), String> {
    let src = PathBuf::from(&source_path);
    let dst = PathBuf::from(&dest_dir);

    if !src.exists() {
        return Err(format!("Source does not exist: {}", source_path));
    }
    if !dst.is_dir() {
        return Err(format!("Destination is not a directory: {}", dest_dir));
    }

    tokio::task::spawn_blocking(move || fileops::move_entry(&src, &dst))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
        .map(|_| ())
}

/// Creates a new directory inside parent_path with the given name.
#[tauri::command]
pub async fn create_directory(parent_path: String, name: String) -> Result<(), String> {
    let parent = PathBuf::from(&parent_path);

    if !parent.is_dir() {
        return Err(format!("Parent is not a directory: {}", parent_path));
    }

    tokio::task::spawn_blocking(move || fileops::create_directory(&parent, &name))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
        .map(|_| ())
}

/// Deletes a file or directory (recursively for directories).
#[tauri::command]
pub async fn delete_entry(target_path: String) -> Result<(), String> {
    let target = PathBuf::from(&target_path);

    if !target.exists() {
        return Err(format!("Does not exist: {}", target_path));
    }

    tokio::task::spawn_blocking(move || fileops::delete_entry(&target))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

/// Persisted pane state saved across app restarts.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub left_path: String,
    pub right_path: String,
    pub left_selected_index: i32,
    pub left_scroll_top: f64,
    pub right_selected_index: i32,
    pub right_scroll_top: f64,
    pub left_show_hidden: bool,
    pub right_show_hidden: bool,
}

fn state_file_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not determine data directory".to_string())?;
    Ok(data_dir.join("com.splitcommander.app").join("state.json"))
}

#[tauri::command]
pub async fn load_app_state() -> Result<Option<PersistedState>, String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let state: PersistedState = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    // Validate that saved paths still exist
    if !PathBuf::from(&state.left_path).is_dir() || !PathBuf::from(&state.right_path).is_dir() {
        return Ok(None);
    }
    Ok(Some(state))
}

#[tauri::command]
pub async fn save_app_state(state: PersistedState) -> Result<(), String> {
    let path = state_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Result of comparing a single directory level between two paths.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareDirectoryResult {
    pub entries: Vec<CompareEntry>,
    pub left_path: String,
    pub right_path: String,
    pub summary: CompareSummary,
}

/// Compares a single directory level from two paths, returning merged entries.
/// Directories present on both sides use cached status if available, otherwise marked Pending.
#[tauri::command]
pub async fn compare_directory(
    left_path: String,
    right_path: String,
    state: State<'_, AppState>,
) -> Result<CompareDirectoryResult, String> {
    let lp = left_path.clone();
    let rp = right_path.clone();
    let cache = Arc::clone(&state.dir_resolve_cache);

    // Run on blocking thread since dir listing does I/O
    let result = tokio::task::spawn_blocking(move || {
        compare_directory_impl(&lp, &rp, &cache)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?;

    let (entries, summary) = result;

    Ok(CompareDirectoryResult {
        entries,
        left_path,
        right_path,
        summary,
    })
}

/// Compare one directory level. Dirs on both sides use cache or are marked Pending.
fn compare_directory_impl(
    left_path: &str,
    right_path: &str,
    cache: &Arc<Mutex<HashMap<DirCacheKey, DirCacheValue>>>,
) -> (Vec<CompareEntry>, CompareSummary) {
    let left_entries = list_directory_impl(left_path).unwrap_or_default();
    let right_entries = list_directory_impl(right_path).unwrap_or_default();

    let left_map: HashMap<String, &BrowseEntry> = left_entries
        .iter()
        .map(|e| (e.name.to_lowercase(), e))
        .collect();
    let right_map: HashMap<String, &BrowseEntry> = right_entries
        .iter()
        .map(|e| (e.name.to_lowercase(), e))
        .collect();

    let mut all_keys: Vec<String> = left_map
        .keys()
        .chain(right_map.keys())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    all_keys.sort();

    let mut entries = Vec::new();
    let mut summary = CompareSummary::default();
    summary.total_left = left_entries.len();
    summary.total_right = right_entries.len();

    for key in &all_keys {
        let left = left_map.get(key);
        let right = right_map.get(key);

        let (name, kind, status, left_size, right_size, left_modified, right_modified, dir_info) =
            match (left, right) {
                (Some(l), Some(r)) => {
                    if l.kind != r.kind {
                        summary.type_mismatch += 1;
                        (
                            l.name.clone(),
                            l.kind,
                            CompareStatus::TypeMismatch,
                            Some(l.size),
                            Some(r.size),
                            l.modified,
                            r.modified,
                            None,
                        )
                    } else if l.kind == EntryKind::Dir {
                        let sub_left = format!("{}/{}", left_path, l.name);
                        let sub_right = format!("{}/{}", right_path, r.name);
                        let cache_key = (sub_left, sub_right);
                        let cached = cache.lock().unwrap().get(&cache_key).cloned();
                        if let Some((cached_status, cached_size)) = cached {
                            if cached_status == CompareStatus::Same {
                                summary.same += 1;
                            } else {
                                summary.meta_diff += 1;
                            }
                            (
                                l.name.clone(),
                                EntryKind::Dir,
                                cached_status,
                                None,
                                None,
                                l.modified,
                                r.modified,
                                Some(DirResolveInfo { total_size: cached_size }),
                            )
                        } else {
                            // No cache hit — mark as pending
                            (
                                l.name.clone(),
                                EntryKind::Dir,
                                CompareStatus::Pending,
                                None,
                                None,
                                l.modified,
                                r.modified,
                                None,
                            )
                        }
                    } else if l.size == r.size {
                        summary.same += 1;
                        (
                            l.name.clone(),
                            l.kind,
                            CompareStatus::Same,
                            Some(l.size),
                            Some(r.size),
                            l.modified,
                            r.modified,
                            None,
                        )
                    } else {
                        summary.meta_diff += 1;
                        (
                            l.name.clone(),
                            l.kind,
                            CompareStatus::Modified,
                            Some(l.size),
                            Some(r.size),
                            l.modified,
                            r.modified,
                            None,
                        )
                    }
                }
                (Some(l), None) => {
                    summary.only_left += 1;
                    (
                        l.name.clone(),
                        l.kind,
                        CompareStatus::OnlyLeft,
                        Some(l.size),
                        None,
                        l.modified,
                        None,
                        None,
                    )
                }
                (None, Some(r)) => {
                    summary.only_right += 1;
                    (
                        r.name.clone(),
                        r.kind,
                        CompareStatus::OnlyRight,
                        None,
                        Some(r.size),
                        None,
                        r.modified,
                        None,
                    )
                }
                (None, None) => unreachable!(),
            };

        entries.push(CompareEntry {
            name,
            kind,
            status,
            left_size,
            right_size,
            left_modified,
            right_modified,
            dir_info,
        });
    }

    // Sort: dirs first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.kind == EntryKind::Dir;
        let b_is_dir = b.kind == EntryKind::Dir;
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    (entries, summary)
}

/// Recursively checks whether two directories have identical contents.
/// Returns (is_same, total_size) where total_size sums file sizes from the left side.
/// Accepts a cancellation flag that is checked between subdirectories.
fn dirs_are_same_recursive_counted(
    left_path: &str,
    right_path: &str,
    cancel: &AtomicBool,
) -> (bool, u64) {
    if cancel.load(Ordering::Relaxed) {
        return (false, 0);
    }

    let ignore_rules = IgnoreRules::new(&[]);

    let left_entries = match std::fs::read_dir(left_path) {
        Ok(rd) => collect_entries(rd, &ignore_rules),
        Err(_) => Vec::new(),
    };
    let right_entries = match std::fs::read_dir(right_path) {
        Ok(rd) => collect_entries(rd, &ignore_rules),
        Err(_) => Vec::new(),
    };

    let left_map: HashMap<String, (EntryKind, u64)> = left_entries
        .iter()
        .map(|e| (e.name.to_lowercase(), (e.kind, e.size)))
        .collect();
    let right_map: HashMap<String, (EntryKind, u64)> = right_entries
        .iter()
        .map(|e| (e.name.to_lowercase(), (e.kind, e.size)))
        .collect();

    let mut total_size = 0u64;
    let mut is_same = left_map.len() == right_map.len();

    for (key, (l_kind, l_size)) in &left_map {
        if cancel.load(Ordering::Relaxed) {
            return (false, total_size);
        }

        if *l_kind != EntryKind::Dir {
            total_size += l_size;
        }

        if !is_same {
            // Already different, but keep accumulating size
            if *l_kind == EntryKind::Dir {
                let l_name = left_entries
                    .iter()
                    .find(|e| e.name.to_lowercase() == *key)
                    .map(|e| &e.name)
                    .unwrap();
                let sub_left = format!("{}/{}", left_path, l_name);
                let (_, sub_size) = dirs_are_same_recursive_counted(&sub_left, &sub_left, cancel);
                total_size += sub_size;
            }
            continue;
        }

        match right_map.get(key) {
            None => {
                is_same = false;
            }
            Some((r_kind, r_size)) => {
                if l_kind != r_kind {
                    is_same = false;
                } else if *l_kind == EntryKind::Dir {
                    let l_name = left_entries
                        .iter()
                        .find(|e| e.name.to_lowercase() == *key)
                        .map(|e| &e.name)
                        .unwrap();
                    let r_name = right_entries
                        .iter()
                        .find(|e| e.name.to_lowercase() == *key)
                        .map(|e| &e.name)
                        .unwrap();
                    let sub_left = format!("{}/{}", left_path, l_name);
                    let sub_right = format!("{}/{}", right_path, r_name);
                    let (sub_same, sub_size) =
                        dirs_are_same_recursive_counted(&sub_left, &sub_right, cancel);
                    total_size += sub_size;
                    if !sub_same {
                        is_same = false;
                    }
                } else if l_size != r_size {
                    is_same = false;
                }
            }
        }
    }

    (is_same, total_size)
}

/// Resolves pending directory statuses one-by-one, emitting events for each.
#[tauri::command]
pub async fn resolve_dir_statuses(
    left_path: String,
    right_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.dir_resolve_cancel.store(false, Ordering::Relaxed);
    let cancel = Arc::clone(&state.dir_resolve_cancel);
    let cache = Arc::clone(&state.dir_resolve_cache);

    tokio::task::spawn_blocking(move || {
        let ignore_rules = IgnoreRules::new(&[]);

        // Re-read directory to find pending dirs (both sides have same-named dirs)
        let left_entries = match std::fs::read_dir(&left_path) {
            Ok(rd) => collect_entries(rd, &ignore_rules),
            Err(_) => return,
        };
        let right_entries = match std::fs::read_dir(&right_path) {
            Ok(rd) => collect_entries(rd, &ignore_rules),
            Err(_) => return,
        };

        let left_map: HashMap<String, &BrowseEntry> = left_entries
            .iter()
            .map(|e| (e.name.to_lowercase(), e))
            .collect();
        let right_map: HashMap<String, &BrowseEntry> = right_entries
            .iter()
            .map(|e| (e.name.to_lowercase(), e))
            .collect();

        // Collect dirs that exist on both sides with same kind
        let mut pending_dirs: Vec<(String, String, String)> = Vec::new(); // (name, sub_left, sub_right)
        for (key, l) in &left_map {
            if let Some(r) = right_map.get(key) {
                if l.kind == EntryKind::Dir && r.kind == EntryKind::Dir {
                    let sub_left = format!("{}/{}", left_path, l.name);
                    let sub_right = format!("{}/{}", right_path, r.name);
                    pending_dirs.push((l.name.clone(), sub_left, sub_right));
                }
            }
        }

        // Resolve all pending dirs in parallel — small dirs finish fast
        std::thread::scope(|s| {
            for (name, sub_left, sub_right) in pending_dirs {
                let cancel = &cancel;
                let cache = &cache;
                let app = &app;
                let left_path = &left_path;
                let right_path = &right_path;

                s.spawn(move || {
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }

                    let (is_same, total_size) =
                        dirs_are_same_recursive_counted(&sub_left, &sub_right, cancel);

                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }

                    let status = if is_same {
                        CompareStatus::Same
                    } else {
                        CompareStatus::Modified
                    };

                    // Cache the result for reuse on re-navigation
                    cache.lock().unwrap().insert(
                        (sub_left, sub_right),
                        (status, total_size),
                    );

                    let _ = app.emit(
                        EVENT_DIR_STATUS_RESOLVED,
                        DirStatusResolvedPayload {
                            name,
                            status,
                            left_path: left_path.clone(),
                            right_path: right_path.clone(),
                            total_size,
                        },
                    );
                });
            }
        });
    });

    Ok(())
}

/// Cancels any running directory resolution background task.
#[tauri::command]
pub async fn cancel_dir_resolve(state: State<'_, AppState>) -> Result<(), String> {
    state.dir_resolve_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

/// Clears the directory resolve cache. Called when starting a new comparison or returning to browse.
#[tauri::command]
pub async fn clear_dir_resolve_cache(state: State<'_, AppState>) -> Result<(), String> {
    state.dir_resolve_cache.lock().unwrap().clear();
    Ok(())
}

// --- Terminal commands ---

/// Returns a reference to the PTY mutex for the given side.
fn get_pty_mutex<'a>(state: &'a AppState, side: &str) -> Result<&'a Mutex<Option<pty::PtyState>>, String> {
    match side {
        "left" => Ok(&state.pty_left),
        "right" => Ok(&state.pty_right),
        _ => Err(format!("Invalid terminal side: {}", side)),
    }
}

/// Spawns a PTY shell in the given working directory and starts streaming output events.
#[tauri::command]
pub async fn spawn_terminal(
    side: String,
    cwd: String,
    rows: u16,
    cols: u16,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pty_mutex = get_pty_mutex(&state, &side)?;
    {
        let pty_lock = pty_mutex.lock().unwrap();
        if pty_lock.is_some() {
            return Err(format!("Terminal already running on {} side", side));
        }
    }

    let (pty_state, mut reader) = pty::spawn_pty(&cwd, rows, cols)?;
    let reader_active = Arc::clone(&pty_state.reader_active);
    *pty_mutex.lock().unwrap() = Some(pty_state);

    let app_handle = app.clone();
    let side_clone = side.clone();
    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
        loop {
            if !reader_active.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_handle.emit(
                        EVENT_TERMINAL_OUTPUT,
                        TerminalOutputPayload {
                            side: side_clone.clone(),
                            data,
                        },
                    );
                }
                Err(_) => break,
            }
        }
        let _ = app_handle.emit(
            EVENT_TERMINAL_EXIT,
            TerminalExitPayload {
                side: side_clone,
            },
        );
    });

    Ok(())
}

/// Writes data (keystrokes) to the PTY stdin.
#[tauri::command]
pub async fn write_terminal(
    side: String,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pty_mutex = get_pty_mutex(&state, &side)?;
    let pty_lock = pty_mutex.lock().unwrap();
    let pty_state = pty_lock.as_ref().ok_or("No terminal running")?;
    let mut writer = pty_state.writer.lock().unwrap();
    use std::io::Write;
    writer.write_all(data.as_bytes()).map_err(|e: std::io::Error| e.to_string())?;
    writer.flush().map_err(|e: std::io::Error| e.to_string())?;
    Ok(())
}

/// Notifies the PTY of a terminal size change.
#[tauri::command]
pub async fn resize_terminal(
    side: String,
    rows: u16,
    cols: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pty_mutex = get_pty_mutex(&state, &side)?;
    let pty_lock = pty_mutex.lock().unwrap();
    let pty_state = pty_lock.as_ref().ok_or("No terminal running")?;
    let master = pty_state.master.lock().unwrap();
    master
        .resize(portable_pty::PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Resize failed: {}", e))?;
    Ok(())
}

/// Kills the PTY process and cleans up state.
#[tauri::command]
pub async fn kill_terminal(
    side: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pty_mutex = get_pty_mutex(&state, &side)?;
    let mut pty_lock = pty_mutex.lock().unwrap();
    if let Some(pty_state) = pty_lock.take() {
        pty_state
            .reader_active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let mut child = pty_state.child.lock().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

/// Lightweight entry collection for recursive comparison (no modified time needed).
fn collect_entries(
    read_dir: std::fs::ReadDir,
    ignore_rules: &IgnoreRules,
) -> Vec<BrowseEntry> {
    let mut entries = Vec::new();
    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if ignore_rules.is_ignored(&name) {
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let is_symlink = file_type.is_symlink();
        let metadata = entry.metadata().or_else(|_| entry.path().symlink_metadata());
        let (kind, size) = match metadata {
            Ok(m) => {
                let kind = if is_symlink {
                    EntryKind::Symlink
                } else if m.is_dir() {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                };
                (kind, m.len())
            }
            Err(_) => {
                let kind = if is_symlink {
                    EntryKind::Symlink
                } else if file_type.is_dir() {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                };
                (kind, 0)
            }
        };
        entries.push(BrowseEntry {
            name,
            kind,
            size,
            modified: None,
        });
    }
    entries
}

fn list_directory_impl(path: &str) -> Result<Vec<BrowseEntry>, String> {
    let dir = PathBuf::from(path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    let ignore_rules = IgnoreRules::new(&[]);
    let mut entries = Vec::with_capacity(64);

    let read_dir = std::fs::read_dir(&dir).map_err(|e| format!("Cannot read {}: {}", path, e))?;

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();

        if ignore_rules.is_ignored(&name) {
            continue;
        }

        // file_type() doesn't follow symlinks — reliable for detecting symlinks
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let is_symlink = file_type.is_symlink();

        // metadata() follows symlinks. Fall back to symlink_metadata for broken links.
        let metadata = entry.metadata().or_else(|_| entry.path().symlink_metadata());

        let (kind, size, modified) = match metadata {
            Ok(m) => {
                let kind = if is_symlink {
                    EntryKind::Symlink
                } else if m.is_dir() {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                };
                let modified = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64);
                (kind, m.len(), modified)
            }
            Err(_) => {
                // No metadata at all — still show the entry
                let kind = if is_symlink {
                    EntryKind::Symlink
                } else if file_type.is_dir() {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                };
                (kind, 0, None)
            }
        };

        entries.push(BrowseEntry {
            name,
            kind,
            size,
            modified,
        });
    }

    // Sort: dirs first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.kind == EntryKind::Dir;
        let b_is_dir = b.kind == EntryKind::Dir;
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}
