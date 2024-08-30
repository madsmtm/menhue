use objc2_app_kit::NSApplication;
use objc2_foundation::MainThreadMarker;

pub fn open_preferences(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    #[allow(deprecated)] // The newer `activate` is only available on macOS 14
    app.activateIgnoringOtherApps(false);
    unsafe { app.orderFrontStandardAboutPanel(None) };
    // deactivate on close
}
