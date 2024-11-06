use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use objc2::{
    declare_class, msg_send_id, rc::Retained, runtime::AnyObject, sel, ClassType, DeclaredClass,
    MainThreadOnly, Message,
};
use objc2_app_kit::{
    NSLayoutAttribute, NSLayoutConstraint, NSSlider, NSStackView, NSStackViewDistribution,
    NSTextField, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{
    ns_string, CGSize, MainThreadMarker, NSArray, NSCopying, NSDictionary, NSInteger, NSNumber,
    NSObject, NSObjectNSDelayedPerforming, NSObjectProtocol, NSRunLoopCommonModes, NSString,
};

use crate::api::Session;

#[derive(Debug)]
pub struct Ivars {
    light_id: Retained<NSString>,
    view: Retained<NSView>,
    slider: Retained<NSSlider>,
    session: Session,
    last_updated_bri: Cell<Instant>,
}

declare_class!(
    #[derive(Debug)]
    pub struct LightController;

    unsafe impl ClassType for LightController {
        type Super = NSObject;
        type ThreadKind = dyn MainThreadOnly;
        const NAME: &'static str = "LightControl";
    }

    impl DeclaredClass for LightController {
        type Ivars = Ivars;
    }

    unsafe impl NSObjectProtocol for LightController {}

    unsafe impl LightController {
        #[method(dragSlider:)]
        fn _drag_slider(&self, _slider: &NSSlider) {
            self.queue_update_bri();
        }

        #[method(setBri:)]
        fn _update_bri_from_slider(&self, _: Option<&AnyObject>) {
            self.update_bri_from_slider();
        }
    }
);

impl LightController {
    pub fn new(
        light_id: &NSString,
        name: &NSString,
        bri: NSInteger,
        session: Session,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        unsafe {
            let view = NSView::new(mtm);
            view.setFrameSize(CGSize {
                height: 100.0,
                width: 300.0,
            });
            // view.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
            // view.setTranslatesAutoresizingMaskIntoConstraints(false);

            let stack = NSStackView::new(mtm);
            stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
            stack.setDistribution(NSStackViewDistribution::FillEqually);
            stack.setAlignment(NSLayoutAttribute::Left);
            view.addSubview(&stack);
            stack.setTranslatesAutoresizingMaskIntoConstraints(false);

            let label = NSTextField::labelWithString(name, mtm);
            // label.setStringValue(name);
            // label.setBackgroundColor(Some(&NSColor::colorWithRed_green_blue_alpha(
            //     0.0, 0.0, 0.0, 0.0,
            // )));
            // label.setBezeled(false);
            // label.setEditable(false);
            stack.addArrangedSubview(&label);

            let slider = NSSlider::new(mtm);
            slider.setFrameSize(CGSize {
                height: 50.0,
                width: 250.0,
            });
            slider.setMinValue(0.0);
            slider.setMaxValue(254.0);
            slider.setIntegerValue(bri);
            stack.addArrangedSubview(&slider);

            NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&[
                stack.leftAnchor().constraintEqualToAnchor_constant(
                    &view.layoutMarginsGuide().leftAnchor(),
                    -4.0,
                ),
                stack.rightAnchor().constraintEqualToAnchor_constant(
                    &view.layoutMarginsGuide().rightAnchor(),
                    4.0,
                ),
                // {
                //     let constraint = stack
                //         .widthAnchor()
                //         .constraintEqualToAnchor(&view.widthAnchor());
                //     constraint.setPriority(NSLayoutPriorityDefaultLow);
                //     constraint
                // },
                view.heightAnchor()
                    .constraintEqualToAnchor(&stack.heightAnchor()),
            ]));

            let this = mtm.alloc().set_ivars(Ivars {
                light_id: light_id.copy(),
                view,
                slider: slider.retain(),
                session,
                last_updated_bri: Cell::new(Instant::now()),
            });
            let this: Retained<Self> = msg_send_id![super(this), init];

            slider.setTarget(Some(&this));
            slider.setAction(Some(sel!(dragSlider:)));

            this
        }
    }

    fn update_bri_from_slider(&self) {
        let bri = unsafe { self.ivars().slider.integerValue() };
        let path = format!("/lights/{}/state", self.ivars().light_id);
        let json = NSDictionary::from_retained_objects(
            &[
                ns_string!("on"),
                ns_string!("bri"),
                ns_string!("transitiontime"),
            ],
            &[
                Retained::into_super(Retained::into_super(NSNumber::numberWithBool(bri > 0))),
                Retained::into_super(Retained::into_super(NSNumber::numberWithInteger(bri))),
                Retained::into_super(Retained::into_super(NSNumber::numberWithInteger(1))),
            ],
        );
        self.ivars().session.request(
            ns_string!("PUT"),
            &self.ivars().session.authenticated_path(&path),
            Some(&json),
            move |res| match res {
                Ok(_obj) => {}
                Err(err) => {
                    eprintln!("failed setting light: {err}");
                }
            },
        );
    }

    fn queue_update_bri(&self) {
        let interval = 0.050;
        let now = Instant::now();
        if (now - self.ivars().last_updated_bri.get())
            > Duration::from_millis((interval * 1000.0) as u64)
        {
            self.ivars().last_updated_bri.set(now);
            unsafe {
                self.performSelector_withObject_afterDelay_inModes(
                    sel!(setBri:),
                    None,
                    interval,
                    &NSArray::from_slice(&[NSRunLoopCommonModes]),
                )
            };
        }
    }

    pub fn view(&self) -> &NSView {
        &self.ivars().view
    }
}
