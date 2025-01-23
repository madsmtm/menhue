#![deny(unsafe_op_in_unsafe_fn)]
use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, DeclaredClass, MainThreadOnly, Message};
use objc2_app_kit::{
    NSImage, NSMenu, NSMenuDelegate, NSMenuItem, NSStatusBar, NSStatusItem, NSStatusItemBehavior,
    NSVariableStatusItemLength,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSDictionary, NSMutableArray, NSNumber, NSObject,
    NSObjectProtocol, NSString,
};

use crate::api::Session;
use crate::light_controller::LightController;
use crate::AppDelegate;

#[derive(Debug)]
pub struct Ivars {
    _status_bar_item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    session: Session,
    /// Keep references to the light controllers around
    light_controllers: RefCell<Retained<NSMutableArray<LightController>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MenuDelegate"]
    #[ivars = Ivars]
    #[derive(Debug)]
    pub struct MenuDelegate;

    unsafe impl NSObjectProtocol for MenuDelegate {}

    #[allow(non_snake_case)]
    unsafe impl NSMenuDelegate for MenuDelegate {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menuNeedsUpdate(&self, _menu: &NSMenu) {
            self.needs_update();
        }
    }
);

const TAG_LOADING: isize = 1;
const TAG_LIGHT: isize = 2;

impl MenuDelegate {
    pub fn new(app_delegate: &AppDelegate, session: Session) -> Retained<Self> {
        let mtm = MainThreadMarker::from(app_delegate);
        unsafe {
            let status_bar = NSStatusBar::systemStatusBar();
            let status_bar_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

            //
            status_bar_item.setBehavior(NSStatusItemBehavior::TerminationOnRemoval);
            status_bar_item.setVisible(true);

            // Documented to create a button for us
            let button = status_bar_item
                .button(mtm)
                .expect("the system did not create a status bar button");

            let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                ns_string!("lightbulb"),
                None,
            )
            .unwrap();
            button.setImage(Some(&image));

            let menu = NSMenu::new(mtm);
            status_bar_item.setMenu(Some(&menu));

            let this = mtm.alloc().set_ivars(Ivars {
                _status_bar_item: status_bar_item,
                menu,
                session,
                light_controllers: RefCell::new(NSMutableArray::new()),
            });
            let this: Retained<Self> = msg_send![super(this), init];

            let menu = &this.ivars().menu;
            menu.setDelegate(Some(ProtocolObject::from_ref(&*this)));

            let item = NSMenuItem::new(mtm);
            item.setTitle(ns_string!("Loading..."));
            item.setHidden(true);
            item.setTag(TAG_LOADING);
            menu.addItem(&item);

            let item = NSMenuItem::separatorItem(mtm);
            menu.addItem(&item);

            let item = NSMenuItem::new(mtm);
            item.setTitle(ns_string!("Preferences..."));
            item.setTarget(Some(app_delegate));
            item.setAction(Some(sel!(openPreferences:)));
            menu.addItem(&item);

            let item = NSMenuItem::separatorItem(mtm);
            menu.addItem(&item);

            let item = NSMenuItem::new(mtm);
            item.setTitle(ns_string!("Quit"));
            item.setAction(Some(sel!(terminate:)));
            menu.addItem(&item);

            this
        }
    }

    fn set_loading(&self, loading: bool) {
        unsafe {
            let item = self
                .ivars()
                .menu
                .itemWithTag(TAG_LOADING)
                .expect("loading item");
            item.setHidden(!loading);
        }
    }

    fn update_lights(&self, obj: &AnyObject) {
        let mtm = MainThreadMarker::from(self);
        let menu = &self.ivars().menu;
        unsafe {
            let light_controllers = self.ivars().light_controllers.borrow_mut();

            // Clear existing menus
            while let Some(item) = menu.itemWithTag(TAG_LIGHT) {
                menu.removeItem(&item);
            }
            light_controllers.removeAllObjects();

            let data = obj
                .downcast_ref::<NSDictionary>()
                .expect("invalid response");

            // Add new menus
            for (i, light_id) in data.keys().enumerate() {
                let light_id = light_id.downcast::<NSString>().expect("invalid response");

                let dict = data
                    .objectForKey(&light_id)
                    .unwrap()
                    .downcast::<NSDictionary>()
                    .expect("invalid response");

                let name = dict
                    .objectForKey(ns_string!("name"))
                    .expect("name")
                    .downcast::<NSString>()
                    .expect("invalid name");

                let state = dict
                    .objectForKey(ns_string!("state"))
                    .expect("state")
                    .downcast::<NSDictionary>()
                    .expect("invalid state");

                let reachable = state
                    .objectForKey(ns_string!("reachable"))
                    .expect("reachable")
                    .downcast::<NSNumber>()
                    .expect("invalid reachable")
                    .as_bool();

                if !reachable {
                    // Ignore light if not reachable
                    continue;
                }

                let bri = state
                    .objectForKey(ns_string!("bri"))
                    .expect("bri")
                    .downcast::<NSNumber>()
                    .expect("invalid bri")
                    .integerValue();

                let light_control =
                    LightController::new(&light_id, &name, bri, self.ivars().session.clone(), mtm);

                let item = NSMenuItem::new(mtm);
                item.setTitle(&name);
                item.setView(Some(light_control.view()));
                item.setTag(TAG_LIGHT);
                menu.insertItem_atIndex(&item, i as isize + 1);

                light_controllers.addObject(&light_control);
            }
        }
    }

    fn needs_update(&self) {
        self.set_loading(true);

        let this = self.retain();
        self.ivars().session.request(
            ns_string!("GET"),
            &self.ivars().session.authenticated_path("/lights"),
            None,
            move |res| match res {
                Ok(obj) => {
                    this.set_loading(false);
                    this.update_lights(&obj);
                }
                Err(err) => {
                    eprintln!("failed fetching lights: {err}");
                    this.set_loading(false);
                }
            },
        );
    }
}
