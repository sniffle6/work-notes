use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use super::{
    quick_capture::show_quick_capture_window, show_main_window, MAIN_WINDOW_LABEL,
    QUICK_CAPTURE_WINDOW_LABEL,
};

const OPEN_MAIN_MENU_ID: &str = MAIN_WINDOW_LABEL;
const SHOW_QUICK_CAPTURE_MENU_ID: &str = QUICK_CAPTURE_WINDOW_LABEL;
const CHECK_UPDATES_MENU_ID: &str = "check-for-updates";
const QUIT_MENU_ID: &str = "quit";
const TRAY_ID: &str = "work-notes-tray";
const CHECK_FOR_UPDATES_EVENT: &str = "work-notes:check-for-updates";

pub fn initialize_tray_menu(app: &AppHandle) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, OPEN_MAIN_MENU_ID, "Open", true, None::<&str>)?;
    let show_quick_capture = MenuItem::with_id(
        app,
        SHOW_QUICK_CAPTURE_MENU_ID,
        "Quick Note",
        true,
        None::<&str>,
    )?;
    let check_updates = MenuItem::with_id(
        app,
        CHECK_UPDATES_MENU_ID,
        "Check for updates",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_main, &show_quick_capture, &check_updates, &quit])?;

    let open_main_id = open_main.id().clone();
    let show_quick_capture_id = show_quick_capture.id().clone();
    let check_updates_id = check_updates.id().clone();
    let quit_id = quit.id().clone();

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Work Notes")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            if event.id() == &open_main_id {
                let _ = show_main_window(app);
            } else if event.id() == &show_quick_capture_id {
                let _ = show_quick_capture_window(app);
            } else if event.id() == &check_updates_id {
                // The main window is created at startup (prepare_main_window) and only hidden
                // on close, never destroyed, so show_main_window takes its synchronous branch
                // and get_webview_window returns Some here.
                let _ = show_main_window(app);
                if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = window.emit(CHECK_FOR_UPDATES_EVENT, ());
                }
            } else if event.id() == &quit_id {
                app.exit(0);
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}
