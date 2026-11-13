pub mod app_state;
pub mod commands;
pub mod db;
pub mod domain;
pub mod jira;
pub mod parser;
pub mod repositories;
pub mod services;
pub mod theme;
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let project_dirs = directories::ProjectDirs::from("com", "aweber", "Work Notes")
                .ok_or_else(|| {
                    IoError::new(ErrorKind::NotFound, "app data directory unavailable")
                })?;
            std::fs::create_dir_all(project_dirs.data_dir())?;
            let database = Database::open(project_dirs.data_dir().join("work-notes.sqlite3"))?;
            let state = AppState::new(database);
            app.handle().plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))?;
            let app_settings = state.settings.get().unwrap_or_default();
            let quick_capture_shortcut = app_settings.global_hotkey.clone();
            let quick_capture_shortcut = windowing::hotkey::validate_shortcut(
                &quick_capture_shortcut,
            )
            .unwrap_or_else(|_| windowing::hotkey::DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string());
            let settings_service = state.settings.clone();
            let parse_worker = services::parse_queue::ParseQueue::with_runtime_settings(
                state.repositories.clone(),
                state.settings.clone(),
                state.parse_queue_config.clone(),
                state.parser_provider_config.clone(),
            );
            parse_worker.requeue_interrupted_jobs()?;
            let _ = windowing::startup::apply_launch_at_startup_setting(
                app.handle(),
                app_settings.launch_at_startup,
            );
            app.manage(state);
            parse_worker.start_background_worker()?;
            windowing::initialize_windowing(
                app.handle(),
                &quick_capture_shortcut,
                settings_service,
            )?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_capture_note,
            commands::list_inbox,
            commands::get_note,
            commands::retry_parse,
            commands::retry_parse_with_feedback,
            commands::cancel_parse,
            commands::update_note_cleaned,
            commands::update_note_raw,
            commands::delete_note,
            commands::complete_note,
            commands::add_card_note,
            commands::reopen_note,
            commands::restore_note,
            commands::permanently_delete_note,
            commands::accept_action_item,
            commands::dismiss_action_item,
            commands::complete_action_item,
            commands::reopen_action_item,
            commands::drop_followup,
            commands::list_suggested_actions,
            commands::list_followups,
            commands::create_manual_followup,
            commands::update_followup_state,
            commands::update_followup_lane,
            commands::get_settings,
            commands::save_settings,
            commands::hide_quick_capture,
            commands::jira::get_jira_status,
            commands::jira::test_jira_connection,
            commands::jira::list_jira_issues,
            commands::jira::get_jira_issue,
            commands::jira::list_jira_issue_types,
            commands::jira::create_jira_issue,
            commands::jira::update_jira_issue,
            commands::jira::transition_jira_issue,
            commands::jira::add_jira_comment,
            commands::jira::assign_jira_issue_to_me,
            commands::jira::download_jira_attachments,
            commands::jira::get_jira_attachment_thumbnail,
            commands::jira::read_jira_attachment_media,
            commands::jira::list_jira_chat_threads,
            commands::jira::create_jira_chat_thread,
            commands::jira::delete_jira_chat_thread,
            commands::jira::list_jira_chat_messages,
            commands::jira::send_jira_chat_message,
            commands::jira::resolve_jira_chat_proposal,
            commands::theme::generate_theme
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
