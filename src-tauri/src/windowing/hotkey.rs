use std::str::FromStr;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use super::quick_capture::show_quick_capture_window;

pub const DEFAULT_QUICK_CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Space";

pub fn normalized_shortcut(configured_shortcut: &str) -> String {
    let shortcut = configured_shortcut.trim();
    if shortcut.is_empty() {
        DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string()
    } else {
        shortcut.to_string()
    }
}

pub fn validate_shortcut(
    configured_shortcut: &str,
) -> std::result::Result<String, tauri_plugin_global_shortcut::Error> {
    let shortcut = normalized_shortcut(configured_shortcut);
    Shortcut::from_str(&shortcut)?;
    Ok(shortcut)
}

pub fn register_global_shortcut(
    app: &AppHandle,
    configured_shortcut: &str,
) -> std::result::Result<(), tauri_plugin_global_shortcut::Error> {
    let shortcut = validate_shortcut(configured_shortcut)?;
    let shortcuts = app.global_shortcut();

    shortcuts.unregister_all()?;

    shortcuts.on_shortcut(shortcut.as_str(), |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = show_quick_capture_window(app);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{normalized_shortcut, validate_shortcut, DEFAULT_QUICK_CAPTURE_SHORTCUT};

    #[test]
    fn empty_shortcut_uses_default() {
        assert_eq!(normalized_shortcut("   "), DEFAULT_QUICK_CAPTURE_SHORTCUT);
    }

    #[test]
    fn invalid_shortcut_is_rejected_before_persisting_settings() {
        assert!(validate_shortcut("not a real shortcut").is_err());
    }
}
