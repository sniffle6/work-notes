pub mod app_state;
pub mod commands;
pub mod db;
pub mod domain;
pub mod parser;
pub mod repositories;
pub mod services;
pub mod windowing;

use std::io::{Error as IoError, ErrorKind};

use app_state::AppState;
use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = windowing::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let project_dirs = directories::ProjectDirs::from("com", "aweber", "Work Notes")
                .ok_or_else(|| {
                    IoError::new(ErrorKind::NotFound, "app data directory unavailable")
                })?;
            std::fs::create_dir_all(project_dirs.data_dir())?;
            let database = Database::open(project_dirs.data_dir().join("work-notes.sqlite3"))?;
            let state = AppState::new(database);
            let quick_capture_shortcut = state
                .settings
                .get()
                .map(|settings| settings.global_hotkey)
                .unwrap_or_else(|_| windowing::hotkey::DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string());
            app.manage(state);
            windowing::initialize_windowing(app.handle(), &quick_capture_shortcut)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_capture_note,
            commands::list_inbox,
            commands::get_note,
            commands::retry_parse,
            commands::accept_action_item,
            commands::dismiss_action_item,
            commands::get_settings,
            commands::save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
