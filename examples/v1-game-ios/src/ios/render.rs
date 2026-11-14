//! The render owner: device preparation, scene encoding, late acquisition and
//! presentation on one resident thread behind the engine's bounded handoff.

use std::{
    fmt::Write as _,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use thrum_render::{
    capabilities::DeviceCapabilities,
    frame::{FrameWorker, TargetLeadStatus, display_target_lead},
    present::SurfaceProfile,
    runtime::{DriveReport, FrameError},
};
use v1_game::{
    EncodedFrame, FrameConfig, FrameHost, SurfaceOutput, ThreadedFrameHost, WorldControls,
};

/// The platform's first device generation; it matches the host's own.
const FIRST_GENERATION: u64 = FrameHost::DEVICE_GENERATION;
/// Longest qualification hold of a recovery before it proceeds anyway.
const RECOVER_HOLD_MAX: Duration = Duration::from_secs(300);
/// Sleep slice inside qualification holds, so a loss can land inside them.
const HOLD_SLICE: Duration = Duration::from_millis(20);
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};

use crate::lease::{Leased, SurfaceLease};

pub(super) const THREAD: &str = "thrum-v1-ios-render";
/// Completed samples polled per job; the host retains at most this many frames.
const SAMPLE_SLOTS: usize = 2;

pub(super) type Lease = SurfaceLease<wgpu::Surface<'static>>;
pub(super) type RenderWorker = FrameWorker<Job, Reported>;

pub(super) enum Job {
    Prepare {
        instance: wgpu::Instance,
        epoch: u64,
        extent: [u32; 2],
        origin: Instant,
        world_advanced: Box<dyn Fn() + Send>,
    },
    Frame(FrameRequest),
    /// Rebuild on a fresh device after generation `from` was lost.
    Recover {
        from: u64,
    },
    /// Route witness only: read the controlled actor's committed position
    /// from the world lane's latest published page.
    SampleActor,
}

/// Where a qualification stall holds the lease section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StallPoint {
    /// After acquiring, before the epoch checks: revocation drops the drawable.
    AfterAcquire,
    /// After the last epoch check, before `present`: the admitted present
    /// completes after a revocation that lands here.
    BeforePresent,
}

/// Jobs cross to the render owner's thread.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Job>();
};

pub(super) struct FrameRequest {
    pub(super) epoch: u64,
    /// The device generation the shell believes current.
    pub(super) generation: u64,
    pub(super) target_media_seconds: f64,
    pub(super) extent: [u32; 2],
    /// The shell permits drawable acquisition (its view is shown).
    pub(super) present: bool,
    /// Qualification only: hold an acquired drawable inside the lease section.
    pub(super) stall: Option<(StallPoint, Duration)>,
}

pub(super) struct Prepared {
    pub(super) controls: WorldControls,
    pub(super) adapter: String,
    pub(super) format: wgpu::TextureFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SurfaceStatus {
    /// Presented under the request's epoch.
    Presented,
    /// The world advanced but no view could use a drawable.
    NoView,
    /// The shell withheld acquisition (hidden view).
    Denied,
    /// No lease, or the epoch changed before acquisition.
    NoLease,
    /// wgpu returned no drawable (timeout or occluded).
    Missing,
    /// The surface needs configuration; pending frames delay it.
    Reconfigure,
    /// A drawable was acquired, but the epoch changed before presentation.
    StaleDropped,
    /// The device is lost: no call was made on it, nothing was presented.
    DeviceLost,
    /// The request named an older device generation; it was refused unrun.
    StaleGeneration,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent per-frame witnesses, not a state machine"
)]
pub(super) struct FrameReply {
    pub(super) epoch: u64,
    /// The owner's device generation when it replied.
    pub(super) generation: u64,
    /// The request's surface extent, which a presented drawable has.
    pub(super) extent: [u32; 2],
    pub(super) frame_extent: Option<thrum_render::runtime::FrameExtent>,
    pub(super) drawable: [u32; 2],
    pub(super) status: SurfaceStatus,
    pub(super) report: Option<DriveReport>,
    pub(super) cooked: String,
    pub(super) completed: u32,
    pub(super) surface_samples: u32,
    pub(super) pending_frames: usize,
    pub(super) target: TargetLeadStatus,
    pub(super) acquired: bool,
    pub(super) presentable_encode: bool,
    pub(super) configured: bool,
    pub(super) stalled: bool,
    /// Wall time inside `get_current_texture`, which Metal may block.
    pub(super) acquire: Duration,
    /// Presented, then found revoked: the present was ordered before the
    /// shell's revocation.
    pub(super) raced_present: bool,
    /// Dropped before presenting: the drawable's size was not the extent the
    /// shell last observed.
    pub(super) extent_dropped: bool,
    /// Presented, then the shell's observed extent differed from the drawable.
    pub(super) old_extent_present: bool,
    /// A recorded frame was skipped on a healthy device: the engine flushed
    /// its staged writes with an empty queue submit.
    pub(super) empty_submit: bool,
    /// A recorded frame was abandoned after loss, with no queue call.
    pub(super) abandoned: bool,
}

/// A published replacement device.
pub(super) struct Recovered {
    pub(super) from: u64,
    pub(super) generation: u64,
    pub(super) extent: [u32; 2],
    /// The candidate was fitted to an extent that changed during preparation.
    pub(super) refitted: bool,
    /// Owner wall time from the job to publication.
    pub(super) wall: Duration,
}

/// Why a recovery did not publish. The world keeps running in every case.
#[derive(Debug)]
pub(super) enum RecoverFault {
    /// `from` is not the owner's current lost generation.
    Stale,
    /// The app is in the background or holds no lease; retry when active.
    Deferred,
    /// The candidate device was lost during preparation.
    CandidateLost,
    Failed(String),
}

#[expect(
    clippy::large_enum_variant,
    reason = "the bounded single reply slot retains frame diagnostics inline without per-frame heap allocation"
)]
pub(super) enum Reply {
    Prepared(Result<Prepared, String>),
    Frame(Result<FrameReply, String>),
    Recovered(Result<Recovered, RecoverFault>),
    Actor(Option<[f64; 3]>),
}

pub(super) struct Reported {
    pub(super) reply: Reply,
    /// Thread that produced the reply.
    pub(super) thread: Option<String>,
}

/// Flags the shell sets and the owner reads; none orders frame admission.
#[derive(Default)]
pub(super) struct Signals {
    pub(super) capture: AtomicU64,
    pub(super) capture_callbacks: super::capture::Callbacks,
    /// Explicit held-view qualification only: begin the cooked camera pan.
    pub(super) cooked_pan: AtomicBool,
    /// Route only: destroy the device at the owner's next opportunity.
    pub(super) lose_now: AtomicBool,
    /// Route only: hold the next recovery after preparation, before the
    /// refit check and publication.
    pub(super) hold_recover: AtomicBool,
    /// Route only: wake the shell once with the previous generation's loss,
    /// through the same path wgpu's lost callback uses.
    pub(super) stale_wake: AtomicBool,
    /// The app is in the background: no recovery may prepare or publish.
    pub(super) background: AtomicBool,
}

/// One device generation's loss watch.
struct Watch {
    generation: u64,
    lost: Arc<AtomicBool>,
    /// The owner called `destroy` and polls until wgpu reports the loss.
    destroyed: bool,
    /// The first uncaptured wgpu error on this device, if any.
    error: Arc<Mutex<Option<String>>>,
}

impl Watch {
    /// Register before the device is used: wgpu's lost callback sets the flag
    /// and wakes the shell with this generation.
    fn register(device: &wgpu::Device, generation: u64, lost_wake: &LostWake) -> Self {
        let lost = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&lost);
        let wake = Arc::clone(lost_wake);
        device.set_device_lost_callback(move |_, _| {
            signal.store(true, Ordering::Release);
            wake(generation);
        });
        let error = Arc::new(Mutex::new(None));
        let first = Arc::clone(&error);
        device.on_uncaptured_error(Arc::new(move |fault: wgpu::Error| {
            let mut slot = first.lock().unwrap_or_else(PoisonError::into_inner);
            slot.get_or_insert_with(|| fault.to_string());
        }));
        Self {
            generation,
            lost,
            destroyed: false,
            error,
        }
    }

    fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    fn take_error(&self) -> Option<String> {
        self.error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

type LostWake = Arc<dyn Fn(u64) + Send + Sync>;

struct Gpu {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    watch: Watch,
    host: ThreadedFrameHost,
    cooked: Option<Arc<Mutex<thrum_asset_gpu::CookedStats>>>,
    cooked_view: Option<super::cooked_view::View>,
    format: wgpu::TextureFormat,
    host_extent: [u32; 2],
}

struct Owner {
    capture: super::capture::Recorder,
    lease: Arc<Lease>,
    /// The instance the first device came from; recoveries use it again.
    instance: Option<wgpu::Instance>,
    signals: Arc<Signals>,
    gpu: Option<Gpu>,
    diagnostics: bool,
    origin: Instant,
    /// The next candidate generation; strictly increasing across attempts.
    next_generation: u64,
    lost_wake: LostWake,
    /// Qualification only: tells the shell the owner now holds the section.
    stalling: Box<dyn Fn() + Send>,
    /// Qualification only: tells the shell a recovery is being held.
    recover_held: Box<dyn Fn() + Send>,
}

/// Callbacks from the owner (and wgpu's loss callback) to the shell.
pub(super) struct Wakes {
    pub(super) reply: Box<dyn FnMut() + Send>,
    pub(super) stalling: Box<dyn Fn() + Send>,
    pub(super) lost: LostWake,
    pub(super) recover_held: Box<dyn Fn() + Send>,
}

pub(super) fn start(
    lease: Arc<Lease>,
    signals: Arc<Signals>,
    wakes: Wakes,
) -> Result<RenderWorker, String> {
    let Wakes {
        reply: wake,
        stalling,
        lost,
        recover_held,
    } = wakes;
    let mut owner = Owner {
        capture: super::capture::Recorder::default(),
        lease,
        instance: None,
        signals,
        gpu: None,
        diagnostics: std::env::var("THRUM_V1_ROUTE").is_ok_and(|value| value == "1"),
        origin: Instant::now(),
        next_generation: FIRST_GENERATION + 1,
        lost_wake: lost,
        stalling,
        recover_held,
    };
    // The thread is named on preparation and the first frame reply only, so
    // frames add no allocation for the witness.
    let mut frame_named = false;
    FrameWorker::start(
        THREAD,
        move |job| {
            let frame = matches!(job, Job::Frame(_));
            let name = !frame || !std::mem::replace(&mut frame_named, true);
            Reported {
                reply: owner.work(job),
                thread: name
                    .then(|| thread::current().name().map(str::to_owned))
                    .flatten(),
            }
        },
        wake,
    )
    .map_err(|error| format!("iOS render owner: {error}"))
}

/// The platform's device recipe, shared by preparation and recovery. wgpu asks
/// for a fresh adapter for each device.
fn request_device(
    instance: &wgpu::Instance,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), String> {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(|error| format!("iOS adapter: {error:?}"))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: DeviceCapabilities::requested_timing_features(&adapter),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))
    .map_err(|error| format!("iOS device: {error:?}"))?;
    Ok((adapter, device, queue))
}

impl FrameReply {
    fn native_witness(&mut self, host: &ThreadedFrameHost) {
        if let Some(native) = host.native_witness_report() {
            let _ = write!(
                self.cooked,
                "\nnative_selection={:?}\nnative_encoded={}\nnative_submitted={}\nnative_discarded={}\nnative_resets={}\nnative_completed={}\n",
                native.selection,
                native.encoded,
                native.submitted,
                native.discarded,
                native.resets,
                native.completed
            );
        }
    }

    fn new(request: &FrameRequest, generation: u64, target: TargetLeadStatus) -> Self {
        Self {
            epoch: request.epoch,
            generation,
            extent: request.extent,
            frame_extent: None,
            drawable: [0; 2],
            status: SurfaceStatus::NoView,
            report: None,
            cooked: String::new(),
            completed: 0,
            surface_samples: 0,
            pending_frames: 0,
            target,
            acquired: false,
            presentable_encode: false,
            configured: false,
            stalled: false,
            acquire: Duration::ZERO,
            raced_present: false,
            extent_dropped: false,
            old_extent_present: false,
            empty_submit: false,
            abandoned: false,
        }
    }
}

impl Owner {
    fn work(&mut self, job: Job) -> Reply {
        match job {
            Job::Prepare {
                instance,
                epoch,
                extent,
                origin,
                world_advanced,
            } => {
                self.origin = origin;
                Reply::Prepared(self.prepare(&instance, epoch, extent, world_advanced))
            }
            Job::Frame(request) => Reply::Frame(self.frame(&request)),
            Job::Recover { from } => Reply::Recovered(self.recover(from)),
            Job::SampleActor => Reply::Actor(
                self.gpu
                    .as_ref()
                    .and_then(|gpu| gpu.host.controlled_position()),
            ),
        }
    }

    fn prepare(
        &mut self,
        instance: &wgpu::Instance,
        epoch: u64,
        extent: [u32; 2],
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared, String> {
        if self.gpu.is_some() {
            return Err("iOS render owner is already prepared".into());
        }
        self.instance = Some(instance.clone());
        let (adapter, device, queue) = request_device(instance)?;
        let watch = Watch::register(&device, FIRST_GENERATION, &self.lost_wake);
        let capabilities = DeviceCapabilities::capture(&adapter, &device, FIRST_GENERATION);
        // Every CAMetalLayer accepts Bgra8Unorm; prefer it as before, and check
        // the live surface's list when the lease is still current.
        let format = self
            .lease
            .section(epoch, |leased| {
                let formats = leased.surface.get_capabilities(&adapter).formats;
                [
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureFormat::Bgra8UnormSrgb,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                ]
                .into_iter()
                .find(|format| formats.contains(format))
            })
            .unwrap_or(Some(wgpu::TextureFormat::Bgra8Unorm))
            .ok_or("no supported iOS SDR surface format")?;
        let extent = [extent[0].max(1), extent[1].max(1)];
        let setup = v1_game::selected_setup()?;
        let config = FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile { format, extent }),
        };
        let mut host = pollster::block_on(FrameHost::new_with_setup(
            &device,
            &queue,
            &capabilities,
            config,
            setup,
        ))?;
        if self.diagnostics && std::env::var("THRUM_CULLING_ORACLE").as_deref() == Ok("1") {
            super::culling::record(&mut host, &device, &queue, &capabilities, config, setup)?;
        }
        let cooked = v1_game::attach_cooked_scene(&mut host)?;
        let cooked_view = super::cooked_view::View::prepare(&mut host)?;
        let (host, controls) = host.into_threaded(self.origin, world_advanced)?;
        let name = format!(
            "{:?}; downlevel={:?}; recipe={:?}; registry={}; unified_memory={:?}",
            capabilities.adapter(),
            capabilities.downlevel(),
            capabilities.recipe(),
            capabilities.registry_digest(),
            capabilities.unified_memory()
        );
        self.gpu = Some(Gpu {
            adapter,
            device,
            queue,
            watch,
            host,
            cooked,
            cooked_view,
            format,
            host_extent: extent,
        });
        Ok(Prepared {
            controls,
            adapter: name,
            format,
        })
    }

    /// One frame job. From the moment the owner knows its device is lost (it
    /// destroyed it, or wgpu's callback fired) it makes no call on it except
    /// the non-blocking poll that delivers wgpu's lost callback.
    fn frame(&mut self, request: &FrameRequest) -> Result<FrameReply, String> {
        let unrun = |generation, status| {
            let lead = display_target_lead(request.target_media_seconds, media_time_now());
            let mut reply = FrameReply::new(request, generation, lead.status);
            reply.status = status;
            reply
        };
        {
            let gpu = self
                .gpu
                .as_mut()
                .ok_or("iOS render owner is not prepared")?;
            if self.signals.background.load(Ordering::Acquire) {
                if let Err(error) =
                    self.capture
                        .cancel(&mut gpu.host, &self.signals, "app backgrounded")
                {
                    self.capture.failed(&mut gpu.host, &self.signals, &error);
                }
                let mut reply = unrun(gpu.watch.generation, SurfaceStatus::NoLease);
                reply.native_witness(&gpu.host);
                return Ok(reply);
            }
            if self.signals.stale_wake.swap(false, Ordering::AcqRel)
                && gpu.watch.generation > FIRST_GENERATION
            {
                (self.lost_wake)(gpu.watch.generation - 1);
            }
            if self.signals.lose_now.swap(false, Ordering::AcqRel) && !gpu.watch.destroyed {
                gpu.device.destroy();
                gpu.watch.destroyed = true;
            }
            if gpu.watch.destroyed && !gpu.watch.is_lost() {
                // `destroy` only marks the device; wgpu reports the loss from
                // maintenance. Never `Wait` on it.
                let _ = gpu.device.poll(wgpu::PollType::Poll);
            }
            if gpu.watch.destroyed || gpu.watch.is_lost() {
                if let Err(error) =
                    self.capture
                        .cancel(&mut gpu.host, &self.signals, "device generation lost")
                {
                    self.capture.failed(&mut gpu.host, &self.signals, &error);
                }
                return Ok(unrun(gpu.watch.generation, SurfaceStatus::DeviceLost));
            }
            if request.generation != gpu.watch.generation {
                return Ok(unrun(gpu.watch.generation, SurfaceStatus::StaleGeneration));
            }
        }
        let result = self.frame_on_device(request);
        let gpu = self
            .gpu
            .as_mut()
            .ok_or("iOS render owner is not prepared")?;
        if gpu.watch.destroyed && !gpu.watch.is_lost() {
            let _ = gpu.device.poll(wgpu::PollType::Poll);
        }
        let lost = gpu.watch.destroyed || gpu.watch.is_lost();
        match result {
            Ok(reply) if !lost => match gpu.watch.take_error() {
                Some(error) => Err(format!("iOS wgpu error: {error}")),
                None => Ok(reply),
            },
            Ok(reply) => Ok(reply),
            Err(_) if lost => {
                // The error may follow an acquisition: assume a drawable was
                // held so the shell replaces the surface (walk step 6).
                let mut reply = unrun(gpu.watch.generation, SurfaceStatus::DeviceLost);
                reply.acquired = true;
                Ok(reply)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Rebuild on a fresh device after generation `from` was lost, through the
    /// engine's detached reconstruction path. The world lane never stops.
    fn recover(&mut self, from: u64) -> Result<Recovered, RecoverFault> {
        let started = Instant::now();
        let instance = self
            .instance
            .clone()
            .ok_or_else(|| RecoverFault::Failed("iOS render owner is not prepared".into()))?;
        {
            let gpu = self
                .gpu
                .as_ref()
                .ok_or_else(|| RecoverFault::Failed("iOS render owner is not prepared".into()))?;
            if gpu.watch.generation != from || !gpu.watch.is_lost() {
                return Err(RecoverFault::Stale);
            }
        }
        // iOS forbids GPU work in the background: prepare and publish only
        // while active.
        let background = || self.signals.background.load(Ordering::Acquire);
        if background() {
            return Err(RecoverFault::Deferred);
        }
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| RecoverFault::Failed("iOS render owner is not prepared".into()))?;
        if let Err(error) =
            self.capture
                .cancel(&mut gpu.host, &self.signals, "device generation recovery")
        {
            self.capture.failed(&mut gpu.host, &self.signals, &error);
        }
        let fail = |error: FrameError| RecoverFault::Failed(error.to_string());
        let inputs = gpu.host.device_prepare_inputs().map_err(fail)?;
        let generation = self.next_generation;
        self.next_generation += 1;
        let (adapter, device, queue) = request_device(&instance).map_err(RecoverFault::Failed)?;
        let watch = Watch::register(&device, generation, &self.lost_wake);
        let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
        if watch.is_lost() {
            return Err(RecoverFault::CandidateLost);
        }
        let extent = self.lease.observed_extent();
        if extent.contains(&0) {
            return Err(RecoverFault::Deferred);
        }
        let mut prepared = pollster::block_on(inputs.prepare(
            &capabilities,
            &queue,
            Some(SurfaceProfile {
                format: gpu.format,
                extent,
            }),
        ))
        .map_err(fail)?;
        prepared.resize_scene_surface(extent).map_err(fail)?;
        let mut held = Duration::ZERO;
        if self.signals.hold_recover.load(Ordering::Acquire) {
            // Route only: widen the window between preparation and the refit
            // check so a rotation lands inside it.
            (self.recover_held)();
            let hold = Instant::now();
            let until = hold + RECOVER_HOLD_MAX;
            while self.signals.hold_recover.load(Ordering::Acquire) && Instant::now() < until {
                thread::sleep(HOLD_SLICE);
            }
            held = hold.elapsed();
        }
        if watch.is_lost() {
            return Err(RecoverFault::CandidateLost);
        }
        // A rotation during preparation: fit the candidate before publishing.
        let current = self.lease.observed_extent();
        if current.contains(&0) {
            return Err(RecoverFault::Deferred);
        }
        let refitted = current != extent;
        if refitted {
            prepared.resize_scene_surface(current).map_err(fail)?;
            if watch.is_lost() {
                return Err(RecoverFault::CandidateLost);
            }
        }
        // Publish inside a section under the current lease: a suspend sets
        // `background` before it revokes, and cannot take the surface while
        // the section holds it, so no publication lands in the background.
        // The surface is left unconfigured for the new device.
        let background = &self.signals.background;
        let published = self.lease.section(self.lease.epoch(), |leased| {
            if background.load(Ordering::Acquire) {
                return None;
            }
            if self.lease.observed_extent() != current || current.contains(&0) {
                return None;
            }
            leased.configured = None;
            Some(gpu.host.publish_device(&mut prepared))
        });
        let Some(Some(published)) = published else {
            return Err(RecoverFault::Deferred);
        };
        published.map_err(fail)?;
        gpu.adapter = adapter;
        gpu.device = device;
        gpu.queue = queue;
        gpu.watch = watch;
        gpu.host_extent = current;
        Ok(Recovered {
            from,
            generation,
            extent: current,
            refitted,
            wall: started.elapsed().saturating_sub(held),
        })
    }

    fn frame_on_device(&mut self, request: &FrameRequest) -> Result<FrameReply, FrameError> {
        let lease = Arc::clone(&self.lease);
        let now = self.origin.elapsed();
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(FrameError::InvalidState("iOS render owner is not prepared"))?;
        if self.diagnostics
            && let Err(error) = self.capture.before(
                &mut gpu.host,
                &self.signals,
                &lease,
                request,
                gpu.watch.generation,
                &gpu.adapter,
            )
        {
            self.capture.failed(&mut gpu.host, &self.signals, &error);
        }
        let lead = display_target_lead(request.target_media_seconds, media_time_now());
        if let Some(view) = &mut gpu.cooked_view {
            view.update(
                &mut gpu.host,
                self.signals.cooked_pan.swap(false, Ordering::AcqRel),
            )
            .map_err(FrameError::Scene)?;
        }
        gpu.host.set_presentation_lead(lead.duration);
        let mut reply = FrameReply::new(request, gpu.watch.generation, lead.status);
        let usable = !request.extent.contains(&0);
        let ready = if !request.present || !usable {
            reply.status = SurfaceStatus::Denied;
            false
        } else {
            match lease.section(request.epoch, |leased| {
                gpu.configure(leased, request, &lease)
            }) {
                None => {
                    reply.status = SurfaceStatus::NoLease;
                    false
                }
                Some(Err(error)) => return Err(error),
                Some(Ok(configured)) => {
                    reply.configured = configured.is_some_and(|fresh| fresh);
                    if configured.is_none() {
                        reply.status = SurfaceStatus::Reconfigure;
                    }
                    configured.is_some()
                }
            }
        };
        reply.report = if ready {
            gpu.present(
                now,
                &lease,
                request,
                &mut reply,
                &*self.stalling,
                &self.signals.lose_now,
                &self.signals.background,
                self.diagnostics,
            )?
        } else if !usable || reply.status == SurfaceStatus::Reconfigure {
            // Drain old-grid submissions before resizing. Encoding another old
            // view here can keep a busy ring occupied indefinitely.
            drop(gpu.host.advance(now)?);
            None
        } else {
            Some(gpu.host.drive(now, None)?)
        };
        if reply.status == SurfaceStatus::DeviceLost {
            // Lost inside the section: no completion poll on the lost device.
            return Ok(reply);
        }
        for _ in 0..SAMPLE_SLOTS {
            let Some(sample) = gpu.host.poll_completed()? else {
                break;
            };
            reply.completed += 1;
            reply.surface_samples += u32::from(sample.surface_presented);
        }
        reply.cooked = gpu
            .cooked
            .as_ref()
            .and_then(|stats| stats.lock().ok())
            .map_or_else(String::new, |stats| {
                format!(
                    "cooked_path={:?}\ncooked_drawn={:?}\ncooked_device_generation={:?}\ncooked_submitted={}\ncooked_errors={:?}",
                    stats.draw_path, stats.drawn, stats.drawn_device_generation, stats.submitted_frames, stats.errors
                )
            });
        reply.pending_frames = gpu.host.pending_frames();
        reply.native_witness(&gpu.host);
        let culling = gpu.host.culling_selection();
        let _ = write!(
            reply.cooked,
            "\nculling_requested={:?}\nculling_resolved={:?}\nculling_reason={:?}\n",
            culling.requested, culling.resolved, culling.reason
        );
        if let Some(view) = &gpu.cooked_view {
            reply.cooked.push_str(&view.witness());
        }
        if self.diagnostics
            && let Err(error) = self
                .capture
                .after(&mut gpu.host, &reply, &lease, &self.signals)
        {
            self.capture.failed(&mut gpu.host, &self.signals, &error);
        }
        Ok(reply)
    }
}

impl Gpu {
    /// Configure the leased surface for this epoch if needed. `None` means it
    /// must wait for submitted frames to retire; `Some(true)` means configured now.
    fn configure(
        &mut self,
        leased: &mut Leased<wgpu::Surface<'static>>,
        request: &FrameRequest,
        lease: &Lease,
    ) -> Result<Option<bool>, FrameError> {
        if leased.configured == Some((request.epoch, request.extent)) {
            return Ok(Some(false));
        }
        if self.host.pending_frames() != 0 {
            return Ok(None);
        }
        if request.extent != self.host_extent {
            if !self
                .host
                .resize_scene_surface_if(request.extent, || lease.is_current(request.epoch))?
            {
                return Ok(None);
            }
            self.host_extent = request.extent;
        }
        let mut config = leased
            .surface
            .get_default_config(&self.adapter, request.extent[0], request.extent[1])
            .ok_or(FrameError::InvalidState("iOS surface has no configuration"))?;
        if !leased
            .surface
            .get_capabilities(&self.adapter)
            .formats
            .contains(&self.format)
        {
            return Err(FrameError::InvalidState(
                "replacement iOS surface changed format",
            ));
        }
        config.format = self.format;
        config.desired_maximum_frame_latency = 2;
        leased.surface.configure(&self.device, &config);
        leased.configured = Some((request.epoch, request.extent));
        Ok(Some(true))
    }

    /// Returns `None` when the frame was abandoned on a lost device: its
    /// world progress stays committed, but no report or queue call follows.
    fn present(
        &mut self,
        now: Duration,
        lease: &Lease,
        request: &FrameRequest,
        reply: &mut FrameReply,
        stalling: &dyn Fn(),
        lose_now: &AtomicBool,
        background: &AtomicBool,
        diagnostics: bool,
    ) -> Result<Option<DriveReport>, FrameError> {
        let advanced = self.host.advance(now)?;
        if !advanced.can_acquire_serial() {
            return advanced.submit(None).map(Some);
        }
        let encoded = advanced.encode_scene()?;
        if !encoded.can_acquire_surface() {
            // Nothing recorded, so this skip makes no queue call.
            return Ok(Some(encoded.skip()));
        }
        if diagnostics {
            reply.frame_extent = encoded.frame_extent();
        }
        reply.presentable_encode = true;
        let queue = &self.queue;
        let device = &self.device;
        let lost = &self.watch.lost;
        let destroyed = std::cell::Cell::new(false);
        let is_lost = || destroyed.get() || lost.load(Ordering::Acquire);
        if is_lost() {
            return Ok(abandon(encoded, reply));
        }
        let mut pending = Some(encoded);
        let result = lease.section(request.epoch, |leased| {
            let encoded = pending
                .take()
                .expect("encoded frame enters the section once");
            if background.load(Ordering::Acquire) {
                reply.status = SurfaceStatus::NoLease;
                return Ok(Some(encoded.cancel_background()));
            }
            let acquiring = Instant::now();
            let texture = leased.surface.get_current_texture();
            reply.acquire = acquiring.elapsed();
            if is_lost() {
                // wgpu's callback fired during the acquire.
                reply.acquired = matches!(
                    texture,
                    CurrentSurfaceTexture::Success(_) | CurrentSurfaceTexture::Suboptimal(_)
                );
                drop(texture);
                return Ok(abandon(encoded, reply));
            }
            let frame = match texture {
                CurrentSurfaceTexture::Success(frame) => frame,
                CurrentSurfaceTexture::Suboptimal(frame) => {
                    leased.configured = None;
                    frame
                }
                CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                    reply.status = SurfaceStatus::Missing;
                    return Ok(Some(cancel_revoked(encoded, reply, background)));
                }
                CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                    leased.configured = None;
                    reply.status = SurfaceStatus::Reconfigure;
                    return Ok(Some(cancel_revoked(encoded, reply, background)));
                }
                CurrentSurfaceTexture::Validation => {
                    cancel_revoked(encoded, reply, background);
                    return Err(FrameError::InvalidState("iOS surface validation fault"));
                }
            };
            reply.acquired = true;
            let hold = |point: StallPoint, reply: &mut FrameReply| {
                if let Some((at, stall)) = request.stall
                    && at == point
                {
                    reply.stalled = true;
                    stalling();
                    // A route `lose-device` that lands inside the hold destroys
                    // the device while this drawable is held.
                    let until = Instant::now() + stall;
                    while Instant::now() < until {
                        if lose_now.swap(false, Ordering::AcqRel) {
                            device.destroy();
                            destroyed.set(true);
                        }
                        thread::sleep(HOLD_SLICE);
                    }
                }
            };
            hold(StallPoint::AfterAcquire, reply);
            if is_lost() {
                // Nothing more is called on a lost device; the drawable drops.
                drop(frame);
                return Ok(abandon(encoded, reply));
            }
            if !lease.is_current(request.epoch) {
                reply.status = SurfaceStatus::StaleDropped;
                drop(frame);
                return Ok(Some(cancel_revoked(encoded, reply, background)));
            }
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            let report = encoded.submit(Some(&SurfaceOutput {
                view: &view,
                extent: request.extent,
            }))?;
            if !lease.is_current(request.epoch) {
                reply.status = SurfaceStatus::StaleDropped;
                drop(frame);
            } else if report.surface_presented {
                let drawn = [frame.texture.width(), frame.texture.height()];
                reply.drawable = drawn;
                if drawn != lease.observed_extent() {
                    reply.status = SurfaceStatus::StaleDropped;
                    reply.extent_dropped = true;
                    drop(frame);
                    return Ok(Some(report));
                }
                hold(StallPoint::BeforePresent, reply);
                queue.present(frame);
                reply.status = SurfaceStatus::Presented;
                reply.raced_present = !lease.is_current(request.epoch);
                reply.old_extent_present = drawn != lease.observed_extent();
            }
            Ok(Some(report))
        });
        if destroyed.get() {
            self.watch.destroyed = true;
        }
        if let Some(report) = result {
            return report;
        }
        let encoded = pending
            .take()
            .expect("a refused section leaves the encoded frame");
        if self.watch.destroyed || self.watch.is_lost() {
            return Ok(abandon(encoded, reply));
        }
        reply.status = SurfaceStatus::NoLease;
        Ok(Some(cancel_revoked(encoded, reply, background)))
    }
}

/// Abandon a recorded frame on a lost device. A healthy skip would flush
/// staging with an empty submit to the lost queue; this makes no queue call.
fn abandon(encoded: EncodedFrame<'_>, reply: &mut FrameReply) -> Option<DriveReport> {
    reply.status = SurfaceStatus::DeviceLost;
    reply.abandoned = true;
    encoded.abandon_lost_device();
    None
}

/// Background revocation forbids queue work and resets correspondence. A
/// foreground resize/replacement keeps the existing healthy staging-flush contract.
fn cancel_revoked(
    encoded: EncodedFrame<'_>,
    reply: &mut FrameReply,
    background: &AtomicBool,
) -> DriveReport {
    if background.load(Ordering::Acquire) {
        encoded.cancel_background()
    } else {
        skip(encoded, reply)
    }
}

/// Skip a recorded frame on a healthy device. The engine flushes its staged
/// writes with an empty queue submit, which the witness counts.
fn skip(encoded: EncodedFrame<'_>, reply: &mut FrameReply) -> DriveReport {
    reply.empty_submit = true;
    encoded.skip()
}

#[expect(
    unsafe_code,
    reason = "QuartzCore's media clock is the display link's time base"
)]
fn media_time_now() -> f64 {
    unsafe extern "C" {
        fn CACurrentMediaTime() -> f64;
    }
    // SAFETY: CACurrentMediaTime takes no arguments, reads a monotonic clock
    // and is documented as callable from any thread.
    unsafe { CACurrentMediaTime() }
}
