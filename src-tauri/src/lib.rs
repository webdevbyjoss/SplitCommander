mod core;

use core::commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            core::commands::set_root,
            core::commands::start_compare,
            core::commands::cancel_compare,
            core::commands::get_diffs,
            core::commands::get_summary,
            core::commands::export_report,
            core::commands::init_browse,
            core::commands::list_directory,
            core::commands::open_file,
            core::commands::copy_entry,
            core::commands::move_entry,
            core::commands::create_directory,
            core::commands::delete_entry,
            core::commands::compare_directory,
            core::commands::resolve_dir_statuses,
            core::commands::cancel_dir_resolve,
            core::commands::clear_dir_resolve_cache,
            core::commands::spawn_terminal,
            core::commands::write_terminal,
            core::commands::resize_terminal,
            core::commands::kill_terminal,
            core::commands::load_app_state,
            core::commands::save_app_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
