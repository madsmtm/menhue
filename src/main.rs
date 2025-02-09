#![deny(unsafe_op_in_unsafe_fn)]
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, DeclaredClass, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSString};

use crate::api::Session;
use crate::menu::MenuDelegate;

mod api;
mod light_controller;
mod menu;
mod preferences;

#[derive(Debug)]
struct Ivars {
    session: Session,
    menu: OnceCell<Retained<MenuDelegate>>,
    username: Rc<RefCell<Option<Retained<NSString>>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AppDelegate"]
    #[ivars = Ivars]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn _did_finish_launching(&self, _notification: &NSNotification) {
            self.init();
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn _will_terminate(&self, _notification: &NSNotification) {
            self.destroy();
        }
    }

    /// Menu actions.
    impl AppDelegate {
        #[unsafe(method(openPreferences:))]
        fn _open_preferences(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            eprintln!("open prefs");
            preferences::open_preferences(mtm);
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        // TODO: Store this with CoreData
        let host = std::env::var("HOST")
            .map(|var| NSString::from_str(&var))
            .ok();
        let host = Rc::new(RefCell::new(host));

        // TODO: Store this with CoreData
        let username = std::env::var("USERNAME_KEY")
            .map(|var| NSString::from_str(&var))
            .ok();
        let username = Rc::new(RefCell::new(username));

        let this = mtm.alloc().set_ivars(Ivars {
            session: Session::new(mtm, host, username.clone()),
            menu: OnceCell::new(),
            username,
        });
        unsafe { msg_send![super(this), init] }
    }

    fn init(&self) {
        println!("bar");
        eprintln!("foo");
        self.ivars()
            .menu
            .set(MenuDelegate::new(self, self.ivars().session.clone()))
            .expect("only initialized menu once");

        let username = self.ivars().username.borrow();
        if username.is_none() {
            drop(username);
            self.ivars().session.connect(move |res| match res {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed logging in: {err}");
                }
            });
        }
    }

    fn destroy(&self) {
        self.ivars().session.destroy();
    }
}

fn main() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let delegate = AppDelegate::new(mtm);
    let object = ProtocolObject::from_ref(&*delegate);
    app.setDelegate(Some(object));

    app.run();
}
