//! Per-view AppKit display cadence. The Objective-C callback only wakes the
//! event loop; it never touches simulation, the renderer or the drawable.

use objc2::{
    AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::AnyObject, sel,
};
use objc2_app_kit::NSApplicationDidChangeScreenParametersNotification;
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol,
    NSProcessInfo, NSProcessInfoThermalState, NSProcessInfoThermalStateDidChangeNotification,
    NSRunLoop, NSRunLoopCommonModes,
};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use std::cell::Cell;
use thrum_render::governor::ThermalState;
use winit::{
    event_loop::EventLoopProxy,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use super::UserEvent;

pub(super) const PREFERRED_REFRESH_HZ: f32 = 120.0;

/// The longest uninterrupted run of surface submissions at the requested display cadence.
#[derive(Default)]
pub(super) struct SubmissionCadence {
    first: Option<f64>,
    previous: Option<(u64, f64)>,
    start: f64,
    longest: (f64, f64),
}

impl SubmissionCadence {
    pub(super) fn presented(&mut self, sequence: u64, target: f64, refresh_hz: f32) {
        let origin = *self.first.get_or_insert(target);
        let period = 1.0 / f64::from(refresh_hz);
        if self
            .previous
            .is_none_or(|(previous_sequence, previous_target)| {
                sequence != previous_sequence + 1
                    || (target - previous_target - period).abs() > 0.000_25
            })
        {
            self.start = target;
        }
        if target - self.start > self.longest.1 - self.longest.0 {
            self.longest = (self.start - origin, target - origin);
        }
        self.previous = Some((sequence, target));
    }

    pub(super) fn longest(&self) -> (f64, f64) {
        self.longest
    }
}

struct LinkIvars {
    wake: EventLoopProxy<UserEvent>,
    callbacks: Cell<u64>,
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
        // SAFETY: NSNotificationCenter sends this selector a notification object.
        #[unsafe(method(screenParametersChanged:))]
        fn screen_parameters_changed(&self, _notification: &NSNotification) {
            let _ = self.ivars().wake.send_event(UserEvent::DisplayParametersChanged);
        }

        // SAFETY: CADisplayLink invokes this selector with one CADisplayLink
        // object. AppKit owns the callback lifetime until invalidate.
        #[unsafe(method(displayFrame:))]
        fn display_frame(&self, link: &CADisplayLink) {
            let target = link.targetTimestamp();
            let number = self.ivars().callbacks.get() + 1;
            self.ivars().callbacks.set(number);
            let _ = self.ivars().wake.send_event(UserEvent::DisplayTarget { target, callback: link.timestamp(), number });
        }
    }
);

impl LinkTarget {
    #[expect(
        unsafe_code,
        reason = "the NSObject initializer is the narrow AppKit callback owner bridge"
    )]
    fn new(wake: EventLoopProxy<UserEvent>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(LinkIvars {
            wake,
            callbacks: Cell::new(0),
        });
        // SAFETY: NSObject's initializer accepts this allocated subclass.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: the observer carries a thread-safe event proxy and has no AppKit state.
    #[unsafe(super = NSObject)]
    #[thread_kind = AnyThread]
    #[ivars = EventLoopProxy<UserEvent>]
    struct ThermalObserver;

    impl ThermalObserver {
        // SAFETY: NSNotificationCenter supplies the notification to this selector.
        #[unsafe(method(thermalStateChanged:))]
        fn thermal_state_changed(&self, _notification: &NSNotification) {
            self.publish();
        }
    }
);

impl ThermalObserver {
    #[expect(
        unsafe_code,
        reason = "Foundation registration and NSObject initialization"
    )]
    fn start(wake: EventLoopProxy<UserEvent>) -> Retained<Self> {
        let observer = Self::alloc().set_ivars(wake);
        // SAFETY: the superclass initializer owns the allocated NSObject.
        let observer: Retained<Self> = unsafe { msg_send![super(observer), init] };
        // SAFETY: the selector accepts one notification and the owner unregisters before drop.
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &observer,
                sel!(thermalStateChanged:),
                Some(NSProcessInfoThermalStateDidChangeNotification),
                None,
            );
        }
        observer.publish();
        observer
    }

    fn publish(&self) {
        let _ = self
            .ivars()
            .send_event(UserEvent::ThermalState(thermal_state()));
    }
}

/// Read the Mac's current ProcessInfo thermal state.
#[must_use]
pub fn thermal_state() -> ThermalState {
    match NSProcessInfo::processInfo().thermalState() {
        NSProcessInfoThermalState::Nominal => ThermalState::Nominal,
        NSProcessInfoThermalState::Fair => ThermalState::Fair,
        NSProcessInfoThermalState::Serious => ThermalState::Serious,
        _ => ThermalState::Critical,
    }
}

/// One display link tied to the actual NSView selected by winit. Retaining the
/// target and link together ensures the callback cannot outlive its event sink.
pub(super) struct ViewDisplayLink {
    link: Retained<CADisplayLink>,
    target: Retained<LinkTarget>,
    observes_display: bool,
    thermal: Retained<ThermalObserver>,
}

impl ViewDisplayLink {
    #[expect(
        unsafe_code,
        reason = "the winit NSView handle and AppKit run-loop method require a narrow checked Objective-C bridge"
    )]
    pub(super) fn start(
        window: &Window,
        wake: EventLoopProxy<UserEvent>,
        observes_display: bool,
    ) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or("display link must start on AppKit main thread")?;
        let handle = window
            .window_handle()
            .map_err(|error| format!("view handle: {error}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("macOS window did not provide an NSView".into());
        };
        let thermal = ThermalObserver::start(wake.clone());
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
            maximum: PREFERRED_REFRESH_HZ,
            preferred: PREFERRED_REFRESH_HZ,
        });
        // SAFETY: creation and removal both occur on AppKit's main thread.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        if observes_display {
            // SAFETY: AppKit posts this notification on the main thread. The
            // retained target implements the selector and unregisters in Drop.
            unsafe {
                NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                    &target,
                    sel!(screenParametersChanged:),
                    Some(NSApplicationDidChangeScreenParametersNotification),
                    None,
                );
            }
        }
        Ok(Self {
            link,
            target,
            observes_display,
            thermal,
        })
    }
}

impl Drop for ViewDisplayLink {
    #[expect(
        unsafe_code,
        reason = "the main-thread display link unregisters its notification target"
    )]
    fn drop(&mut self) {
        self.link.invalidate();
        // SAFETY: this object is the registered thermal observer.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(&self.thermal) };
        if self.observes_display {
            // SAFETY: This is the retained observer registered during start.
            unsafe { NSNotificationCenter::defaultCenter().removeObserver(&self.target) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SubmissionCadence;

    #[test]
    fn missing_submissions_and_display_callbacks_break_the_interval() {
        let mut cadence = SubmissionCadence::default();
        for (sequence, tick) in [(1, 0), (2, 1), (4, 3), (5, 4), (6, 6), (7, 7), (8, 8)] {
            cadence.presented(sequence, f64::from(tick) / 120.0, 120.0);
        }
        assert_eq!(cadence.longest(), (6.0 / 120.0, 8.0 / 120.0));
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
