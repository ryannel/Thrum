//! AppKit notifications for the winit window, with winit retaining its delegate.

use super::output_scale::ModeEvent;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained, sel};
use objc2_app_kit::{
    NSWindow, NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
    NSWindowStyleMask, NSWindowWillEnterFullScreenNotification,
    NSWindowWillExitFullScreenNotification,
};
use objc2_foundation::{MainThreadMarker, NSNotification, NSNotificationCenter, NSObject};
use objc2_quartz_core::CACurrentMediaTime;
use std::cell::RefCell;
use winit::window::Window;

define_class!(
    // SAFETY: NSObject has no subclass invariants. AppKit posts these events on the main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<Vec<(ModeEvent, f64)>>]
    struct ModeObserver;

    impl ModeObserver {
        // SAFETY: NSNotificationCenter supplies one NSNotification to this selector.
        #[unsafe(method(windowModeChanged:))]
        #[expect(unsafe_code, reason = "AppKit exports static notification names")]
        fn window_mode_changed(&self, notification: &NSNotification) {
            let name = notification.name();
            // SAFETY: These are immutable AppKit notification names.
            let event = unsafe {
                if &*name == NSWindowWillEnterFullScreenNotification {
                    ModeEvent::Start(true)
                } else if &*name == NSWindowWillExitFullScreenNotification {
                    ModeEvent::Start(false)
                } else {
                    ModeEvent::Complete(&*name == NSWindowDidEnterFullScreenNotification)
                }
            };
            self.ivars().borrow_mut().push((event, CACurrentMediaTime()));
        }
    }
);

pub(super) struct WindowMode {
    observer: Retained<ModeObserver>,
    window: Retained<NSWindow>,
}

impl WindowMode {
    #[expect(
        unsafe_code,
        reason = "NSNotificationCenter selector registration and NSObject initialization"
    )]
    pub(super) fn attach(window: &Window) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or("window mode requires the AppKit main thread")?;
        let window = super::display::window_view(window)?
            .window()
            .ok_or("AppKit view has no window")?;
        let observer = ModeObserver::alloc(mtm).set_ivars(RefCell::default());
        // SAFETY: NSObject initializes this subclass, which implements the single notification
        // selector. The observer is retained until it unregisters on the main thread.
        let observer: Retained<ModeObserver> = unsafe { msg_send![super(observer), init] };
        // SAFETY: The registered selector takes one notification. Drop unregisters this observer.
        unsafe {
            for name in [
                NSWindowWillEnterFullScreenNotification,
                NSWindowDidEnterFullScreenNotification,
                NSWindowWillExitFullScreenNotification,
                NSWindowDidExitFullScreenNotification,
            ] {
                NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                    &observer,
                    sel!(windowModeChanged:),
                    Some(name),
                    Some(&window),
                );
            }
        }
        Ok(Self { observer, window })
    }

    pub(super) fn has_pending(&self) -> bool {
        !self.observer.ivars().borrow().is_empty()
    }

    pub(super) fn take(&self) -> Vec<(ModeEvent, f64)> {
        std::mem::take(&mut *self.observer.ivars().borrow_mut())
    }

    pub(super) fn fullscreen(&self) -> bool {
        self.window
            .styleMask()
            .contains(NSWindowStyleMask::FullScreen)
    }

    pub(super) fn toggle(&self) {
        self.window.toggleFullScreen(None);
    }
}

impl Drop for WindowMode {
    #[expect(
        unsafe_code,
        reason = "the retained observer unregisters before it is dropped"
    )]
    fn drop(&mut self) {
        // SAFETY: This main-thread observer is still alive and was registered in attach.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(&self.observer) };
    }
}
