use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::compare;
use crate::core::events::*;
use crate::core::export;
use crate::core::ignore::IgnoreRules;
use crate::core::model::*;
use crate::core::scan;

/// Shared application state managed by Tauri.
pub struct AppState {
    pub left_root: Mutex<Option<PathBuf>>,
    pub right_root: Mutex<Option<PathBuf>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub last_result: Mutex<Option<LastCompareResult>>,
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
            last_result: Mutex::new(None),
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
