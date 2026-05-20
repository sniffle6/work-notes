use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::quick_capture::show_quick_capture_window;

pub const QUICK_CAPTURE_SHORTCUT: &str = "CmdOrCtrl+Shift+Space";

pub fn register_global_shortcut(
    app: &AppHandle,
) -> std::result::Result<(), tauri_plugin_global_shortcut::Error> {
    let shortcuts = app.global_shortcut();

    if shortcuts.is_registered(QUICK_CAPTURE_SHORTCUT) {
        return Ok(());
    }

    shortcuts.on_shortcut(QUICK_CAPTURE_SHORTCUT, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = show_quick_capture_window(app);
        }
    })
}
