use tauri::{
    menu::{Menu, MenuItem},
    AppHandle, Manager,
};

use super::{
    quick_capture::{hide_quick_capture_window, show_quick_capture_window},
    MAIN_WINDOW_LABEL, QUICK_CAPTURE_WINDOW_LABEL,
};

const OPEN_MAIN_MENU_ID: &str = MAIN_WINDOW_LABEL;
const SHOW_QUICK_CAPTURE_MENU_ID: &str = QUICK_CAPTURE_WINDOW_LABEL;
const HIDE_QUICK_CAPTURE_MENU_ID: &str = "hide-quick-capture";

pub fn initialize_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let open_main = MenuItem::with_id(
        app,
        OPEN_MAIN_MENU_ID,
        "Open Work Notes",
        true,
        None::<&str>,
    )?;
    let show_quick_capture = MenuItem::with_id(
        app,
        SHOW_QUICK_CAPTURE_MENU_ID,
        "Quick Capture",
        true,
        None::<&str>,
    )?;
    let hide_quick_capture = MenuItem::with_id(
        app,
        HIDE_QUICK_CAPTURE_MENU_ID,
        "Hide Quick Capture",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&open_main, &show_quick_capture, &hide_quick_capture])?;

    let open_main_id = open_main.id().clone();
    let show_quick_capture_id = show_quick_capture.id().clone();
    let hide_quick_capture_id = hide_quick_capture.id().clone();

    app.on_menu_event(move |app, event| {
        if event.id() == &open_main_id {
            let _ = show_main_window(app);
        } else if event.id() == &show_quick_capture_id {
            let _ = show_quick_capture_window(app);
        } else if event.id() == &hide_quick_capture_id {
            let _ = hide_quick_capture_window(app);
        }
    });

    Ok(menu)
}

fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
    }

    Ok(())
}
