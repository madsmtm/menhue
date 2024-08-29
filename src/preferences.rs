use objc2_app_kit::NSApplication;
use objc2_foundation::MainThreadMarker;

pub fn open_preferences(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.activate() };
    unsafe { app.orderFrontStandardAboutPanel(None) };
    // deactivate on close
}
