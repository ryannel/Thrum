//! Per-view AppKit display cadence. The Objective-C callback only wakes the
//! event loop; it never touches simulation, the renderer or the drawable.

use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained, runtime::AnyObject, sel,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes,
};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use winit::{
    event_loop::EventLoopProxy,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use super::UserEvent;

struct LinkIvars {
    wake: EventLoopProxy<UserEvent>,
}

define_class!(
    // SAFETY: NSObject has no subclass invariants; the target has no Drop
    // implementation and is used only on AppKit's main run loop.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = LinkIvars]
    struct LinkTarget;

    // SAFETY: NSObjectProtocol adds no callback signature requirements.
    unsafe impl NSObjectProtocol for LinkTarget {}

    impl LinkTarget {
        // SAFETY: CADisplayLink invokes this selector with one CADisplayLink
        // object. AppKit owns the callback lifetime until invalidate.
        #[unsafe(method(displayFrame:))]
        fn display_frame(&self, link: &CADisplayLink) {
            let target = link.targetTimestamp();
            let _ = self.ivars().wake.send_event(UserEvent::DisplayTarget(target));
        }
    }
);

impl LinkTarget {
    #[expect(
        unsafe_code,
        reason = "the NSObject initializer is the narrow AppKit callback owner bridge"
    )]
    fn new(wake: EventLoopProxy<UserEvent>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(LinkIvars { wake });
        // SAFETY: NSObject's initializer accepts this allocated subclass.
        unsafe { msg_send![super(this), init] }
    }
}

/// One display link tied to the actual NSView selected by winit. Retaining the
/// target and link together ensures the callback cannot outlive its event sink.
pub(super) struct ViewDisplayLink {
    link: Retained<CADisplayLink>,
    _target: Retained<LinkTarget>,
}

impl ViewDisplayLink {
    #[expect(
        unsafe_code,
        reason = "the winit NSView handle and AppKit run-loop method require a narrow checked Objective-C bridge"
    )]
    pub(super) fn start(window: &Window, wake: EventLoopProxy<UserEvent>) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or("display link must start on AppKit main thread")?;
        let handle = window
            .window_handle()
            .map_err(|error| format!("view handle: {error}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("macOS window did not provide an NSView".into());
        };
        let target = LinkTarget::new(wake, mtm);
        // SAFETY: winit owns this NSView for at least the lifetime of `window`.
        // The selector is declared on NSView in the macOS SDK and returns an
        // autoreleased CADisplayLink, which objc2 retains for this owner.
        let view = unsafe { &*handle.ns_view.as_ptr().cast::<AnyObject>() };
        // SAFETY: The selector belongs to the live NSView and the target's
        // method signature accepts one CADisplayLink argument.
        let link: Retained<CADisplayLink> = unsafe {
            msg_send![view, displayLinkWithTarget: &*target, selector: sel!(displayFrame:)]
        };
        link.setPreferredFrameRateRange(CAFrameRateRange {
            minimum: 60.0,
            maximum: 120.0,
            preferred: 120.0,
        });
        // SAFETY: creation and removal both occur on AppKit's main thread.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        Ok(Self {
            link,
            _target: target,
        })
    }
}

impl Drop for ViewDisplayLink {
    fn drop(&mut self) {
        self.link.invalidate();
    }
}

/// Stable OS window number for joining this process to Instruments surface rows.
#[expect(
    unsafe_code,
    reason = "the live winit NSView owns its AppKit window; only its numeric windowNumber is read"
)]
pub(super) fn window_number(window: &Window) -> Result<isize, String> {
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("window has no AppKit view".to_owned());
    };
    // SAFETY: winit retains the NSView and attached NSWindow throughout this call.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<AnyObject>() };
    // SAFETY: NSView's window selector returns its owning live NSWindow.
    let native: Option<Retained<AnyObject>> = unsafe { msg_send![view, window] };
    let native = native.ok_or("view has no AppKit window")?;
    // SAFETY: windowNumber is an NSInteger-returning selector on NSWindow.
    Ok(unsafe { msg_send![&*native, windowNumber] })
}
