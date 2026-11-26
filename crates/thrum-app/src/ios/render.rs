//! The render owner: device preparation, scene encoding, late acquisition and
//! presentation on one resident thread behind the engine's bounded handoff.

use std::{
    fmt::Write as _,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use super::{MobileGame, MobileHost, MobileOptions, MobilePrepared, SceneRequest};
use crate::Gpu as PrepareGpu;
use thrum_render::{
    capabilities::{
        DeviceRecipe, DeviceWatch, FIRST_DEVICE_GENERATION, OpenedDevice, PREFERRED_SDR_FORMAT,
        configure_surface, sdr_surface_format,
    },
    frame::{FrameWorker, TargetLeadStatus, display_target_lead},
    present::SurfaceProfile,
    runtime::{DriveReport, EncodedFrame, FrameError, FrameWorld, SceneSource, SurfaceOutput},
};

/// Longest qualification hold of a recovery before it proceeds anyway.
const RECOVER_HOLD_MAX: Duration = Duration::from_secs(300);
/// Sleep slice inside qualification holds, so a loss can land inside them.
const HOLD_SLICE: Duration = Duration::from_millis(20);
use crate::{
    frame::SAMPLE_SLOTS,
    surface::{SurfaceAction, surface_action},
};
use wgpu::TextureViewDescriptor;

use crate::ios_lease::{Leased, SurfaceLease, TickPermission};

mod replace;

pub(super) type Lease = SurfaceLease<wgpu::Surface<'static>>;
pub(super) type RenderWorker<G> = FrameWorker<Job, Reported<G>>;

pub(super) enum Job {
    Prepare {
        instance: wgpu::Instance,
        epoch: u64,
        extent: [u32; 2],
        origin: Instant,
        scene: Option<SceneRequest>,
    },
    ReplaceScene(SceneRequest),
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

pub(super) type FrameRequest = crate::frame::FrameRequest<FrameAdmission>;

pub(super) struct FrameAdmission {
    /// The shell permits drawable acquisition (its view is shown).
    pub(super) present: bool,
    /// Qualification only: hold an acquired drawable inside the lease section.
    pub(super) stall: Option<(StallPoint, Duration)>,
    #[cfg(feature = "interface")]
    pub(super) interface: Option<crate::OutputMetrics>,
}

pub(super) struct Prepared<G: MobileGame> {
    pub(super) scene: SceneReady<G>,
    pub(super) adapter: String,
    pub(super) format: wgpu::TextureFormat,
}

pub(super) struct SceneReady<G: MobileGame> {
    pub(super) origin: Instant,
    pub(super) controls: G::Controls,
    pub(super) input: G::Input,
    #[cfg(feature = "interface")]
    pub(super) output_metrics: bool,
}

impl<G: MobileGame> SceneReady<G> {
    fn split(
        prepared: MobilePrepared<G>,
        origin: Instant,
    ) -> (MobileHost<G>, G::Presentation, Self) {
        (
            prepared.host,
            prepared.scene,
            Self {
                origin,
                controls: prepared.controls,
                input: prepared.input,
                #[cfg(feature = "interface")]
                output_metrics: prepared.output_metrics,
            },
        )
    }
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

pub(super) type FrameReply<R> = crate::frame::FrameReply<Option<DriveReport>, FrameObservation<R>>;

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent per-frame witnesses, not a state machine"
)]
pub(super) struct FrameObservation<R> {
    /// Lease epoch admitted by the request.
    pub(super) epoch: u64,
    pub(super) frame_extent: Option<thrum_render::runtime::FrameExtent>,
    pub(super) drawable: [u32; 2],
    pub(super) status: SurfaceStatus,
    pub(super) cooked: String,
    pub(super) pending_frames: usize,
    pub(super) headroom: f32,
    pub(super) format: Option<wgpu::TextureFormat>,
    pub(super) acquired: bool,
    /// The acquire call ran without the requested epoch and extent configured.
    pub(super) unconfigured_acquire: bool,
    pub(super) presentable_encode: bool,
    pub(super) configured: bool,
    pub(super) stalled: bool,
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
    /// Submission order and predecessor of an encoded frame cancelled in the background.
    pub(super) background_discard: Option<(u64, u64)>,
    pub(super) readback: Option<R>,
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
pub(super) enum Reply<G: MobileGame> {
    Prepared(Result<Prepared<G>, String>),
    SceneReplaced {
        request: SceneRequest,
        result: Result<Option<SceneReady<G>>, String>,
    },
    Frame(Result<FrameReply<G::Readback>, String>),
    Recovered(Result<Recovered, RecoverFault>),
    Actor(Option<[f64; 3]>),
}

pub(super) struct Reported<G: MobileGame> {
    pub(super) reply: Reply<G>,
    /// Thread that produced the reply.
    pub(super) thread: Option<String>,
}

/// Flags the shell sets and the owner reads; none orders frame admission.
#[derive(Default)]
pub(super) struct Signals {
    pub(super) headroom: AtomicU64,
    pub(super) capture: AtomicU64,
    pub(super) capture_callbacks: super::capture::Callbacks,
    /// Optional game qualification command for its render owner.
    pub(super) game_command: AtomicU64,
    /// Foreground transitions observed by the inline world's clock.
    pub(super) resumes: AtomicU64,
    /// Route only: destroy the device at the owner's next opportunity.
    pub(super) lose_now: AtomicBool,
    /// Route only: hold the next recovery after preparation, before the
    /// refit check and publication.
    pub(super) hold_recover: AtomicBool,
    /// Route only: wake the shell once with the previous generation's loss,
    /// through the same path wgpu's lost callback uses.
    pub(super) stale_wake: AtomicBool,
    /// The app is in the background: no recovery may prepare or publish.
    pub(super) background: Arc<AtomicBool>,
}

type LostWake = Arc<dyn Fn(u64) + Send + Sync>;

struct Gpu<G: MobileGame> {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    watch: DeviceWatch,
    destroyed: bool,
    host: MobileHost<G>,
    request_epoch: Arc<AtomicU64>,
    presentation: G::Presentation,
    format: wgpu::TextureFormat,
    host_extent: [u32; 2],
}

struct Owner<G: MobileGame> {
    game: G,
    capture: super::capture::Recorder,
    lease: Arc<Lease>,
    /// The instance the first device came from; recoveries use it again.
    instance: Option<wgpu::Instance>,
    signals: Arc<Signals>,
    gpu: Option<Gpu<G>>,
    resumes: u64,
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

pub(super) fn start<G: MobileGame>(
    game: G,
    lease: Arc<Lease>,
    signals: Arc<Signals>,
    options: &MobileOptions,
    wakes: Wakes,
) -> Result<RenderWorker<G>, String> {
    let Wakes {
        reply: wake,
        stalling,
        lost,
        recover_held,
    } = wakes;
    let mut owner = Owner {
        game,
        capture: super::capture::Recorder::default(),
        lease,
        instance: None,
        signals,
        gpu: None,
        diagnostics: options.witness.is_some(),
        resumes: 0,
        origin: Instant::now(),
        next_generation: FIRST_DEVICE_GENERATION + 1,
        lost_wake: lost,
        stalling,
        recover_held,
    };
    // The thread is named on preparation and the first frame reply only, so
    // frames add no allocation for the witness.
    let mut frame_named = false;
    FrameWorker::start(
        options.render_thread,
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

impl<R> FrameObservation<R> {
    fn native_witness<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &thrum_render::runtime::FrameHost<W, V>,
    ) {
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
}
fn frame_reply<R>(
    request: &FrameRequest,
    generation: u64,
    target: TargetLeadStatus,
) -> FrameReply<R> {
    FrameReply {
        sequence: request.sequence,
        device_generation: generation,
        outcome: None,
        samples: [const { None }; SAMPLE_SLOTS],
        target_status: Some(target),
        acquire_wall: None,
        surface_extent: request.extent,
        platform: FrameObservation {
            epoch: request.surface_generation,
            frame_extent: None,
            drawable: [0; 2],
            status: SurfaceStatus::NoView,
            cooked: String::new(),
            pending_frames: 0,
            headroom: 1.0,
            format: None,
            acquired: false,
            unconfigured_acquire: false,
            presentable_encode: false,
            configured: false,
            stalled: false,
            raced_present: false,
            extent_dropped: false,
            old_extent_present: false,
            empty_submit: false,
            abandoned: false,
            background_discard: None,
            readback: None,
        },
    }
}

impl<G: MobileGame> Owner<G> {
    fn work(&mut self, job: Job) -> Reply<G> {
        match job {
            Job::Prepare {
                instance,
                epoch,
                extent,
                origin,
                scene,
            } => {
                self.origin = origin;
                Reply::Prepared(self.prepare(&instance, epoch, extent, scene))
            }
            Job::ReplaceScene(request) => Reply::SceneReplaced {
                result: self.replace_scene(&request),
                request,
            },
            Job::Frame(request) => Reply::Frame(self.frame(&request)),
            Job::Recover { from } => Reply::Recovered(self.recover(from)),
            Job::SampleActor => Reply::Actor(
                self.gpu
                    .as_ref()
                    .and_then(|gpu| G::sample_position(&gpu.host)),
            ),
        }
    }

    fn prepare(
        &mut self,
        instance: &wgpu::Instance,
        epoch: u64,
        extent: [u32; 2],
        scene: Option<SceneRequest>,
    ) -> Result<Prepared<G>, String> {
        if self.gpu.is_some() {
            return Err("iOS render owner is already prepared".into());
        }
        self.instance = Some(instance.clone());
        let wake = Arc::clone(&self.lost_wake);
        let OpenedDevice {
            adapter,
            device,
            queue,
            capabilities,
            watch,
        } = pollster::block_on(DeviceRecipe::default().open_with_wake(
            instance,
            None,
            move |generation| {
                wake(generation);
            },
        ))
        .map_err(|error| error.to_string())?;
        // Adapter selection cannot hold the revocable surface lease while the
        // request runs. Check the live format offer when the lease is current.
        let format = self
            .lease
            .section(epoch, |leased| {
                sdr_surface_format(&leased.surface.get_capabilities(&adapter).formats)
            })
            .unwrap_or(Some(PREFERRED_SDR_FORMAT))
            .ok_or("no supported iOS SDR surface format")?;
        let extent = [extent[0].max(1), extent[1].max(1)];
        let prepared = pollster::block_on(self.game.prepare(
            PrepareGpu {
                device: &device,
                queue: &queue,
                capabilities: &capabilities,
                surface: SurfaceProfile { format, extent },
            },
            self.origin,
            super::notify_world,
            scene,
            self.diagnostics,
        ))
        .map_err(|error| error.to_string())?;
        let (mut host, presentation, scene) = SceneReady::split(prepared, self.origin);
        let request_epoch = Arc::new(AtomicU64::new(epoch));
        host.set_tick_submission_permission(Box::new(TickPermission {
            lease: Arc::clone(&self.lease),
            request_epoch: Arc::clone(&request_epoch),
            background: Arc::clone(&self.signals.background),
        }));
        let name = format!(
            "{:?}; downlevel={:?}; recipe={:?}; registry={}; unified_memory={:?}; texture_profiles={:?}; max_storage_buffers_per_shader_stage={}",
            capabilities.adapter(),
            capabilities.downlevel(),
            capabilities.recipe(),
            capabilities.registry_digest(),
            capabilities.unified_memory(),
            thrum_asset_gpu::texture_profiles(device.features()),
            device.limits().max_storage_buffers_per_shader_stage
        );
        self.gpu = Some(Gpu {
            adapter,
            device,
            queue,
            watch,
            destroyed: false,
            host,
            presentation,
            format,
            host_extent: extent,
            request_epoch,
        });
        Ok(Prepared {
            scene,
            adapter: name,
            format,
        })
    }

    /// One frame job. From the moment the owner knows its device is lost (it
    /// destroyed it, or wgpu's callback fired) it makes no call on it except
    /// the non-blocking poll that delivers wgpu's lost callback.
    fn frame(&mut self, request: &FrameRequest) -> Result<FrameReply<G::Readback>, String> {
        let unrun = |generation, status| {
            let lead = display_target_lead(request.target_media_seconds, media_time_now());
            let mut reply = frame_reply(request, generation, lead.status);
            reply.platform.status = status;
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
                reply.platform.native_witness(&gpu.host);
                return Ok(reply);
            }
            if self.signals.stale_wake.swap(false, Ordering::AcqRel)
                && gpu.watch.generation > FIRST_DEVICE_GENERATION
            {
                (self.lost_wake)(gpu.watch.generation - 1);
            }
            if self.signals.lose_now.swap(false, Ordering::AcqRel) && !gpu.destroyed {
                gpu.device.destroy();
                gpu.destroyed = true;
            }
            if gpu.destroyed && !gpu.watch.is_lost() {
                // `destroy` only marks the device; wgpu reports the loss from
                // maintenance. Never `Wait` on it.
                let _ = gpu.device.poll(wgpu::PollType::Poll);
            }
            if gpu.destroyed || gpu.watch.is_lost() {
                if let Err(error) =
                    self.capture
                        .cancel(&mut gpu.host, &self.signals, "device generation lost")
                {
                    self.capture.failed(&mut gpu.host, &self.signals, &error);
                }
                return Ok(unrun(gpu.watch.generation, SurfaceStatus::DeviceLost));
            }
            if request.device_generation != gpu.watch.generation {
                return Ok(unrun(gpu.watch.generation, SurfaceStatus::StaleGeneration));
            }
        }
        let result = self.frame_on_device(request);
        let gpu = self
            .gpu
            .as_mut()
            .ok_or("iOS render owner is not prepared")?;
        if gpu.destroyed && !gpu.watch.is_lost() {
            let _ = gpu.device.poll(wgpu::PollType::Poll);
        }
        let lost = gpu.destroyed || gpu.watch.is_lost();
        match result {
            Err(FrameError::NativeCommandBuffer { generation })
                if generation == gpu.watch.generation =>
            {
                gpu.watch.retire();
                (self.lost_wake)(generation);
                Ok(unrun(generation, SurfaceStatus::DeviceLost))
            }
            Ok(reply) => Ok(reply),
            Err(_) if lost => {
                // The error may follow an acquisition: assume a drawable was
                // held so the shell replaces the surface (walk step 6).
                let mut reply = unrun(gpu.watch.generation, SurfaceStatus::DeviceLost);
                reply.platform.acquired = true;
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
        let wake = Arc::clone(&self.lost_wake);
        let OpenedDevice {
            adapter,
            device,
            queue,
            capabilities,
            watch,
        } = pollster::block_on(
            DeviceRecipe {
                generation,
                ..Default::default()
            }
            .open_with_wake(&instance, None, move |generation| wake(generation)),
        )
        .map_err(|error| RecoverFault::Failed(error.to_string()))?;
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
        gpu.destroyed = false;
        gpu.host_extent = current;
        Ok(Recovered {
            from,
            generation,
            extent: current,
            refitted,
            wall: started.elapsed().saturating_sub(held),
        })
    }

    fn frame_on_device(
        &mut self,
        request: &FrameRequest,
    ) -> Result<FrameReply<G::Readback>, FrameError> {
        let lease = Arc::clone(&self.lease);
        let now = self.origin.elapsed();
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(FrameError::InvalidState("iOS render owner is not prepared"))?;
        gpu.request_epoch
            .store(request.surface_generation, Ordering::Release);
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
        let resumes = self.signals.resumes.load(Ordering::Acquire);
        if resumes != self.resumes {
            G::resume_inline(gpu.host.world_mut(), now)
                .map_err(|error| FrameError::World(error.to_string()))?;
            self.resumes = resumes;
        }
        #[cfg(feature = "interface")]
        if let Some(output) = request.platform.interface {
            G::output(&mut gpu.host, output);
        }
        G::frame(
            &mut gpu.host,
            &mut gpu.presentation,
            self.signals.game_command.swap(0, Ordering::AcqRel),
            now,
        )
        .map_err(|error| FrameError::Scene(error.to_string()))?;
        gpu.host.set_presentation_lead(lead.duration);
        let mut reply = frame_reply(request, gpu.watch.generation, lead.status);
        let bits = u32::try_from(self.signals.headroom.load(Ordering::Acquire)).unwrap_or_default();
        reply.platform.headroom = f32::from_bits(bits).max(1.0);
        reply.platform.format = Some(gpu.format);
        let usable = !request.extent.contains(&0);
        let ready = if !request.platform.present || !usable {
            reply.platform.status = SurfaceStatus::Denied;
            false
        } else {
            match lease.section(request.surface_generation, |leased| {
                gpu.configure(leased, request, &lease)
            }) {
                None => {
                    reply.platform.status = SurfaceStatus::NoLease;
                    false
                }
                Some(Err(error)) => return Err(error),
                Some(Ok(configured)) => {
                    reply.platform.configured = configured.is_some_and(|fresh| fresh);
                    if configured.is_none() {
                        reply.platform.status = SurfaceStatus::Reconfigure;
                    }
                    configured.is_some()
                }
            }
        };
        reply.outcome = if ready {
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
        } else if !usable || reply.platform.status == SurfaceStatus::Reconfigure {
            // Drain old-grid submissions before resizing. Encoding another old
            // view here can keep a busy ring occupied indefinitely.
            drop(gpu.host.advance(now)?);
            None
        } else {
            Some(gpu.host.drive(now, None)?)
        };
        if reply.platform.status == SurfaceStatus::DeviceLost {
            // Lost inside the section: no completion poll on the lost device.
            return Ok(reply);
        }
        if reply.platform.status == SurfaceStatus::Presented {
            reply.platform.readback = G::readback(&mut gpu.host);
            #[cfg(feature = "interface")]
            if reply
                .platform
                .readback
                .as_ref()
                .and_then(G::viewport)
                .is_some_and(|viewport| viewport.extent != request.extent)
            {
                reply.platform.readback = None;
            }
        }
        for slot in &mut reply.samples {
            *slot = gpu.host.poll_completed()?;
            if slot.is_none() {
                break;
            }
        }
        reply.platform.cooked = G::frame_witness(&gpu.host, &gpu.presentation);
        reply.platform.pending_frames = gpu.host.pending_frames();
        reply.platform.native_witness(&gpu.host);
        if self.diagnostics {
            let _ = write!(
                reply.platform.cooked,
                "\nreconstructor={:?}",
                gpu.host.resolved_reconstructor()
            );
        }
        let culling = gpu.host.culling_selection();
        let _ = write!(
            reply.platform.cooked,
            "\nculling_requested={:?}\nculling_resolved={:?}\nculling_reason={:?}\n",
            culling.requested, culling.resolved, culling.reason
        );
        if self.diagnostics
            && let Err(error) = self.capture.after(
                &mut gpu.host,
                &reply,
                &lease,
                &self.signals,
                G::still_bounds,
            )
        {
            self.capture.failed(&mut gpu.host, &self.signals, &error);
        }
        Ok(reply)
    }
}

impl<G: MobileGame> Gpu<G> {
    /// Configure the leased surface for this epoch if needed. `None` means it
    /// must wait for submitted frames to retire; `Some(true)` means configured now.
    fn configure(
        &mut self,
        leased: &mut Leased<wgpu::Surface<'static>>,
        request: &FrameRequest,
        lease: &Lease,
    ) -> Result<Option<bool>, FrameError> {
        if leased.configured == Some((request.surface_generation, request.extent)) {
            return Ok(Some(false));
        }
        if self.host.pending_frames() != 0 {
            return Ok(None);
        }
        if request.extent != self.host_extent {
            if !self.host.resize_scene_surface_if(request.extent, || {
                lease.is_current(request.surface_generation)
            })? {
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
        configure_surface(&leased.surface, &self.device, &config)
            .map_err(|_| FrameError::InvalidState("iOS surface configuration refused"))?;
        leased.configured = Some((request.surface_generation, request.extent));
        Ok(Some(true))
    }

    /// Returns `None` when the frame was abandoned on a lost device: its
    /// world progress stays committed, but no report or queue call follows.
    fn present(
        &mut self,
        now: Duration,
        lease: &Lease,
        request: &FrameRequest,
        reply: &mut FrameReply<G::Readback>,
        stalling: &dyn Fn(),
        lose_now: &AtomicBool,
        background: &AtomicBool,
        diagnostics: bool,
    ) -> Result<Option<DriveReport>, FrameError> {
        let observe_background_discard = self.host.native_witness_report().is_some()
            || (diagnostics && self.host.resolved_reconstructor().is_some());
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
            reply.platform.frame_extent = encoded.frame_extent();
        }
        reply.platform.presentable_encode = true;
        let queue = &self.queue;
        let device = &self.device;
        let watch = &self.watch;
        let destroyed = std::cell::Cell::new(false);
        let is_lost = || destroyed.get() || watch.is_lost();
        if is_lost() {
            return Ok(abandon(encoded, reply));
        }
        let mut pending = Some(encoded);
        let result = lease.section(request.surface_generation, |leased| {
            let encoded = pending
                .take()
                .expect("encoded frame enters the section once");
            if background.load(Ordering::Acquire) {
                reply.platform.status = SurfaceStatus::NoLease;
                return Ok(Some(cancel_background(encoded, reply)));
            }
            let acquiring = Instant::now();
            reply.platform.unconfigured_acquire =
                leased.configured != Some((request.surface_generation, request.extent));
            let action = surface_action(leased.surface.get_current_texture());
            reply.acquire_wall = Some(acquiring.elapsed());
            if is_lost() {
                // wgpu's callback fired during the acquire.
                reply.platform.acquired = matches!(action, SurfaceAction::Draw { .. });
                drop(action);
                return Ok(abandon(encoded, reply));
            }
            let frame = match action {
                SurfaceAction::Draw { frame, reconfigure } => {
                    if reconfigure {
                        leased.configured = None;
                    }
                    frame
                }
                SurfaceAction::Skip { .. } => {
                    reply.platform.status = SurfaceStatus::Missing;
                    return Ok(Some(cancel_revoked(encoded, reply, background)));
                }
                SurfaceAction::Reconfigure { .. } => {
                    leased.configured = None;
                    reply.platform.status = SurfaceStatus::Reconfigure;
                    return Ok(Some(cancel_revoked(encoded, reply, background)));
                }
            };
            reply.platform.acquired = true;
            let hold = |point: StallPoint, reply: &mut FrameReply<G::Readback>| {
                if let Some((at, stall)) = request.platform.stall
                    && at == point
                {
                    reply.platform.stalled = true;
                    stalling();
                    // A route `lose-device` that lands inside the hold destroys
                    // the device while this drawable is held.
                    let until = Instant::now() + stall;
                    while Instant::now() < until {
                        // Finish the diagnostic hold as soon as cancellation
                        // can be observed while the app is still in background.
                        if observe_background_discard
                            && background.load(Ordering::Acquire)
                            && !lease.is_current(request.surface_generation)
                        {
                            break;
                        }
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
            if !lease.is_current(request.surface_generation) {
                reply.platform.status = SurfaceStatus::StaleDropped;
                drop(frame);
                return Ok(Some(cancel_revoked(encoded, reply, background)));
            }
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            let report = encoded.submit(Some(&SurfaceOutput {
                view: &view,
                extent: request.extent,
            }))?;
            if !lease.is_current(request.surface_generation) {
                reply.platform.status = SurfaceStatus::StaleDropped;
                drop(frame);
            } else if report.surface_presented {
                let drawn = [frame.texture.width(), frame.texture.height()];
                reply.platform.drawable = drawn;
                if drawn != lease.observed_extent() {
                    reply.platform.status = SurfaceStatus::StaleDropped;
                    reply.platform.extent_dropped = true;
                    drop(frame);
                    return Ok(Some(report));
                }
                hold(StallPoint::BeforePresent, reply);
                queue.present(frame);
                reply.platform.status = SurfaceStatus::Presented;
                reply.platform.raced_present = !lease.is_current(request.surface_generation);
                reply.platform.old_extent_present = drawn != lease.observed_extent();
            }
            Ok(Some(report))
        });
        if destroyed.get() {
            self.destroyed = true;
        }
        if let Some(report) = result {
            return report;
        }
        let encoded = pending
            .take()
            .expect("a refused section leaves the encoded frame");
        if self.destroyed || self.watch.is_lost() {
            return Ok(abandon(encoded, reply));
        }
        reply.platform.status = SurfaceStatus::NoLease;
        Ok(Some(cancel_revoked(encoded, reply, background)))
    }
}

/// Abandon a recorded frame on a lost device. A healthy skip would flush
/// staging with an empty submit to the lost queue; this makes no queue call.
fn abandon<W: FrameWorld, V: SceneSource<W::Page>, R>(
    encoded: EncodedFrame<'_, W, V>,
    reply: &mut FrameReply<R>,
) -> Option<DriveReport> {
    reply.platform.status = SurfaceStatus::DeviceLost;
    reply.platform.abandoned = true;
    encoded.abandon_lost_device();
    None
}

/// Background revocation forbids queue work and resets correspondence. A
/// foreground resize/replacement keeps the existing healthy staging-flush contract.
fn cancel_revoked<W: FrameWorld, V: SceneSource<W::Page>, R>(
    encoded: EncodedFrame<'_, W, V>,
    reply: &mut FrameReply<R>,
    background: &AtomicBool,
) -> DriveReport {
    if background.load(Ordering::Acquire) {
        cancel_background(encoded, reply)
    } else {
        skip(encoded, reply)
    }
}

fn cancel_background<W: FrameWorld, V: SceneSource<W::Page>, R>(
    encoded: EncodedFrame<'_, W, V>,
    reply: &mut FrameReply<R>,
) -> DriveReport {
    reply.platform.background_discard = reply
        .platform
        .frame_extent
        .zip(encoded.history_identity())
        .map(|(extent, [_, predecessor])| (extent.submission, predecessor));
    encoded.cancel_background()
}

/// Skip a recorded frame on a healthy device. The engine flushes its staged
/// writes with an empty queue submit, which the witness counts.
fn skip<W: FrameWorld, V: SceneSource<W::Page>, R>(
    encoded: EncodedFrame<'_, W, V>,
    reply: &mut FrameReply<R>,
) -> DriveReport {
    reply.platform.empty_submit = true;
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
