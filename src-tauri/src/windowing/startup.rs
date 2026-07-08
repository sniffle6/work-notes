use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::{AutoLaunchManager, ManagerExt};

pub trait LaunchAtStartupController {
    type Error;

    fn enable(&self) -> Result<(), Self::Error>;
    fn disable(&self) -> Result<(), Self::Error>;
}

impl LaunchAtStartupController for AutoLaunchManager {
    type Error = tauri_plugin_autostart::Error;

    fn enable(&self) -> Result<(), Self::Error> {
        AutoLaunchManager::enable(self)
    }

    fn disable(&self) -> Result<(), Self::Error> {
        AutoLaunchManager::disable(self)
    }
}

pub fn apply_launch_at_startup<C: LaunchAtStartupController>(
    controller: &C,
    launch_at_startup: bool,
) -> Result<(), C::Error> {
    if launch_at_startup {
        controller.enable()
    } else {
        controller.disable()
    }
}

pub fn apply_launch_at_startup_setting<R: Runtime>(
    app: &AppHandle<R>,
    launch_at_startup: bool,
) -> Result<(), tauri_plugin_autostart::Error> {
    let controller = app.autolaunch();
    apply_launch_at_startup(&*controller, launch_at_startup)
}

#[cfg(test)]
mod tests {
    use super::{apply_launch_at_startup, LaunchAtStartupController};
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeStartupController {
        calls: RefCell<Vec<&'static str>>,
    }

    impl LaunchAtStartupController for FakeStartupController {
        type Error = std::convert::Infallible;

        fn enable(&self) -> Result<(), Self::Error> {
            self.calls.borrow_mut().push("enable");
            Ok(())
        }

        fn disable(&self) -> Result<(), Self::Error> {
            self.calls.borrow_mut().push("disable");
            Ok(())
        }
    }

    #[test]
    fn apply_launch_at_startup_enables_native_autostart_when_requested() {
        let controller = FakeStartupController::default();

        apply_launch_at_startup(&controller, true).unwrap();

        assert_eq!(*controller.calls.borrow(), vec!["enable"]);
    }

    #[test]
    fn apply_launch_at_startup_disables_native_autostart_when_unrequested() {
        let controller = FakeStartupController::default();

        apply_launch_at_startup(&controller, false).unwrap();

        assert_eq!(*controller.calls.borrow(), vec!["disable"]);
    }
}
