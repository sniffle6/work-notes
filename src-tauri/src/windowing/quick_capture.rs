use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use super::{
    bottom_right_position, WindowPosition, WindowSize, WorkArea, QUICK_CAPTURE_WINDOW_LABEL,
};

pub const QUICK_CAPTURE_WIDTH: u32 = 560;
pub const QUICK_CAPTURE_HEIGHT: u32 = 260;
pub const QUICK_CAPTURE_MARGIN: i32 = 24;
pub const FOCUS_NOTE_TEXTAREA_EVENT: &str = "quick-capture:focus-note-textarea";

pub fn prepare_quick_capture_window(app: &AppHandle) -> tauri::Result<()> {
    let window = ensure_quick_capture_window(app)?;
    window.set_always_on_top(true)?;
    window.hide()
}

pub fn show_quick_capture_window(app: &AppHandle) -> tauri::Result<()> {
    let window = ensure_quick_capture_window(app)?;
    window.set_always_on_top(true)?;
    position_quick_capture_window(app, &window)?;
    window.show()?;
    window.unminimize()?;
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
        window.emit_to(
            EventTarget::webview_window(QUICK_CAPTURE_WINDOW_LABEL),
            FOCUS_NOTE_TEXTAREA_EVENT,
            (),
        )?;
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
    .always_on_top(true)
    .visible(false)
    .build()
}

fn position_quick_capture_window(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let monitor = target_monitor(app, window)?;

    if let Some(monitor) = monitor {
        let work_area = monitor.work_area();
        let position = quick_capture_position(WorkArea {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width,
            height: work_area.size.height,
        });

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

fn target_monitor(
    app: &AppHandle,
    window: &WebviewWindow,
) -> tauri::Result<Option<tauri::Monitor>> {
    if let Ok(cursor_position) = app.cursor_position() {
        if let Ok(Some(monitor)) = app.monitor_from_point(cursor_position.x, cursor_position.y) {
            return Ok(Some(monitor));
        }
    }

    match window.current_monitor()? {
        Some(monitor) => Ok(Some(monitor)),
        None => app.primary_monitor(),
    }
}

pub fn quick_capture_position(work_area: WorkArea) -> WindowPosition {
    bottom_right_position(
        WorkArea {
            x: work_area.x,
            y: work_area.y,
            width: work_area.width,
            height: work_area.height,
        },
        WindowSize {
            width: QUICK_CAPTURE_WIDTH,
            height: QUICK_CAPTURE_HEIGHT,
        },
        QUICK_CAPTURE_MARGIN,
    )
}

fn apply_window_geometry(
    window: &WebviewWindow,
    size: WindowSize,
    position: WindowPosition,
) -> tauri::Result<()> {
    window.set_size(PhysicalSize::new(size.width, size.height))?;
    window.set_position(PhysicalPosition::new(position.x, position.y))
}

#[cfg(test)]
mod tests {
    use super::{quick_capture_position, WindowPosition, WorkArea};

    #[test]
    fn quick_capture_position_uses_window_defaults_and_monitor_origin() {
        let work_area = WorkArea {
            x: -1600,
            y: 100,
            width: 1600,
            height: 900,
        };

        let position = quick_capture_position(work_area);

        assert_eq!(position, WindowPosition { x: -584, y: 716 });
    }
}
