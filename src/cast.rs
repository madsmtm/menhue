use objc2::{msg_send, ClassType, Message};

pub trait Downcast {
    fn downcast<New: ClassType>(&self) -> Option<&New>;
}

impl<Obj: Message> Downcast for Obj {
    fn downcast<New: ClassType>(&self) -> Option<&New> {
        if unsafe { msg_send![self, isKindOfClass: New::class()] } {
            // SAFETY: Unsound, but really needed for ergonomics for now
            Some(unsafe { &*(self as *const Self as *const New) })
        } else {
            None
        }
    }
}
