//! One winit event-loop owner for the bundled local I1 game.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use thrum_core::{ClockState, HeadlessHost, HostPublication};
#[cfg(target_os = "android")]
use thrum_mobile_shell::AndroidLifecycleSignal;
use thrum_mobile_shell::{LocalReceiptCursor, MobileGate, TouchAdmission};
use winit::{
    application::ApplicationHandler,
    event::{TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::game::{self, COUNTER, Rules};
use crate::render::{ClearRenderer, DrawOutcome};

#[cfg(target_os = "android")]
static ANDROID_LIFECYCLE: AndroidLifecycleSignal = AndroidLifecycleSignal::new();

#[cfg(target_os = "android")]
pub fn android_set_foreground(active: bool) {
    if active {
        ANDROID_LIFECYCLE.on_resume();
    } else if !ANDROID_LIFECYCLE.on_pause() {
        std::process::abort();
    }
}

#[cfg(target_os = "ios")]
pub fn start_ios() {
    let event_loop = EventLoop::new().expect("iOS event loop");
    event_loop
        .run_app(&mut MobileApp::new())
        .expect("iOS app loop");
}

#[cfg(target_os = "android")]
pub fn start_android(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoop::builder()
        .with_android_app(app)
        .build()
        .expect("Android event loop");
    event_loop
        .run_app(&mut MobileApp::new())
        .expect("Android app loop");
}

struct MobileApp {
    gate: MobileGate,
    window: Option<Arc<Window>>,
    renderer: Option<ClearRenderer>,
    host: Option<HeadlessHost<Rules>>,
    receipts: Option<LocalReceiptCursor>,
    view: game::LocalView,
    next_sequence: u64,
    pending_taps: u8,
    displayed_taps: u64,
    presented_color_key: Option<u8>,
    redraw_pending: bool,
    redraw_retry_at: Instant,
    pending_resize: Option<(u32, u32)>,
    queued_world_ticks: u32,
    input_overloaded: bool,
    input_overload_presented: bool,
    world_overloaded: bool,
    world_overload_presented: bool,
    faulted: bool,
}

impl MobileApp {
    const MAX_PENDING_TAPS: usize = 16;

    fn new() -> Self {
        Self {
            gate: MobileGate::new(),
            window: None,
            renderer: None,
            host: None,
            receipts: None,
            view: game::LocalView::default(),
            next_sequence: 1,
            pending_taps: 0,
            displayed_taps: 0,
            presented_color_key: None,
            redraw_pending: false,
            redraw_retry_at: Instant::now(),
            pending_resize: None,
            queued_world_ticks: 0,
            input_overloaded: false,
            input_overload_presented: false,
            world_overloaded: false,
            world_overload_presented: false,
            faulted: false,
        }
    }

    fn reconcile_lifecycle(&mut self) {
        #[cfg(target_os = "android")]
        self.gate.observe_android(ANDROID_LIFECYCLE.snapshot());
        if self.gate.pause_pending() {
            if let Some(host) = &mut self.host {
                host.pause();
            }
            self.gate.mark_host_paused();
        }
        if !self.gate.ready_for_play()
            && let Some(host) = &mut self.host
            && host.clock_state() == ClockState::Active
        {
            host.pause();
            self.gate.mark_host_paused();
        }
    }

    fn admit_queued_taps(&mut self) {
        while self.pending_taps != 0 {
            self.reconcile_lifecycle();
            if !self.gate.accepts_input() || self.faulted || self.world_overloaded {
                break;
            }
            let Some(host) = &mut self.host else { break };
            let observed_tick = host.authority().tick_index();
            match host.submit(game::tap_input(self.next_sequence, observed_tick)) {
                Ok(_) => {
                    self.next_sequence += 1;
                    self.pending_taps -= 1;
                }
                Err(thrum_core::HostAdmissionError::Unavailable)
                | Err(thrum_core::HostAdmissionError::Authority(
                    thrum_core::AdmissionError::Busy
                    | thrum_core::AdmissionError::PublicationCapacity
                    | thrum_core::AdmissionError::ReplicaCapacity,
                )) => {
                    self.input_overloaded = true;
                    self.input_overload_presented = false;
                    break; // Retain this intent and sequence for retry.
                }
                Err(_) => {
                    self.faulted = true;
                    break;
                }
            }
        }
        if self.input_overload_presented
            && self.pending_taps == 0
            && self.gate.active_touches() < Self::MAX_PENDING_TAPS
        {
            self.input_overloaded = false;
            self.input_overload_presented = false;
        }
    }

    fn service_world(&mut self, now: Instant) {
        if self.faulted {
            return;
        }
        self.reconcile_lifecycle();
        let Some(host) = &mut self.host else { return };
        let Some(receipts) = &mut self.receipts else {
            return;
        };
        let callback = match host.on_callback(now) {
            Ok(callback) => callback,
            Err(_) => {
                self.faulted = true;
                return;
            }
        };
        self.queued_world_ticks = callback.clock.queued;
        // Preserve the overload reason across a later lifecycle pause. The
        // clock may then report Paused even though recovery is still owed.
        self.world_overloaded |=
            callback.clock.state == ClockState::Overloaded || callback.clock.step_error.is_some();
        let delivery = receipts.deliver(host, &callback, |publication| {
            // The visible count comes from the replica. Handle outcomes before
            // the local cursor acknowledges their contiguous sequence prefix.
            match publication {
                HostPublication::Replicated { admitted, .. } => {
                    self.view.observe(admitted);
                }
                _ => unreachable!("this app requires its in-memory replica"),
            }
        });
        if delivery.is_err() || callback.terminal_fault.is_some() {
            self.faulted = true;
            return;
        }
        if let Some(state) = host.replica().and_then(|replica| replica.get(COUNTER))
            && state.taps != self.displayed_taps
        {
            self.displayed_taps = state.taps;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        if callback.clock.state != ClockState::Active
            && callback.clock.queued == 0
            && self.gate.ready_for_play()
            && (!self.world_overloaded || self.world_overload_presented)
            && host.resume(now).is_ok()
        {
            self.gate.mark_host_active();
            self.world_overloaded = false;
            self.world_overload_presented = false;
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        if self.faulted {
            return wgpu::Color::BLACK;
        }
        if self.world_overloaded {
            return wgpu::Color {
                r: 0.7,
                g: 0.05,
                b: 0.08,
                a: 1.0,
            };
        }
        if self.input_overloaded {
            return wgpu::Color {
                r: 0.75,
                g: 0.25,
                b: 0.02,
                a: 1.0,
            };
        }
        match self.displayed_taps % 3 {
            0 => wgpu::Color {
                r: 0.03,
                g: 0.13,
                b: 0.32,
                a: 1.0,
            },
            1 => wgpu::Color {
                r: 0.02,
                g: 0.48,
                b: 0.40,
                a: 1.0,
            },
            _ => wgpu::Color {
                r: 0.30,
                g: 0.15,
                b: 0.55,
                a: 1.0,
            },
        }
    }

    fn color_key(&self) -> u8 {
        if self.faulted {
            5
        } else if self.world_overloaded {
            4
        } else if self.input_overloaded {
            3
        } else {
            (self.displayed_taps % 3) as u8
        }
    }
}

impl ApplicationHandler for MobileApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.reconcile_lifecycle();
        if self.window.is_none() {
            let title = format!(
                "Thrum I1 {} ({})",
                env!("CARGO_PKG_VERSION"),
                game::BUNDLED_GAME_ID
            );
            match event_loop.create_window(Window::default_attributes().with_title(title)) {
                Ok(window) => self.window = Some(Arc::new(window)),
                Err(_) => {
                    self.faulted = true;
                    return;
                }
            }
        }
        if let Some(window) = &self.window {
            let prepared = if let Some(renderer) = &mut self.renderer {
                renderer.surface_ready(window.clone())
            } else {
                ClearRenderer::new(window.clone()).map(|renderer| {
                    self.renderer = Some(renderer);
                })
            };
            if prepared.is_err() {
                self.faulted = true;
                return;
            }
            self.pending_resize = None;
            window.request_redraw();
            self.redraw_pending = true;
            self.presented_color_key = None;
            self.redraw_retry_at = Instant::now();
        }
        self.gate.surface_ready();
        #[cfg(target_os = "ios")]
        self.gate.set_ios_foreground(true);
        if self.host.is_none() {
            let (host, receipts) = game::create_host(Instant::now());
            self.host = Some(host);
            self.receipts = Some(receipts);
        }
        self.reconcile_lifecycle();
        if self
            .host
            .as_ref()
            .is_some_and(|host| host.clock_state() == ClockState::Active)
        {
            self.gate.mark_host_active();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.reconcile_lifecycle();
        if !self.gate.surface_lost() {
            self.faulted = true;
        }
        if let Some(host) = &mut self.host {
            host.pause();
        }
        self.gate.mark_host_paused();
        if let Some(renderer) = &mut self.renderer {
            renderer.surface_lost();
        }
        self.presented_color_key = None;
        self.redraw_pending = false;
        self.window = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.reconcile_lifecycle();
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => self.gate.set_focus(focused),
            WindowEvent::Resized(size) => {
                if self.gate.ready_for_play() {
                    if let Some(renderer) = &mut self.renderer
                        && renderer.resize(size.width, size.height).is_err()
                    {
                        self.faulted = true;
                    }
                    self.presented_color_key = None;
                } else {
                    self.pending_resize = Some((size.width, size.height));
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw_pending = false;
                if !self.gate.ready_for_play() {
                    return;
                }
                if let Some((width, height)) = self.pending_resize.take()
                    && let Some(renderer) = &mut self.renderer
                    && renderer.resize(width, height).is_err()
                {
                    self.faulted = true;
                    return;
                }
                let color_key = self.color_key();
                let color = self.clear_color();
                if let Some(renderer) = &mut self.renderer {
                    match renderer.draw(color) {
                        Ok(DrawOutcome::Presented) => {
                            self.presented_color_key = Some(color_key);
                            self.redraw_retry_at = Instant::now();
                            if color_key == 4 {
                                self.world_overload_presented = true;
                            } else if color_key == 3 {
                                self.input_overload_presented = true;
                            }
                        }
                        Ok(DrawOutcome::Deferred) => {
                            self.presented_color_key = None;
                            self.redraw_retry_at = Instant::now() + Duration::from_millis(50);
                        }
                        Err(_) => {
                            if let Some(window) = &self.window
                                && renderer.surface_ready(window.clone()).is_ok()
                            {
                                self.presented_color_key = None;
                                self.redraw_retry_at = Instant::now() + Duration::from_millis(50);
                            } else {
                                self.faulted = true;
                            }
                        }
                    }
                }
            }
            #[cfg(target_os = "ios")]
            WindowEvent::Occluded(occluded) => self.gate.set_ios_foreground(!occluded),
            WindowEvent::Touch(touch) => match touch.phase {
                TouchPhase::Started => {
                    if self.pending_taps as usize + self.gate.active_touches()
                        >= Self::MAX_PENDING_TAPS
                        || self.gate.touch_started(touch.id) == TouchAdmission::Capacity
                    {
                        self.input_overloaded = true;
                        self.input_overload_presented = false;
                    }
                }
                TouchPhase::Ended => {
                    if self.gate.touch_ended(touch.id) {
                        self.pending_taps += 1; // Start reserved this finite slot.
                        self.admit_queued_taps();
                    }
                }
                TouchPhase::Cancelled => self.gate.touch_cancelled(touch.id),
                TouchPhase::Moved => {}
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.service_world(now);
        self.admit_queued_taps();
        let color_key = self.color_key();
        if self.presented_color_key != Some(color_key)
            && !self.redraw_pending
            && self.gate.ready_for_play()
            && now >= self.redraw_retry_at
            && let Some(window) = &self.window
        {
            window.request_redraw();
            self.redraw_pending = true;
        }
        if self.host.is_some()
            && !self.faulted
            && (self.gate.ready_for_play() || self.queued_world_ticks != 0)
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(8)));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
