pub mod hotkey;
pub mod quick_capture;
pub mod tray;

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const QUICK_CAPTURE_WINDOW_LABEL: &str = "quick-capture";

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

#[cfg(test)]
mod tests {
    use super::{bottom_right_position, WindowPosition, WindowSize, WorkArea};

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
}
