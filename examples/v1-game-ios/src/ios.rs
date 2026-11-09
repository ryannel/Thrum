//! UIKit shell for the shared V1 game. The main thread forwards display
//! targets, touches and lifecycle; the world lane owns time and rules; the
//! render owner prepares, encodes, acquires late and presents.

use std::{
    sync::{Arc, OnceLock, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use thrum_core::sim::{WorldLaneFault, WorldLifecycle};
use winit::{
    application::ApplicationHandler,
    event::{TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId},
};

use crate::{
    gesture::{Gesture, Phase},
    lease::Take,
    world_policy::WorldPolicy,
};

mod cooked_view;
mod render;
mod witness;

use render::{
    FrameRequest, Job, Lease, RecoverFault, Recovered, RenderWorker, Reply, Reported, Signals,
    StallPoint, Wakes,
};
use witness::Witness;

static SHELL: OnceLock<EventLoopProxy<ShellEvent>> = OnceLock::new();
static WORLD_THREAD: OnceLock<String> = OnceLock::new();
/// Qualification hold inside the render owner's lease section.
const STALL: Duration = Duration::from_secs(8);
/// Frame replies between routine witness writes.
const WITNESS_EVERY: u64 = 120;
/// Recently detached windows whose late touches are still recognised.
const RETIRED_WINDOWS: usize = 4;
/// Failed recovery attempts (not deferrals) before the shell gives up.
const RECOVER_ATTEMPTS: u32 = 3;

#[derive(Clone, Copy, Debug)]
enum ShellEvent {
    Target(f64),
    RenderDone,
    WorldAdvanced,
    /// Qualification only: the render owner is holding a stalled section.
    Stalling,
    /// wgpu reported this device generation lost.
    DeviceLost(u64),
    /// Qualification only: the render owner is holding a recovery.
    RecoverHeld,
    Command(Command),
}

/// Qualification commands from Darwin notifications, honoured only when the
/// app was launched with `THRUM_V1_ROUTE=1`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    ReplaceSurface,
    HideView,
    ShowView,
    StallRender,
    StallPresent,
    SampleActor,
    LoseDevice,
    HoldRecover,
    ReleaseRecover,
    StaleWake,
    CookedPan,
}

#[expect(
    unsafe_code,
    reason = "Swift owns the UIKit display link and calls this C ABI forward"
)]
#[unsafe(no_mangle)]
extern "C" fn thrum_v1_display_target(target_media_seconds: f64) {
    if let Some(shell) = SHELL.get() {
        let _ = shell.send_event(ShellEvent::Target(target_media_seconds));
    }
}

#[expect(
    unsafe_code,
    reason = "Swift forwards qualification notifications through this C ABI"
)]
#[unsafe(no_mangle)]
extern "C" fn thrum_v1_shell_command(code: i32) {
    let command = match code {
        1 => Command::ReplaceSurface,
        2 => Command::HideView,
        3 => Command::ShowView,
        4 => Command::StallRender,
        5 => Command::StallPresent,
        6 => Command::SampleActor,
        7 => Command::LoseDevice,
        8 => Command::HoldRecover,
        9 => Command::ReleaseRecover,
        10 => Command::StaleWake,
        11 => Command::CookedPan,
        _ => return,
    };
    if let Some(shell) = SHELL.get() {
        let _ = shell.send_event(ShellEvent::Command(command));
    }
}

#[expect(
    unsafe_code,
    reason = "Swift attaches Winit's borrowed UIView to the UIKit scene"
)]
unsafe extern "C" {
    fn thrum_v1_attach_view(view: *mut std::ffi::c_void);
    fn thrum_v1_detach_view(view: *mut std::ffi::c_void);
    fn thrum_v1_fit_layer(view: *mut std::ffi::c_void) -> i32;
    fn thrum_v1_live_detached_windows() -> i32;
    fn thrum_v1_start_display_link() -> i32;
    fn thrum_v1_stop_display_link();
}

#[expect(
    unsafe_code,
    reason = "UIKit owns this synchronous start bridge on its main thread"
)]
fn start_display_link() -> Result<(), String> {
    // SAFETY: UIKit owns the link on its main thread, which runs this event loop.
    (unsafe { thrum_v1_start_display_link() } != 0)
        .then_some(())
        .ok_or_else(|| "iOS display link has no connected scene".to_owned())
}

#[expect(
    unsafe_code,
    reason = "UIKit owns this synchronous stop bridge on its main thread"
)]
fn stop_display_link() {
    // SAFETY: Lifecycle callbacks run on UIKit's main thread.
    unsafe { thrum_v1_stop_display_link() };
}

#[expect(
    unsafe_code,
    reason = "the live main-thread Winit window owns this UIView for the call"
)]
fn attach_to_scene(window: &Window) -> Result<(), String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("iOS window handle: {error}"))?;
    let RawWindowHandle::UiKit(view) = handle.as_raw() else {
        return Err("iOS window has no UIKit view".into());
    };
    // SAFETY: Winit owns `window` and its UIView across this synchronous call
    // on UIKit's main thread; Swift does not retain the pointer.
    unsafe { thrum_v1_attach_view(view.ui_view.as_ptr()) };
    Ok(())
}

/// Keep the drawable's aspect while UIKit animates the layer's bounds through a
/// rotation; at rest the drawable matches the bounds and this changes nothing.
/// Returns how many Metal layers were set.
#[expect(
    unsafe_code,
    reason = "the live main-thread Winit window owns this UIView for the call"
)]
fn fit_layer(window: &Window) -> Result<i32, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("iOS window handle: {error}"))?;
    let RawWindowHandle::UiKit(view) = handle.as_raw() else {
        return Err("iOS window has no UIKit view".into());
    };
    // SAFETY: Winit owns `window` and its UIView across this synchronous call
    // on UIKit's main thread; Swift does not retain the pointer.
    Ok(unsafe { thrum_v1_fit_layer(view.ui_view.as_ptr()) })
}

/// Close a window whose surface is gone: hide it, take it out of the scene
/// and remove the `CAMetalLayer` sublayer raw-window-metal added. Winit's iOS
/// window does not close its `UIWindow` on drop.
#[expect(
    unsafe_code,
    reason = "the live main-thread Winit window owns this UIView for the call"
)]
fn detach_window(window: Arc<Window>) {
    if let Ok(handle) = window.window_handle()
        && let RawWindowHandle::UiKit(view) = handle.as_raw()
    {
        // SAFETY: Winit owns `window` and its UIView across this synchronous
        // call on UIKit's main thread; Swift keeps only a weak reference.
        unsafe { thrum_v1_detach_view(view.ui_view.as_ptr()) };
    }
    drop(window);
}

#[expect(
    unsafe_code,
    reason = "Swift counts its weak references on UIKit's main thread"
)]
fn live_detached_windows() -> i32 {
    // SAFETY: Called from the event loop on UIKit's main thread.
    unsafe { thrum_v1_live_detached_windows() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Starting,
    Active,
    Suspended,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent shell flags; lifecycle state is the State enum"
)]
struct Shell {
    route: bool,
    instance: wgpu::Instance,
    lease: Arc<Lease>,
    signals: Arc<Signals>,
    origin: Instant,
    state: State,
    /// The device generation frames are requested under.
    device_generation: u64,
    /// The generation wgpu reported lost; frames wait for its recovery.
    lost_generation: Option<u64>,
    /// A `Recover` job is outstanding. At most one exists.
    recovering: bool,
    stale_request: bool,
    recover_failures: u32,
    /// A new window and surface whose install met the owner inside a section;
    /// retried on the next reply, attached to the scene only once installed.
    pending_install: Option<(Arc<Window>, wgpu::Surface<'static>)>,
    window: Option<Arc<Window>>,
    /// The surface's window is retained until its deferred release completes.
    releasing: Option<Arc<Window>>,
    /// A new window and surface wait for the deferred release.
    restore: bool,
    /// Detached windows whose late touches are routed to the gesture as stale.
    retired: Vec<WindowId>,
    render: Option<RenderWorker>,
    in_flight: bool,
    stall_in_flight: bool,
    pending_target: Option<f64>,
    observed_extent: [u32; 2],
    hidden: bool,
    stall_next: Option<StallPoint>,
    /// The route asked for one committed actor position.
    sample_actor: bool,
    /// The render owner reported that it holds a stalled section.
    stalling: bool,
    controls: Option<v1_game::WorldControls>,
    policy: WorldPolicy,
    gesture: Gesture,
    witness: Witness,
    error: Option<String>,
}

impl Shell {
    fn new() -> Self {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        Self {
            route: std::env::var("THRUM_V1_ROUTE").is_ok_and(|value| value == "1"),
            instance: wgpu::Instance::new(descriptor),
            lease: Arc::new(Lease::new()),
            signals: Arc::new(Signals::default()),
            origin: Instant::now(),
            state: State::Starting,
            device_generation: 1,
            lost_generation: None,
            recovering: false,
            stale_request: false,
            recover_failures: 0,
            pending_install: None,
            window: None,
            releasing: None,
            restore: false,
            retired: Vec::new(),
            render: None,
            in_flight: false,
            stall_in_flight: false,
            pending_target: None,
            observed_extent: [0; 2],
            hidden: false,
            stall_next: None,
            sample_actor: false,
            stalling: false,
            controls: None,
            policy: WorldPolicy::default(),
            gesture: Gesture::new(),
            witness: Witness::new(),
            error: None,
        }
    }

    /// Create a fresh window and surface and install it under a new epoch.
    /// `None` means the owner held the slot for a moment (a section admitted
    /// under the old epoch): the pair waits and is retried on the next reply.
    /// The window joins the scene only once its surface is installed.
    fn attach_surface(&mut self, event_loop: &ActiveEventLoop) -> Result<Option<u64>, String> {
        let (window, surface) = if let Some(pending) = self.pending_install.take() {
            pending
        } else {
            let window = Arc::new(
                event_loop
                    .create_window(Window::default_attributes().with_title("Thrum V1 scene"))
                    .map_err(|error| format!("iOS window: {error}"))?,
            );
            let surface = self
                .instance
                .create_surface(Arc::clone(&window))
                .map_err(|error| format!("iOS surface: {error}"))?;
            if fit_layer(&window)? != 1 {
                return Err("iOS surface did not add one Metal layer".into());
            }
            (window, surface)
        };
        let epoch = match self.lease.try_install(surface) {
            Ok(epoch) => epoch,
            Err(surface) => {
                self.pending_install = Some((window, surface));
                self.witness.install_retries += 1;
                return Ok(None);
            }
        };
        attach_to_scene(&window)?;
        if self.hidden {
            window.set_visible(false);
        }
        let size = layer_extent(&window);
        self.observed_extent = [size.width, size.height];
        self.lease.observe_extent(self.observed_extent);
        self.window = Some(window);
        self.restore = false;
        self.witness.installs += 1;
        Ok(Some(epoch))
    }

    /// Revoke the lease and release the surface without waiting. A section in
    /// progress defers the release, and the detach, to its reply. It drops its
    /// drawable at a later epoch check, or, if the revocation landed after its
    /// final check, presents that one drawable onto the still-attached old
    /// layer (`raced_presents`). No later section is admitted on this surface.
    fn release_surface(&mut self) {
        self.lease.invalidate();
        let window = self.window.take();
        match self.lease.try_take() {
            Take::Taken(surface) => {
                drop(surface);
                self.detach(window);
                self.witness.release_immediate += 1;
            }
            Take::Empty => self.detach(window),
            Take::Busy => {
                // The owner's section still borrows the surface; hide the view
                // now and detach it when the section's reply arrives.
                if let Some(window) = &window {
                    window.set_visible(false);
                }
                self.releasing = window.or(self.releasing.take());
                self.witness.release_deferred += 1;
            }
        }
    }

    fn detach(&mut self, window: Option<Arc<Window>>) {
        if let Some(window) = window {
            if self.retired.len() == RETIRED_WINDOWS {
                self.retired.remove(0);
            }
            self.retired.push(window.id());
            detach_window(window);
            self.witness.detached_windows += 1;
        }
    }

    fn finish_release(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.releasing.is_some() {
            match self.lease.try_take() {
                Take::Taken(surface) => {
                    drop(surface);
                    let window = self.releasing.take();
                    self.detach(window);
                    self.witness.release_completed += 1;
                }
                Take::Empty => {
                    let window = self.releasing.take();
                    self.detach(window);
                }
                Take::Busy => return Ok(()),
            }
        }
        if self.restore && self.releasing.is_none() {
            self.attach_surface(event_loop)?;
        }
        Ok(())
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        // No render owner exists yet, so nothing can hold the slot.
        let epoch = self
            .attach_surface(event_loop)?
            .ok_or("iOS surface slot is held before the render owner started")?;
        let wake = SHELL.get().ok_or("iOS shell proxy missing")?.clone();
        let stalling = wake.clone();
        let lost = wake.clone();
        let held = wake.clone();
        let render = render::start(
            Arc::clone(&self.lease),
            Arc::clone(&self.signals),
            Wakes {
                reply: Box::new(move || {
                    let _ = wake.send_event(ShellEvent::RenderDone);
                }),
                stalling: Box::new(move || {
                    let _ = stalling.send_event(ShellEvent::Stalling);
                }),
                lost: Arc::new(move |generation| {
                    let _ = lost.send_event(ShellEvent::DeviceLost(generation));
                }),
                recover_held: Box::new(move || {
                    let _ = held.send_event(ShellEvent::RecoverHeld);
                }),
            },
        )?;
        let world = SHELL.get().ok_or("iOS shell proxy missing")?.clone();
        self.origin = Instant::now();
        let job = Job::Prepare {
            instance: self.instance.clone(),
            epoch,
            extent: self.observed_extent,
            origin: self.origin,
            world_advanced: Box::new(move || {
                WORLD_THREAD.get_or_init(thread_name);
                let _ = world.send_event(ShellEvent::WorldAdvanced);
            }),
        };
        if render.try_send(job).is_err() {
            return Err("iOS render owner refused preparation".into());
        }
        self.render = Some(render);
        self.in_flight = true;
        self.witness.prepare_requested_on = thread_name();
        Ok(())
    }

    fn suspend(&mut self) -> Result<(), String> {
        if self.state == State::Suspended {
            return Ok(());
        }
        stop_display_link();
        self.signals.background.store(true, Ordering::Release);
        self.pending_target = None;
        self.release_surface();
        self.clear_input()?;
        self.policy.set_background(true);
        self.state = State::Suspended;
        self.witness.suspends += 1;
        self.witness.lifecycle_thread = thread_name();
        self.reconcile_world()?;
        self.write();
        Ok(())
    }

    fn resume(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.state != State::Suspended {
            return Ok(());
        }
        self.signals.background.store(false, Ordering::Release);
        self.restore = true;
        self.finish_release(event_loop)?;
        self.policy.set_background(false);
        self.state = State::Active;
        self.witness.resumes += 1;
        self.reconcile_world()?;
        if self.controls.is_some() {
            start_display_link()?;
        }
        self.write();
        Ok(())
    }

    fn replace_surface(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.state != State::Active {
            return Ok(());
        }
        self.release_surface();
        self.clear_input()?;
        self.restore = true;
        self.finish_release(event_loop)?;
        self.witness.replacements += 1;
        self.write();
        Ok(())
    }

    fn clear_input(&mut self) -> Result<(), String> {
        if let Some(axes) = self.gesture.clear() {
            self.send_axes(axes)?;
        }
        Ok(())
    }

    fn send_axes(&mut self, axes: [f32; 3]) -> Result<(), String> {
        if let Some(controls) = &self.controls {
            controls.set_controls(axes)?;
            self.witness.axes_sent += 1;
            self.witness.last_axes = axes;
            if axes.iter().any(|axis| axis.abs() > 0.0) {
                self.witness.axes_moving += 1;
            }
        }
        Ok(())
    }

    fn touch(&mut self, id: u64, phase: TouchPhase, location: [f64; 2]) -> Result<(), String> {
        if self.route && self.witness.touch_thread.is_empty() {
            // Thread roles are recorded once, and only for the route.
            self.witness.touch_thread = thread_name();
        }
        let phase = match phase {
            TouchPhase::Started => Phase::Started,
            TouchPhase::Moved => Phase::Moved,
            TouchPhase::Ended => Phase::Ended,
            TouchPhase::Cancelled => Phase::Cancelled,
        };
        let scale = self
            .window
            .as_ref()
            .map_or(1.0, |window| window.scale_factor());
        let sent = self.gesture.touch(id, phase, location, scale);
        if let Some(axes) = sent {
            self.send_axes(axes)?;
        }
        // Touch transitions are rare; the route reads them promptly.
        if sent.is_some() || phase != Phase::Moved {
            self.write();
        }
        Ok(())
    }

    fn command(&mut self, event_loop: &ActiveEventLoop, command: Command) -> Result<(), String> {
        self.witness.commands += 1;
        if !self.route {
            self.witness.commands_refused += 1;
            return Ok(());
        }
        match command {
            Command::ReplaceSurface => self.replace_surface(event_loop)?,
            Command::HideView => {
                self.hidden = true;
                if let Some(window) = &self.window {
                    window.set_visible(false);
                }
            }
            Command::ShowView => {
                self.hidden = false;
                if let Some(window) = &self.window {
                    window.set_visible(true);
                }
            }
            Command::StallRender => self.stall_next = Some(StallPoint::AfterAcquire),
            Command::StallPresent => self.stall_next = Some(StallPoint::BeforePresent),
            Command::SampleActor => self.sample_actor = true,
            Command::LoseDevice => self.signals.lose_now.store(true, Ordering::Release),
            Command::HoldRecover => self.signals.hold_recover.store(true, Ordering::Release),
            Command::ReleaseRecover => self.signals.hold_recover.store(false, Ordering::Release),
            Command::StaleWake => {
                // An old-generation wake from the owner, and one request that
                // still carries the old generation: both must be refused.
                self.signals.stale_wake.store(true, Ordering::Release);
                self.stale_request = true;
            }
            Command::CookedPan => self.signals.cooked_pan.store(true, Ordering::Release),
        }
        self.witness.last_command = format!("{command:?}");
        self.write();
        Ok(())
    }

    fn target(&mut self, target: f64) {
        self.witness.display_link_callbacks += 1;
        if self.route && self.witness.display_thread.is_empty() {
            // Thread roles are recorded once, and only for the route.
            self.witness.display_thread = thread_name();
        }
        if self.stall_in_flight {
            self.witness.stall_targets += 1;
        }
        if self.state != State::Active {
            self.witness.targets_ignored += 1;
            return;
        }
        if self.pending_target.replace(target).is_some() {
            self.witness.targets_coalesced += 1;
        }
    }

    /// An extent change revokes the current epoch at once: an in-flight section
    /// drops its old-extent drawable at its next check, and the next request
    /// configures the same surface at the new extent.
    fn resized(&mut self, extent: [u32; 2]) -> bool {
        if extent == self.observed_extent {
            return false;
        }
        self.lease.observe_extent(extent);
        self.lease.invalidate();
        self.observed_extent = extent;
        self.witness.resized(self.lease.epoch());
        true
    }

    fn dispatch(&mut self) -> Result<(), String> {
        if self.in_flight || self.controls.is_none() || self.state != State::Active {
            return Ok(());
        }
        if std::mem::take(&mut self.sample_actor) {
            // An explicit route witness; display callbacks never read the page.
            let render = self.render.as_ref().ok_or("iOS render owner missing")?;
            if render.try_send(Job::SampleActor).is_err() {
                return Err("bounded iOS render owner refused an actor sample".into());
            }
            self.in_flight = true;
            return Ok(());
        }
        if let Some(from) = self.lost_generation {
            // Frames wait while the device is lost; one recovery at a time.
            self.pending_target = None;
            if !self.recovering {
                let render = self.render.as_ref().ok_or("iOS render owner missing")?;
                if render.try_send(Job::Recover { from }).is_err() {
                    return Err("bounded iOS render owner refused a recovery".into());
                }
                self.in_flight = true;
                self.recovering = true;
                self.witness.recover_requests += 1;
                self.write();
            }
            return Ok(());
        }
        let Some(target) = self.pending_target.take() else {
            return Ok(());
        };
        // Backstop for a resize whose event has not been delivered yet.
        if let Some(window) = &self.window {
            let size = layer_extent(window);
            self.resized([size.width, size.height]);
        }
        // A stall request stays until a frame reaches the section and holds it.
        let stall = self.stall_next.map(|point| (point, STALL));
        let request = FrameRequest {
            epoch: self.lease.epoch(),
            generation: if std::mem::take(&mut self.stale_request) {
                self.device_generation.saturating_sub(1)
            } else {
                self.device_generation
            },
            target_media_seconds: target,
            extent: self.observed_extent,
            present: !self.hidden && self.window.is_some(),
            stall,
        };
        let render = self.render.as_ref().ok_or("iOS render owner missing")?;
        match render.try_send(Job::Frame(request)) {
            Ok(()) => {}
            Err(thrum_render::frame::SendFault::Busy(_)) => {
                return Err("bounded iOS render owner refused an idle request".into());
            }
            Err(thrum_render::frame::SendFault::Closed(_)) => {
                return Err("iOS render owner stopped".into());
            }
        }
        self.in_flight = true;
        self.witness.requests += 1;
        self.stall_in_flight = stall.is_some();
        Ok(())
    }

    fn render_done(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let render = self.render.as_ref().ok_or("iOS render owner missing")?;
        let Some(Reported { reply, thread }) = render.try_recv() else {
            if render.is_closed() {
                return Err("iOS render owner stopped before replying".into());
            }
            return Ok(());
        };
        self.in_flight = false;
        match reply {
            Reply::Prepared(prepared) => {
                let prepared = prepared?;
                self.witness.prepare_thread = thread.unwrap_or_default();
                self.witness.adapter = prepared.adapter;
                self.witness.format = format!("{:?}", prepared.format);
                self.controls = Some(prepared.controls);
                self.witness.device_generation = self.device_generation;
                if self.state == State::Starting {
                    self.state = State::Active;
                }
                // A resume during preparation found no controls and left the
                // link stopped; the link starts here for any active state.
                if self.state == State::Active {
                    start_display_link()?;
                }
                self.reconcile_world()?;
            }
            Reply::Recovered(result) => {
                self.recovering = false;
                self.recovered(result)?;
            }
            Reply::Actor(position) => {
                self.witness.actor_samples += 1;
                self.witness.actor_position = position;
                self.write();
            }
            Reply::Frame(frame) => {
                let frame = frame?;
                if let Some(thread) = thread {
                    self.witness.frame_thread = thread;
                }
                self.stall_in_flight = false;
                let stall_ended = frame.stalled;
                if stall_ended {
                    self.stall_next = None;
                    self.stalling = false;
                    self.witness.stalls += 1;
                    self.witness.stall_status = format!("{:?}", frame.status);
                }
                if let Some(report) = frame.report
                    && let Some(refused) = report.overload
                {
                    self.policy.overloaded(refused);
                }
                self.witness.frame(&frame);
                if frame.status == render::SurfaceStatus::DeviceLost
                    && frame.acquired
                    && self.state == State::Active
                {
                    // wgpu cannot discard a drawable of a destroyed device, so
                    // the surface stays "acquired" for any device: replace it
                    // (revoke, detach, and a fresh window and surface below).
                    self.release_surface();
                    self.restore = true;
                    self.witness.loss_replacements += 1;
                }
                self.reconcile_world()?;
                if stall_ended {
                    // World wakes kept updating the tick on UIKit during the stall.
                    self.witness.stall_tick_end = self.witness.world_tick;
                }
                if stall_ended || self.witness.replies.is_multiple_of(WITNESS_EVERY) {
                    self.write();
                }
            }
        }
        self.finish_release(event_loop)?;
        self.dispatch()
    }

    /// wgpu reported a device generation lost. Revoke the lease (bump the
    /// epoch) so no section admitted before the loss presents, keep the world
    /// lane running, and let `dispatch` send the one recovery.
    fn device_lost(&mut self, generation: u64) -> Result<(), String> {
        if generation != self.device_generation || self.lost_generation.is_some() {
            self.witness.stale_loss_wakes += 1;
            self.write();
            return Ok(());
        }
        self.lease.invalidate();
        self.lost_generation = Some(generation);
        self.witness.losses += 1;
        self.witness.lost_generation = generation;
        self.write();
        self.dispatch()
    }

    fn recovered(&mut self, result: Result<Recovered, RecoverFault>) -> Result<(), String> {
        match result {
            Ok(recovered) if self.lost_generation == Some(recovered.from) => {
                self.device_generation = recovered.generation;
                self.lost_generation = None;
                self.recover_failures = 0;
                // The surface is configured afresh for the new device.
                self.lease.invalidate();
                self.witness.recovered(&recovered);
            }
            Ok(_) | Err(RecoverFault::Stale) => self.witness.recover_stale += 1,
            Err(RecoverFault::Deferred) => self.witness.recover_deferred += 1,
            Err(RecoverFault::CandidateLost) => {
                self.witness.recover_candidate_lost += 1;
                self.recover_failures += 1;
            }
            Err(RecoverFault::Failed(error)) => {
                self.witness.recover_failed += 1;
                self.witness.recover_error = error.replace([' ', '\n'], "_");
                self.recover_failures += 1;
            }
        }
        self.write();
        if self.recover_failures >= RECOVER_ATTEMPTS {
            return Err(format!(
                "iOS device recovery failed {} times; the world lane is still running",
                self.recover_failures
            ));
        }
        Ok(())
    }

    /// Consume one lifecycle reply, observe health, and send the next request.
    fn reconcile_world(&mut self) -> Result<(), String> {
        let Some(controls) = self.controls.clone() else {
            return Ok(());
        };
        if self.policy.needs_reply()
            && let Some(reply) = controls
                .poll_lifecycle_reply()
                .map_err(|error| error.to_string())?
        {
            self.policy.acknowledge(reply.request)?;
            self.witness
                .world_reply(reply.request, reply.id.epoch.0, reply.id.tick);
            self.write();
        }
        match controls.check_health() {
            Ok(tick) => self.witness.world_tick = tick,
            Err(WorldLaneFault::Overload(refused)) => self.policy.overloaded(refused),
            Err(error) => return Err(error.to_string()),
        }
        if let Some(request) = self.policy.next_request() {
            match controls.request_lifecycle(request) {
                Ok(()) => {
                    self.policy.accepted(request);
                    if request == WorldLifecycle::Pause {
                        self.witness.pause_requests += 1;
                    }
                }
                Err(WorldLaneFault::Busy) => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }

    /// Rewrite the witness file. Route runs only: an ordinary launch does no
    /// witness I/O on UIKit.
    fn write(&mut self) {
        if !self.route {
            return;
        }
        self.witness.state = format!("{:?}", self.state);
        self.witness.epoch = self.lease.epoch();
        self.witness.world_paused = self.policy.confirmed_paused();
        self.witness.overloads = self.policy.overloads();
        self.witness.refused_ticks = self.policy.refused_ticks();
        self.witness.gesture = self.gesture.counters();
        self.witness.input_epoch = self.gesture.epoch();
        self.witness.stall_active = self.stalling;
        self.witness.world_thread = WORLD_THREAD.get().cloned().unwrap_or_default();
        self.witness.live_detached_windows = live_detached_windows();
        self.witness.write();
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        witness::write_fault(&error);
        self.error = Some(error);
        event_loop.exit();
    }

    /// Time one UIKit callback; a long one would show UIKit waiting.
    /// Callbacks that create or detach windows (lifecycle, commands, or a
    /// frame reply completing a deferred replacement) are timed apart from
    /// per-frame and input callbacks.
    fn timed(
        &mut self,
        event_loop: &ActiveEventLoop,
        kind: &'static str,
        run: impl FnOnce(&mut Self, &ActiveEventLoop) -> Result<(), String>,
    ) {
        let start = Instant::now();
        // A held section or an outstanding recovery: a UIKit callback that
        // waited on the owner would show here.
        let stalled = self.stall_in_flight || self.recovering;
        let windows = (self.witness.installs, self.witness.detached_windows);
        let result = run(self, event_loop);
        let lifecycle = matches!(kind, "resumed" | "suspended" | "command")
            || windows != (self.witness.installs, self.witness.detached_windows);
        self.witness.main_callback(
            start.elapsed(),
            stalled || self.stall_in_flight || self.recovering,
            lifecycle,
            kind,
        );
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }
}

impl ApplicationHandler<ShellEvent> for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.timed(event_loop, "resumed", |shell, event_loop| {
            if shell.render.is_none() {
                shell.start(event_loop)
            } else {
                shell.resume(event_loop)
            }
        });
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.timed(event_loop, "suspended", |shell, _| shell.suspend());
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ShellEvent) {
        let kind = match event {
            ShellEvent::Target(_) => "target",
            ShellEvent::RenderDone => "render",
            ShellEvent::WorldAdvanced => "world",
            ShellEvent::Stalling => "stalling",
            ShellEvent::DeviceLost(_) => "device-lost",
            ShellEvent::RecoverHeld => "recover-held",
            ShellEvent::Command(_) => "command",
        };
        self.timed(event_loop, kind, |shell, event_loop| match event {
            ShellEvent::Target(target) => {
                shell.target(target);
                shell.dispatch()
            }
            ShellEvent::RenderDone => shell.render_done(event_loop),
            ShellEvent::WorldAdvanced => shell.reconcile_world(),
            ShellEvent::Stalling => {
                shell.stalling = true;
                shell.witness.stall_tick_start = shell.witness.world_tick;
                // The route waits for this before revoking the stalled surface.
                shell.write();
                Ok(())
            }
            ShellEvent::DeviceLost(generation) => shell.device_lost(generation),
            ShellEvent::RecoverHeld => {
                shell.witness.recover_held += 1;
                shell.write();
                Ok(())
            }
            ShellEvent::Command(command) => shell.command(event_loop, command),
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            // A touch held across replacement ends on its detached window. Its
            // revoked id makes the gesture count it stale.
            if let WindowEvent::Touch(touch) = event
                && touch.phase != TouchPhase::Started
                && self.retired.contains(&id)
            {
                self.witness.old_window_touches += 1;
                self.timed(event_loop, "touch", |shell, _| {
                    shell.touch(touch.id, touch.phase, [touch.location.x, touch.location.y])
                });
            }
            return;
        }
        self.timed(event_loop, "window", |shell, event_loop| match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                Ok(())
            }
            WindowEvent::Touch(touch) => {
                shell.touch(touch.id, touch.phase, [touch.location.x, touch.location.y])
            }
            WindowEvent::Resized(_) => {
                // winit also queues `Resized` at window creation and layout
                // with sizes that can lag the layer; read the one source.
                let Some(size) = shell.window.as_ref().map(|window| layer_extent(window)) else {
                    return Ok(());
                };
                if shell.resized([size.width, size.height]) {
                    shell.witness.resize_events += 1;
                    shell.write();
                }
                Ok(())
            }
            _ => Ok(()),
        });
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

/// The surface's extent: the view's full bounds in pixels, which the Metal
/// layer tracks and `Resized` reports. winit's iOS `inner_size` is the safe
/// area, which would configure a smaller drawable stretched over the layer.
fn layer_extent(window: &Window) -> winit::dpi::PhysicalSize<u32> {
    window.outer_size()
}

/// The calling thread's role: `main` when the OS reports UIKit's main thread
/// (Rust leaves the iOS main thread unnamed), else the Rust thread name.
#[expect(
    unsafe_code,
    reason = "pthread_main_np reports whether the caller is the process main thread"
)]
fn thread_name() -> String {
    unsafe extern "C" {
        fn pthread_main_np() -> i32;
    }
    // SAFETY: pthread_main_np takes no arguments and only reads thread state.
    if unsafe { pthread_main_np() } != 0 {
        return "main".to_owned();
    }
    thread::current().name().unwrap_or("unnamed").to_owned()
}

pub(super) fn run() {
    let event_loop = EventLoop::<ShellEvent>::with_user_event()
        .build()
        .expect("iOS event loop");
    SHELL
        .set(event_loop.create_proxy())
        .unwrap_or_else(|_| panic!("one iOS shell event loop"));
    let mut shell = Shell::new();
    if let Err(error) = event_loop.run_app(&mut shell) {
        shell.error = Some(format!("iOS event loop: {error}"));
    }
    if let Some(error) = shell.error {
        witness::write_fault(&error);
    }
}
