//! UIKit shell for the shared V1 game. The main thread forwards display
//! targets, touches and lifecycle; the world lane owns time and rules; the
//! render owner prepares, encodes, acquires late and presents.

use std::{
    sync::{Arc, OnceLock},
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

mod render;
mod witness;

use render::{FrameRequest, Job, Lease, RenderWorker, Reply, Reported};
use witness::Witness;

static SHELL: OnceLock<EventLoopProxy<ShellEvent>> = OnceLock::new();
static WORLD_THREAD: OnceLock<String> = OnceLock::new();
/// Qualification hold inside the render owner's lease section.
const STALL: Duration = Duration::from_secs(5);
/// Frame replies between routine witness writes.
const WITNESS_EVERY: u64 = 120;
/// Recently detached windows whose late touches are still recognised.
const RETIRED_WINDOWS: usize = 4;

#[derive(Clone, Copy, Debug)]
enum ShellEvent {
    Target(f64),
    RenderDone,
    WorldAdvanced,
    /// Qualification only: the render owner is holding a stalled section.
    Stalling,
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
    origin: Instant,
    state: State,
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
    stall_next: bool,
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
            origin: Instant::now(),
            state: State::Starting,
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
            stall_next: false,
            stalling: false,
            controls: None,
            policy: WorldPolicy::default(),
            gesture: Gesture::new(),
            witness: Witness::new(),
            error: None,
        }
    }

    /// Create a fresh window and surface and install it under a new epoch.
    fn attach_surface(&mut self, event_loop: &ActiveEventLoop) -> Result<u64, String> {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Thrum V1 scene"))
                .map_err(|error| format!("iOS window: {error}"))?,
        );
        attach_to_scene(&window)?;
        let surface = self
            .instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("iOS surface: {error}"))?;
        let epoch = self
            .lease
            .try_install(surface)
            .map_err(|_| "iOS surface slot is still leased".to_owned())?;
        let size = window.inner_size();
        self.observed_extent = [size.width, size.height];
        self.window = Some(window);
        self.restore = false;
        self.witness.installs += 1;
        Ok(epoch)
    }

    /// Revoke the lease and release the surface without waiting. A section in
    /// progress defers the release to its reply; its drawable cannot present.
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
        let epoch = self.attach_surface(event_loop)?;
        let wake = SHELL.get().ok_or("iOS shell proxy missing")?.clone();
        let stalling = wake.clone();
        let render = render::start(
            Arc::clone(&self.lease),
            move || {
                let _ = wake.send_event(ShellEvent::RenderDone);
            },
            move || {
                let _ = stalling.send_event(ShellEvent::Stalling);
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
        self.witness.touch_thread = thread_name();
        let phase = match phase {
            TouchPhase::Started => Phase::Started,
            TouchPhase::Moved => Phase::Moved,
            TouchPhase::Ended => Phase::Ended,
            TouchPhase::Cancelled => Phase::Cancelled,
        };
        let scale = self.window.as_ref().map_or(1.0, |window| window.scale_factor());
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
            Command::StallRender => self.stall_next = true,
        }
        self.witness.last_command = format!("{command:?}");
        self.write();
        Ok(())
    }

    fn target(&mut self, target: f64) {
        self.witness.display_link_callbacks += 1;
        self.witness.display_thread = thread_name();
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

    fn dispatch(&mut self) -> Result<(), String> {
        if self.in_flight || self.controls.is_none() || self.state != State::Active {
            return Ok(());
        }
        let Some(target) = self.pending_target.take() else {
            return Ok(());
        };
        if let Some(window) = &self.window {
            let size = window.inner_size();
            let extent = [size.width, size.height];
            if extent != self.observed_extent {
                self.lease.invalidate();
                self.observed_extent = extent;
                self.witness.resizes += 1;
            }
        }
        // A stall request stays until a frame reaches the section and holds it.
        let stall = self.stall_next.then_some(STALL);
        let request = FrameRequest {
            epoch: self.lease.epoch(),
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
                if self.state == State::Starting {
                    self.state = State::Active;
                    start_display_link()?;
                }
                self.reconcile_world()?;
            }
            Reply::Frame(frame) => {
                let frame = frame?;
                self.witness.frame_thread = thread.unwrap_or_default();
                self.stall_in_flight = false;
                let stall_ended = frame.stalled;
                if stall_ended {
                    self.stall_next = false;
                    self.stalling = false;
                    self.witness.stalls += 1;
                    self.witness.stall_status = format!("{:?}", frame.status);
                }
                if let Some(report) = frame.report
                    && let Some(refused) = report.overload
                {
                    self.policy.overloaded(refused);
                }
                self.witness.frame(&frame, self.lease.epoch());
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
            self.witness.world_reply(reply.request, reply.id.epoch.0, reply.id.tick);
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

    fn write(&mut self) {
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
        let stalled = self.stall_in_flight;
        let windows = (self.witness.installs, self.witness.detached_windows);
        let result = run(self, event_loop);
        let lifecycle = matches!(kind, "resumed" | "suspended" | "command")
            || windows != (self.witness.installs, self.witness.detached_windows);
        self.witness.main_callback(
            start.elapsed(),
            stalled || self.stall_in_flight,
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
            WindowEvent::Touch(touch) => shell.touch(
                touch.id,
                touch.phase,
                [touch.location.x, touch.location.y],
            ),
            _ => Ok(()),
        });
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
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
