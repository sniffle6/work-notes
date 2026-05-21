pub mod hotkey;
pub mod quick_capture;
pub mod tray;

use tauri::{
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const QUICK_CAPTURE_WINDOW_LABEL: &str = "quick-capture";
const MAIN_WINDOW_TITLE: &str = "Work Notes";
const MAIN_WINDOW_WIDTH: u32 = 1100;
const MAIN_WINDOW_HEIGHT: u32 = 720;
const MAIN_WINDOW_URL: &str = "index.html";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainWindowDefinition {
    pub label: &'static str,
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub url: &'static str,
}

pub fn main_window_definition() -> MainWindowDefinition {
    MainWindowDefinition {
        label: MAIN_WINDOW_LABEL,
        title: MAIN_WINDOW_TITLE,
        width: MAIN_WINDOW_WIDTH,
        height: MAIN_WINDOW_HEIGHT,
        url: MAIN_WINDOW_URL,
    }
}

pub fn bottom_right_position(
    work_area: WorkArea,
    window: WindowSize,
    margin: i32,
) -> WindowPosition {
    WindowPosition {
        x: work_area.x + work_area.width as i32 - window.width as i32 - margin,
        y: work_area.y + work_area.height as i32 - window.height as i32 - margin,
    }
}

pub fn initialize_windowing(
    app: &AppHandle,
    quick_capture_shortcut: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    prepare_main_window(app)?;
    quick_capture::prepare_quick_capture_window(app)?;
    tray::initialize_tray_menu(app)?;
    hotkey::register_global_shortcut(app, quick_capture_shortcut)?;
    Ok(())
}

pub fn prepare_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        install_close_to_tray_handler(&window);
    } else {
        create_main_window(app)?;
    }

    Ok(())
}

pub fn show_main_window<R: Runtime + 'static>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        show_existing_window(&window)?;
    } else {
        let app = app.clone();
        std::thread::spawn(move || {
            if let Ok(window) = create_main_window(&app) {
                let _ = show_existing_window(&window);
            }
        });
    }

    Ok(())
}

fn create_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    let definition = main_window_definition();
    let window = WebviewWindowBuilder::new(
        app,
        definition.label,
        WebviewUrl::App(definition.url.into()),
    )
    .title(definition.title)
    .inner_size(definition.width as f64, definition.height as f64)
    .visible(true)
    .build()?;

    install_close_to_tray_handler(&window);
    Ok(window)
}

fn install_close_to_tray_handler<R: Runtime>(window: &WebviewWindow<R>) {
    let event_window = window.clone();
    let hide_window = window.clone();
    event_window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hide_window.hide();
        }
    });
}

fn show_existing_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.show()?;
    window.unminimize()?;
    window.set_focus()
}

#[cfg(test)]
mod tests {
    use super::{
        bottom_right_position, main_window_definition, WindowPosition, WindowSize, WorkArea,
        MAIN_WINDOW_LABEL,
    };

    #[test]
    fn bottom_right_position_respects_margin() {
        let work_area = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let window = WindowSize {
            width: 560,
            height: 260,
        };

        let position = bottom_right_position(work_area, window, 24);

        assert_eq!(position, WindowPosition { x: 1336, y: 796 });
    }

    #[test]
    fn main_window_definition_matches_configured_inbox_window() {
        let definition = main_window_definition();

        assert_eq!(definition.label, MAIN_WINDOW_LABEL);
        assert_eq!(definition.title, "Work Notes");
        assert_eq!(definition.width, 1100);
        assert_eq!(definition.height, 720);
        assert_eq!(definition.url, "index.html");
    }
}
