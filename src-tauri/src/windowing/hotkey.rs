use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::quick_capture::show_quick_capture_window;

pub const DEFAULT_QUICK_CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Space";

pub fn register_global_shortcut(
    app: &AppHandle,
    configured_shortcut: &str,
) -> std::result::Result<(), tauri_plugin_global_shortcut::Error> {
    let shortcut = configured_shortcut.trim();
    let shortcut = if shortcut.is_empty() {
        DEFAULT_QUICK_CAPTURE_SHORTCUT
    } else {
        shortcut
    };
    let shortcuts = app.global_shortcut();

    if shortcuts.is_registered(shortcut) {
        return Ok(());
    }

    shortcuts.on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = show_quick_capture_window(app);
        }
    })
}
