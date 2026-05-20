use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use super::{
    bottom_right_position, WindowPosition, WindowSize, WorkArea, QUICK_CAPTURE_WINDOW_LABEL,
};

pub const QUICK_CAPTURE_WIDTH: u32 = 560;
pub const QUICK_CAPTURE_HEIGHT: u32 = 260;
pub const QUICK_CAPTURE_MARGIN: i32 = 24;
pub const FOCUS_NOTE_TEXTAREA_EVENT: &str = "quick-capture:focus-note-textarea";

pub fn show_quick_capture_window(app: &AppHandle) -> tauri::Result<()> {
    let window = ensure_quick_capture_window(app)?;
    position_quick_capture_window(app, &window)?;
    window.show()?;
    window.set_focus()?;
    focus_note_textarea(app)
}

pub fn hide_quick_capture_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(QUICK_CAPTURE_WINDOW_LABEL) {
        window.hide()?;
    }

    Ok(())
}

pub fn focus_note_textarea(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(QUICK_CAPTURE_WINDOW_LABEL) {
        window.emit(FOCUS_NOTE_TEXTAREA_EVENT, ())?;
    }

    Ok(())
}

pub fn ensure_quick_capture_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(QUICK_CAPTURE_WINDOW_LABEL) {
        return Ok(window);
    }

    WebviewWindowBuilder::new(
        app,
        QUICK_CAPTURE_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Quick Capture")
    .inner_size(QUICK_CAPTURE_WIDTH as f64, QUICK_CAPTURE_HEIGHT as f64)
    .resizable(false)
    .decorations(false)
    .skip_taskbar(true)
    .visible(false)
    .build()
}

fn position_quick_capture_window(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let monitor = match window.current_monitor()? {
        Some(monitor) => Some(monitor),
        None => app.primary_monitor()?,
    };

    if let Some(monitor) = monitor {
        let work_area = monitor.work_area();
        let position = bottom_right_position(
            WorkArea {
                x: work_area.position.x,
                y: work_area.position.y,
                width: work_area.size.width,
                height: work_area.size.height,
            },
            WindowSize {
                width: QUICK_CAPTURE_WIDTH,
                height: QUICK_CAPTURE_HEIGHT,
            },
            QUICK_CAPTURE_MARGIN,
        );

        apply_window_geometry(
            window,
            WindowSize {
                width: QUICK_CAPTURE_WIDTH,
                height: QUICK_CAPTURE_HEIGHT,
            },
            position,
        )?;
    }

    Ok(())
}

fn apply_window_geometry(
    window: &WebviewWindow,
    size: WindowSize,
    position: WindowPosition,
) -> tauri::Result<()> {
    window.set_size(PhysicalSize::new(size.width, size.height))?;
    window.set_position(PhysicalPosition::new(position.x, position.y))
}
