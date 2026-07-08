pub mod hotkey;
pub mod quick_capture;
pub mod startup;
pub mod tray;

use tauri::{
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::{app_state::AppState, services::settings::SettingsService};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    HideToTray,
    ExitApplication,
}

pub fn close_action_for_minimize_to_tray(minimize_to_tray: bool) -> CloseAction {
    if minimize_to_tray {
        CloseAction::HideToTray
    } else {
        CloseAction::ExitApplication
    }
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
    settings: SettingsService,
) -> Result<(), Box<dyn std::error::Error>> {
    prepare_main_window(app, settings)?;
    quick_capture::prepare_quick_capture_window(app)?;
    tray::initialize_tray_menu(app)?;
    hotkey::register_global_shortcut(app, quick_capture_shortcut)?;
    Ok(())
}

pub fn prepare_main_window<R: Runtime>(
    app: &AppHandle<R>,
    settings: SettingsService,
) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        apply_default_window_icons(app, &window)?;
        install_close_to_tray_handler(&window, Some(settings));
    } else {
        create_main_window(app, Some(settings))?;
    }

    Ok(())
}

pub fn show_main_window<R: Runtime + 'static>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        show_existing_window(&window)?;
    } else {
        let settings = settings_service(app);
        let app = app.clone();
        std::thread::spawn(move || {
            if let Ok(window) = create_main_window(&app, settings) {
                let _ = show_existing_window(&window);
            }
        });
    }

    Ok(())
}

fn create_main_window<R: Runtime>(
    app: &AppHandle<R>,
    settings: Option<SettingsService>,
) -> tauri::Result<WebviewWindow<R>> {
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

    apply_default_window_icons(app, &window)?;
    install_close_to_tray_handler(&window, settings);
    Ok(window)
}

fn settings_service<R: Runtime>(app: &AppHandle<R>) -> Option<SettingsService> {
    app.try_state::<AppState>()
        .map(|state| state.settings.clone())
}

fn apply_default_window_icons<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
) -> tauri::Result<()> {
    if let Some(icon) = app.default_window_icon().cloned() {
        window.set_icon(icon.clone())?;
        #[cfg(target_os = "windows")]
        windows_taskbar_icon::set(window, &icon)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
mod windows_taskbar_icon {
    use std::{
        io::{Error as IoError, ErrorKind},
        sync::Mutex,
    };

    use tauri::{image::Image, Runtime, WebviewWindow};
    use windows::Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{CreateIcon, SendMessageW, HICON, ICON_BIG, WM_SETICON},
    };

    static TASKBAR_ICON_HANDLES: Mutex<Vec<isize>> = Mutex::new(Vec::new());

    pub fn set<R: Runtime>(window: &WebviewWindow<R>, icon: &Image<'_>) -> tauri::Result<()> {
        let hicon = create_hicon(icon)?;
        let hwnd = window.hwnd()?;

        unsafe {
            SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(ICON_BIG as usize)),
                Some(LPARAM(hicon.0 as isize)),
            );
        }

        // Keep the icon resource alive for the process lifetime after Windows stores the handle.
        if let Ok(mut handles) = TASKBAR_ICON_HANDLES.lock() {
            handles.push(hicon.0 as isize);
        }

        Ok(())
    }

    fn create_hicon(icon: &Image<'_>) -> tauri::Result<HICON> {
        let width = icon.width();
        let height = icon.height();
        let expected_len = width as usize * height as usize * 4;
        let mut bgra = icon.rgba().to_vec();

        if bgra.len() != expected_len {
            return Err(io_error(format!(
                "icon RGBA buffer has {} bytes, expected {}",
                bgra.len(),
                expected_len
            )));
        }

        let mut and_mask = Vec::with_capacity(width as usize * height as usize);
        for pixel in bgra.chunks_exact_mut(4) {
            and_mask.push(pixel[3].wrapping_sub(u8::MAX));
            pixel.swap(0, 2);
        }

        unsafe {
            CreateIcon(
                None,
                width as i32,
                height as i32,
                1,
                32,
                and_mask.as_ptr(),
                bgra.as_ptr(),
            )
        }
        .map_err(|error| io_error(format!("failed to create taskbar icon: {error}")))
    }

    fn io_error(message: String) -> tauri::Error {
        IoError::new(ErrorKind::Other, message).into()
    }
}

fn install_close_to_tray_handler<R: Runtime>(
    window: &WebviewWindow<R>,
    settings: Option<SettingsService>,
) {
    let event_window = window.clone();
    let hide_window = window.clone();
    let app = window.app_handle().clone();
    event_window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let close_action = settings
                .as_ref()
                .and_then(|settings| settings.get().ok())
                .map(|settings| close_action_for_minimize_to_tray(settings.minimize_to_tray))
                .unwrap_or(CloseAction::HideToTray);

            match close_action {
                CloseAction::HideToTray => {
                    let _ = hide_window.hide();
                }
                CloseAction::ExitApplication => {
                    app.exit(0);
                }
            }
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
        bottom_right_position, close_action_for_minimize_to_tray, main_window_definition,
        CloseAction, WindowPosition, WindowSize, WorkArea, MAIN_WINDOW_LABEL,
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

    #[test]
    fn close_action_hides_main_window_when_minimize_to_tray_is_enabled() {
        assert_eq!(
            close_action_for_minimize_to_tray(true),
            CloseAction::HideToTray
        );
    }

    #[test]
    fn close_action_exits_app_when_minimize_to_tray_is_disabled() {
        assert_eq!(
            close_action_for_minimize_to_tray(false),
            CloseAction::ExitApplication
        );
    }
}
