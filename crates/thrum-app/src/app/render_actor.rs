//! One render owner serializes frames, configuration and GPU replacement.
use super::{
    UserEvent,
    device::{PlatformGpu, WindowDevice},
    qualification::TransitionProof,
    refresh_account::{Account, Place},
};
use crate::{Config, Game, frame::SAMPLE_SLOTS, inline_lifecycle::InlineLifecycle};
use objc2_quartz_core::CACurrentMediaTime;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use thrum_render::{
    capabilities::{DeviceWatch, retry_surface_configuration},
    frame::{FrameWorker, TargetLeadStatus, display_target_lead},
    present::SurfaceProfile,
    runtime::{
        DriveReport, EncodedFrame, ExtentRequest, ExtentTransition, FrameError, FrameHost,
        FrameSample, FrameWorld, SceneKind, SceneSource, SurfaceLease, SurfaceOutput,
        SurfaceSwitchOutcome,
    },
};
use wgpu::TextureViewDescriptor;

use crate::surface::{SurfaceAction, surface_action};
use winit::event_loop::EventLoopProxy;

pub(super) type FrameRequest<E> = crate::frame::FrameRequest<FrameEdits<E>>;
pub(super) struct FrameEdits<E> {
    pub(super) profile_revision: u64,
    pub(super) timing_requested: bool,
    pub(super) backing_scale: u64,
    pub(super) governor_display: thrum_render::governor::DisplayRecord,
    pub(super) display_tick: u64,
    pub(super) edits: E,
}
pub(super) enum RenderJob<E> {
    Frame(FrameRequest<E>),
    #[cfg(feature = "interface")]
    LoseForInterfaceProof,
    Recover {
        from_generation: u64,
        generation: u64,
        extent: [u32; 2],
        surface_generation: u64,
    },
    Finish,
}
#[derive(Clone, Copy, Default)]
pub(super) struct FrameCounters {
    pub(super) skipped_views: u64,
    pub(super) expired_views: u64,
    pub(super) missed_surfaces: u64,
    pub(super) lost_reports: u64,
    pub(super) inline_overloads: (u64, u64),
}
impl FrameCounters {
    pub(super) fn read<W: FrameWorld, V: SceneSource<W::Page>>(host: &FrameHost<W, V>) -> Self {
        Self {
            skipped_views: host.dropped_views(),
            expired_views: host.dropped_stale_views(),
            missed_surfaces: host.dropped_surface(),
            lost_reports: host.dropped_reports(),
            inline_overloads: (0, 0),
        }
    }
}
#[derive(Debug, Default)]
pub(super) enum SurfaceStatus {
    #[default]
    NotAttempted,
    Presented,
    FrameSlotsOccupied,
    WorldOverloaded,
    WorldPageUnavailable,
    SceneFrameUnavailable,
    DrawableTimeout,
    WindowExtentEmpty,
    SurfaceConfigurationPending,
    SurfaceConfigurationBlocked,
    SurfacePreparationBusy,
    SubmissionSkipped,
    Occluded,
    NeedsReconfigure,
    SurfaceEpochChanged,
    ConfigurationRefused(String),
}
pub(super) struct RecoveryInfo {
    pub(super) format: wgpu::TextureFormat,
    pub(super) adapter_name: String,
}
pub(super) enum RecoveryFailure {
    Unavailable(String),
    Fatal(String),
}
impl RecoveryFailure {
    fn frame(error: FrameError) -> Self {
        match error {
            FrameError::Gpu { operation, error } => {
                Self::Unavailable(format!("{operation}: {error}"))
            }
            FrameError::DeviceLost(message) | FrameError::ContributionDeviceLimit(message) => {
                Self::Unavailable(message)
            }
            error @ FrameError::TimingDiscard { .. } => Self::Unavailable(error.to_string()),
            other => Self::Fatal(other.to_string()),
        }
    }
}
pub(super) enum RenderOutcome {
    Frame(Result<DriveReport, String>),
    DeviceLost,
    Recovered {
        attempted: u64,
        result: Result<RecoveryInfo, RecoveryFailure>,
    },
    Finish(Result<Finished, String>),
}
pub(super) struct Finished {
    pub(super) samples: Vec<FrameSample>,
    pub(super) lost_device: bool,
}
pub(super) type RenderReply<R> = crate::frame::FrameReply<RenderOutcome, FrameReadback<R>>;

pub(super) struct FrameReadback<R> {
    pub(super) profile_revision: u64,
    pub(super) profile_minimum: u64,
    pub(super) counters: FrameCounters,
    pub(super) worker_wall: Duration,
    pub(super) surface_status: SurfaceStatus,
    pub(super) transition_error: Option<String>,
    pub(super) surface_format: wgpu::TextureFormat,
    pub(super) readback: R,
}

pub(super) fn collect_observer<R, W: FrameWorld, V: SceneSource<W::Page>>(
    reply: &mut RenderReply<R>,
    watch: &DeviceWatch,
    host: &mut FrameHost<W, V>,
    observer: Option<&mut super::qualification::Observer>,
) {
    // A native completion can report loss independently of the wgpu callback.
    // Never collect from a reply already handed to recovery (or shutdown).
    if !matches!(reply.outcome, RenderOutcome::Frame(Ok(_))) || watch.is_lost() {
        return;
    }
    if let Some(observer) = observer
        && let Err(error) = observer.collect(host)
    {
        // Collection can be the first observer of a native completion error.
        reply.outcome = frame_failure(error, watch.is_lost());
    }
}

struct Actor<G: Game> {
    host: FrameHost<G::World, G::Scene>,
    inline_lifecycle: InlineLifecycle,
    gpu: PlatformGpu,
    recipe: WindowDevice,
    surface_generation: Arc<AtomicU64>,
    surface_lease: SurfaceLease,
    stopped: Arc<AtomicBool>,
    current_extent: ExtentRequest,
    transition: ExtentTransition,
    transition_proof: Option<TransitionProof>,
    display_proof: Option<super::display::proof::DisplayProof>,
    origin: Instant,
    frame_budget: Option<Duration>,
    wake: EventLoopProxy<UserEvent>,
    observer: Option<super::qualification::Observer>,
    extent_capture: Option<super::qualification::ExtentCapture>,
    profile_minimum: u64,
    profile_revision: u64,
    refresh_account: Arc<Mutex<Account>>,
}
pub(super) type RenderWorker<G> =
    FrameWorker<RenderJob<<G as Game>::Edits>, RenderReply<<G as Game>::Readback>>;
#[expect(
    clippy::too_many_arguments,
    reason = "the render owner takes every platform handle it serializes in one move"
)]
pub(super) fn start<G: Game<World: Send, Scene: Send> + 'static>(
    config: &Config,
    mut host: FrameHost<G::World, G::Scene>,
    gpu: PlatformGpu,
    recipe: WindowDevice,
    surface_generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    origin: Instant,
    wake: &EventLoopProxy<UserEvent>,
    extent_output: Option<super::WindowExtentCapture>,
    backing_scale: u64,
    proof: Option<&super::WindowTransitionProof>,
    qualification: Option<&super::WindowQualification>,
    display: Option<&super::WindowDisplayOptions>,
    refresh_account: Arc<Mutex<Account>>,
) -> Result<RenderWorker<G>, String> {
    const {
        assert!(FrameHost::<G::World, G::Scene>::MAX_IN_FLIGHT == SAMPLE_SLOTS);
    }
    let signal = wake.clone();
    let observer = super::qualification::Observer::start(
        &mut host,
        origin,
        [gpu.config.width, gpu.config.height],
        proof,
        qualification,
    )?;
    let extent_capture = extent_output
        .map(|output| {
            super::qualification::ExtentCapture::start(
                &mut host,
                output,
                f64::from_bits(backing_scale),
            )
        })
        .transpose()?;
    let current_extent = ExtentRequest {
        extent: [gpu.config.width, gpu.config.height],
        backing_scale,
        format: gpu.config.format,
        device: gpu.watch.generation,
    };
    let transition_proof = proof.map(TransitionProof::start).transpose()?;
    let display_proof = super::display::proof::DisplayProof::start(&mut host, display)?;
    let mut actor = Actor::<G> {
        transition: ExtentTransition::new(current_extent),
        transition_proof,
        display_proof,
        current_extent,
        observer,
        extent_capture,
        profile_minimum: 0,
        profile_revision: 0,
        refresh_account,
        host,
        inline_lifecycle: InlineLifecycle::default(),
        gpu,
        recipe,
        surface_lease: SurfaceLease::new(surface_generation.load(Ordering::Acquire), false),
        surface_generation,
        stopped,
        origin,
        frame_budget: config.frame_budget,
        wake: wake.clone(),
    };
    FrameWorker::start(
        config.render_thread,
        move |job| actor.work(job),
        move || {
            let _ = signal.send_event(UserEvent::RenderDone);
        },
    )
    .map_err(|error| format!("render worker: {error}"))
}
impl<G: Game> Actor<G> {
    fn work(&mut self, job: RenderJob<G::Edits>) -> RenderReply<G::Readback> {
        let start = Instant::now();
        if let RenderJob::Frame(request) = &job {
            let arrived = CACurrentMediaTime();
            self.refresh_account
                .lock()
                .expect("refresh account lock")
                .begin(request.target_media_seconds, arrived);
        }
        let mut reply = RenderReply {
            sequence: 0,
            device_generation: self.gpu.watch.generation,
            outcome: RenderOutcome::DeviceLost,
            samples: [const { None }; SAMPLE_SLOTS],
            target_status: None,
            acquire_wall: None,
            surface_extent: [self.gpu.config.width, self.gpu.config.height],
            platform: FrameReadback {
                profile_revision: self.profile_revision,
                profile_minimum: self.profile_minimum,
                counters: FrameCounters::read(&self.host),
                worker_wall: Duration::ZERO,
                surface_status: SurfaceStatus::NotAttempted,
                transition_error: None,
                surface_format: self.gpu.config.format,
                readback: G::readback(&self.host),
            },
        };
        let mut backing = None;
        reply.outcome = match job {
            RenderJob::Frame(request) => {
                reply.sequence = request.sequence;
                if let Some(proof) = &mut self.transition_proof {
                    proof.begin_frame(request.sequence);
                }
                backing = Some((
                    request.extent,
                    request.platform.backing_scale,
                    request.platform.governor_display,
                ));
                let mut drawable_late = false;
                let mut present_call = None;
                let frame = self
                    .frame(&request, &mut reply, &mut drawable_late, &mut present_call)
                    .and_then(|report| {
                        if let Some(world) = G::inline_world(self.host.world_mut()) {
                            self.inline_lifecycle
                                .observe(world, report, self.origin.elapsed())
                                .map_err(|error| FrameError::World(error.to_string()))?;
                        }
                        Ok(report)
                    });
                let place = match reply.platform.surface_status {
                    SurfaceStatus::FrameSlotsOccupied => Place::FrameSlot,
                    SurfaceStatus::DrawableTimeout
                    | SurfaceStatus::WindowExtentEmpty
                    | SurfaceStatus::SurfaceConfigurationPending
                    | SurfaceStatus::SurfaceConfigurationBlocked
                    | SurfaceStatus::SurfacePreparationBusy
                    | SurfaceStatus::Occluded
                    | SurfaceStatus::NeedsReconfigure
                    | SurfaceStatus::SurfaceEpochChanged
                    | SurfaceStatus::ConfigurationRefused(_) => Place::Drawable,
                    _ if drawable_late => Place::Drawable,
                    _ => Place::EncodeSubmit,
                };
                self.refresh_account
                    .lock()
                    .expect("refresh account lock")
                    .complete(place, present_call);
                if let Some(proof) = &mut self.transition_proof {
                    proof.stage("collecting frame completions and reply");
                }
                match frame {
                    Ok(report) => {
                        let mut failure = None;
                        if !self.gpu.watch.is_lost() {
                            for sample in reply.samples.iter_mut().filter(|sample| sample.is_none())
                            {
                                match self.host.poll_completed() {
                                    Ok(next) => *sample = next,
                                    Err(error) => {
                                        failure = Some(error);
                                        break;
                                    }
                                }
                                if sample.is_none() {
                                    break;
                                }
                            }
                        }
                        if let Some(error) = failure {
                            frame_failure(error, self.gpu.watch.is_lost())
                        } else if self.gpu.watch.is_lost() {
                            RenderOutcome::DeviceLost
                        } else {
                            RenderOutcome::Frame(Ok(report))
                        }
                    }
                    Err(error) => frame_failure(error, self.gpu.watch.is_lost()),
                }
            }
            #[cfg(feature = "interface")]
            RenderJob::LoseForInterfaceProof => {
                self.gpu.device.destroy();
                RenderOutcome::DeviceLost
            }
            RenderJob::Recover {
                from_generation,
                generation,
                extent,
                surface_generation,
            } => RenderOutcome::Recovered {
                attempted: generation,
                result: self.recover(from_generation, generation, extent, surface_generation),
            },
            RenderJob::Finish => RenderOutcome::Finish(self.finish()),
        };
        collect_observer(
            &mut reply,
            &self.gpu.watch,
            &mut self.host,
            self.observer.as_mut(),
        );
        if let RenderOutcome::Frame(Ok(frame)) = &reply.outcome
            && let Some(proof) = &mut self.display_proof
            && let Err(error) = proof.frame(&mut self.host, reply.sequence, frame, &self.gpu.config)
        {
            reply.outcome = RenderOutcome::Frame(Err(error));
        }
        if matches!(reply.outcome, RenderOutcome::DeviceLost) {
            self.transition.cancel();
        }
        if matches!(reply.outcome, RenderOutcome::Frame(Ok(_)))
            && let Some(capture) = &mut self.extent_capture
        {
            let recipe = format!("{:?}", self.gpu.capabilities.recipe());
            match capture.collect(&mut self.host, &recipe, &self.wake) {
                Ok(true) => self.extent_capture = None,
                Ok(false) => {}
                Err(error) => reply.outcome = frame_failure(error, self.gpu.watch.is_lost()),
            }
        }
        reply.device_generation = self.gpu.watch.generation;
        reply.platform.profile_minimum = self.profile_minimum;
        reply.platform.profile_revision = self.profile_revision;
        reply.surface_extent = [self.gpu.config.width, self.gpu.config.height];
        reply.platform.surface_format = self.gpu.config.format;
        reply.platform.counters = FrameCounters::read(&self.host);
        reply.platform.counters.inline_overloads = self.inline_lifecycle.counts();
        reply.platform.readback = G::readback(&self.host);
        reply.platform.worker_wall = start.elapsed();
        if let (Some(proof), Some((extent, scale, display))) = (&mut self.transition_proof, backing)
            && let Err(error) = proof.frame(&reply, extent, scale, display, &self.host)
        {
            reply.outcome = RenderOutcome::Frame(Err(error.to_string()));
        }
        reply
    }
    fn observe_display_target(
        &mut self,
        target_media_seconds: f64,
        ready: bool,
    ) -> (Duration, TargetLeadStatus) {
        if ready && let Some(budget) = self.frame_budget {
            let lead = display_target_lead(target_media_seconds, CACurrentMediaTime());
            let delay = lead.duration.saturating_sub(budget);
            if lead.status == TargetLeadStatus::Future && !delay.is_zero() {
                // Reserve time to encode and observe the world closer to the
                // deadline. The AppKit target still determines sampling.
                std::thread::sleep(delay);
            }
        }
        let observation = self.origin.elapsed();
        let media_now = CACurrentMediaTime();
        let lead = display_target_lead(target_media_seconds, media_now);
        self.host.set_presentation_lead(lead.duration);
        let target_seconds = observation.as_secs_f64() + (target_media_seconds - media_now);
        if lead.status != TargetLeadStatus::Invalid
            && let Ok(target) = Duration::try_from_secs_f64(target_seconds)
        {
            self.host.set_presentation_target(target, lead.status);
        }
        (observation, lead.status)
    }

    fn frame(
        &mut self,
        request: &FrameRequest<G::Edits>,
        reply: &mut RenderReply<G::Readback>,
        drawable_late: &mut bool,
        present_call: &mut Option<(f64, u64)>,
    ) -> Result<DriveReport, FrameError> {
        if request.device_generation != self.gpu.watch.generation {
            return Err(FrameError::InvalidState("stale frame device generation"));
        }
        // CPU policy errors remain fatal even when a loss is concurrent.
        if request.platform.profile_revision != self.profile_revision {
            self.profile_minimum = self.host.last_submission().map_or(0, |order| order + 1);
            self.profile_revision = request.platform.profile_revision;
        }
        if request.platform.timing_requested != self.host.timing_requested() {
            self.profile_minimum = self.host.last_submission().map_or(0, |order| order + 1);
            self.host
                .set_timing_requested(request.platform.timing_requested);
        }
        G::apply(&mut self.host, request.platform.edits).map_err(|error| match error {
            crate::GameError::Frame(error) => error,
            other => FrameError::Scene(other.to_string()),
        })?;
        ensure_device(&self.gpu.watch)?;
        ensure_running(&self.stopped)?;
        let headroom = self
            .recipe
            .headroom
            .as_ref()
            .map_or((1.0, 1.0), |record| record.load());
        let current = self.surface_generation.load(Ordering::Acquire) == request.surface_generation;
        if current {
            self.surface_lease.observe(request.surface_generation);
            self.advance_extent(request, reply, headroom.1)?;
        } else {
            self.surface_lease.invalidate();
        }
        self.host.set_display_headroom(headroom.0);
        // A sprite scene has one output pixel per backing pixel. A pending
        // generation can leave the old surface usable while the window grows.
        let matches_grid = self.host.scene_kind() != SceneKind::TwoD
            || request.extent == self.current_extent.extent;
        let ready = current
            && matches_grid
            && !request.extent.contains(&0)
            && self.transition.surface.usable()
            && !self.surface_lease.refused(request.surface_generation)
            && !self.surface_lease.pending();
        let now = self.origin.elapsed();
        let observed = request.platform.governor_display;
        let display = self.transition_proof.as_ref().map_or(observed, |proof| {
            proof.display_record(observed, request.platform.display_tick)
        });
        self.host
            .governor_display(display, self.transition.holds_governor());
        self.host
            .governor_display_tick(now, request.platform.display_tick);
        if let Some(replay) = self
            .transition_proof
            .as_ref()
            .and_then(|proof| proof.scale_replay)
        {
            let tick = self
                .host
                .world()
                .source()
                .tick
                .saturating_sub(replay.origin);
            self.host.set_render_scale((replay.scale_at)(tick))?;
        }
        if ready && !self.host.has_free_slot() {
            // A callback can arrive while an older packet is retiring. Give
            // it the part of this opportunity that encoding can spare.
            let lead = display_target_lead(request.target_media_seconds, CACurrentMediaTime());
            let deadline = Instant::now() + lead.duration.saturating_sub(Duration::from_millis(3));
            wait_for_frame_slot(
                &mut self.host,
                &mut reply.samples,
                &self.stopped,
                || deadline.saturating_duration_since(Instant::now()),
                std::thread::sleep,
            )?;
        }
        let (observation, status) =
            self.observe_display_target(request.target_media_seconds, ready);
        reply.target_status = Some(status);
        if !ready {
            if matches!(reply.platform.surface_status, SurfaceStatus::NotAttempted) {
                reply.platform.surface_status = if !current {
                    SurfaceStatus::SurfaceEpochChanged
                } else if request.extent.contains(&0) {
                    SurfaceStatus::WindowExtentEmpty
                } else if self.surface_lease.refused(request.surface_generation)
                    || !self.transition.surface.usable()
                {
                    SurfaceStatus::SurfaceConfigurationBlocked
                } else if self.transition.busy() {
                    SurfaceStatus::SurfacePreparationBusy
                } else if self.host.pending_frames() != 0 {
                    SurfaceStatus::FrameSlotsOccupied
                } else {
                    SurfaceStatus::SurfaceConfigurationPending
                };
            }
            return self.host.drive(observation, None);
        }
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("advancing the world and polling GPU completions");
        }
        let advanced = self.host.advance(observation)?;
        if !advanced.can_acquire_serial() {
            let report = advanced.submit(None)?;
            reply.platform.surface_status = if report.overload.is_some() {
                SurfaceStatus::WorldOverloaded
            } else if !self.host.world().has_current_page() {
                SurfaceStatus::WorldPageUnavailable
            } else {
                SurfaceStatus::FrameSlotsOccupied
            };
            return Ok(report);
        }
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("encoding the scene");
        }
        let encoded = check_encoded_device(&self.gpu.watch, advanced.encode_scene()?)?;
        if !encoded.can_acquire_surface() {
            reply.platform.surface_status = SurfaceStatus::SceneFrameUnavailable;
            return Ok(encoded.skip());
        }
        if self.surface_generation.load(Ordering::Acquire) != request.surface_generation {
            self.surface_lease.invalidate();
            reply.platform.surface_status = SurfaceStatus::SurfaceEpochChanged;
            return Ok(encoded.skip());
        }
        ensure_running(&self.stopped)?;
        let acquire = Instant::now();
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("acquiring a drawable");
        }
        let acquired_at = CACurrentMediaTime();
        let acquired = self.recipe.surface.get_current_texture();
        *drawable_late = acquired_at < request.target_media_seconds
            && request.target_media_seconds <= CACurrentMediaTime();
        reply.acquire_wall = Some(acquire.elapsed());
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("handling drawable acquisition");
        }
        let encoded = check_encoded_device(&self.gpu.watch, encoded)?;
        let frame = match surface_action(acquired) {
            SurfaceAction::Draw { frame, reconfigure } => {
                if reconfigure {
                    self.surface_lease.invalidate();
                }
                frame
            }
            SurfaceAction::Skip { occluded } => {
                reply.platform.surface_status = if occluded {
                    SurfaceStatus::Occluded
                } else {
                    SurfaceStatus::DrawableTimeout
                };
                return Ok(encoded.skip());
            }
            SurfaceAction::Reconfigure { .. } => {
                self.surface_lease.invalidate();
                reply.platform.surface_status = SurfaceStatus::NeedsReconfigure;
                return Ok(encoded.skip());
            }
        };
        if self.surface_generation.load(Ordering::Acquire) != request.surface_generation {
            self.surface_lease.invalidate();
            reply.platform.surface_status = SurfaceStatus::SurfaceEpochChanged;
            return Ok(encoded.skip());
        }
        if let Some(proof) = &mut self.transition_proof
            && proof
                .drawable(&frame.texture)
                .map_err(|error| FrameError::Scene(error.to_string()))?
        {
            return Err(FrameError::Scene(
                "control acquired conflicting drawable extents".into(),
            ));
        }
        let frame_extent = (self.extent_capture.is_some() || self.transition_proof.is_some())
            .then(|| encoded.frame_extent())
            .flatten();
        let drawable_extent = [frame.texture.width(), frame.texture.height()];
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent: [self.gpu.config.width, self.gpu.config.height],
        };
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("submitting the scene and presenting the drawable");
        }
        let report = encoded.submit(Some(&output))?;
        ensure_device(&self.gpu.watch)?;
        if report.surface_presented {
            *present_call = Some(self.present(frame));
            if let Some(proof) = &mut self.transition_proof {
                proof.presented(frame_extent);
            }
            reply.platform.surface_status = SurfaceStatus::Presented;
            if let Some(extent) = frame_extent
                && let Some(capture) = &mut self.extent_capture
            {
                capture.frame(extent, drawable_extent);
            }
        } else {
            reply.platform.surface_status = SurfaceStatus::SubmissionSkipped;
        }
        self.observe_submission(request, report)?;
        Ok(report)
    }

    fn present(&mut self, frame: wgpu::SurfaceTexture) -> (f64, u64) {
        let submission = self.host.last_submission().expect("presented submission");
        let time = {
            let mut account = self.refresh_account.lock().expect("refresh account lock");
            let time = CACurrentMediaTime();
            account.present(time, submission);
            time
        };
        self.gpu.queue.present(frame);
        self.host.governor_present_called();
        (time, submission)
    }

    fn observe_submission(
        &mut self,
        request: &FrameRequest<G::Edits>,
        report: DriveReport,
    ) -> Result<(), FrameError> {
        if report.submitted
            && let Some(observer) = &mut self.observer
        {
            observer
                .frame(
                    &self.host,
                    request.sequence,
                    request.surface_generation,
                    report.surface_presented,
                )
                .map_err(FrameError::Scene)?;
            if observer.replace_after_present() {
                self.surface_generation.fetch_add(1, Ordering::AcqRel);
                self.surface_lease.invalidate();
            }
        }
        Ok(())
    }
    fn advance_extent(
        &mut self,
        frame: &FrameRequest<G::Edits>,
        reply: &mut RenderReply<G::Readback>,
        potential: f32,
    ) -> Result<(), FrameError> {
        let promote = self.gpu.edr.is_some() && potential > 1.0;
        let request = ExtentRequest {
            extent: frame.extent,
            backing_scale: frame.platform.backing_scale,
            format: if promote {
                wgpu::TextureFormat::Rgba16Float
            } else {
                self.gpu.config.format
            },
            device: self.gpu.watch.generation,
        };
        if request.extent.contains(&0) {
            self.transition
                .observe(request, self.current_extent, self.origin.elapsed());
            return Ok(());
        }
        // Fault injection runs the same configure path as an ordinary switch.
        if self
            .transition_proof
            .as_ref()
            .is_some_and(TransitionProof::synchronous)
        {
            return self.synchronous_extent(frame, request);
        }
        if let Err(error) =
            self.transition
                .advance(request, self.current_extent, self.origin.elapsed(), || {
                    let inputs = self
                        .host
                        .extent_prepare_inputs_for_format(request.extent, request.format)?;
                    Ok(move |cancel: &AtomicBool| inputs.build(cancel))
                })
        {
            ensure_device(&self.gpu.watch)?;
            reply.platform.transition_error = Some(format!("{request:?}: {error}"));
        }
        if let Some(candidate) = self.transition.ready() {
            if !self.host.admits_extent(&candidate) {
                self.transition.discard(candidate);
                self.transition.refresh();
                return Ok(());
            }
            let config = self.gpu.config_for(request.extent, request.format);
            let mut calls = 0;
            let configure_started = CACurrentMediaTime();
            let outcome = self.transition.surface.take(
                false,
                self.transition.busy(),
                &config,
                &self.gpu.config,
                |config| {
                    calls += 1;
                    let mut attempt = config.clone();
                    if let Some(proof) = &mut self.transition_proof {
                        if calls == 2 && proof.skip_restore() {
                            return Ok(());
                        }
                        if proof.fault_configure(calls) {
                            attempt.width = self.gpu.device.limits().max_texture_dimension_2d + 1;
                        }
                    }
                    TransitionProof::configure(
                        self.transition_proof.as_mut(),
                        if calls == 1 { "switch" } else { "restore" },
                        &self.recipe.surface,
                        &self.gpu.device,
                        &attempt,
                    )
                },
            );
            if let Some(proof) = &mut self.transition_proof {
                proof.switch(request, self.transition.surface.record(), configure_started);
            }
            if let Some(proof) = &mut self.display_proof {
                proof.configures += u64::from(calls);
            }
            ensure_device(&self.gpu.watch)?;
            match outcome {
                SurfaceSwitchOutcome::Deferred => self.transition.defer(candidate),
                SurfaceSwitchOutcome::Taken => {
                    assert!(self.host.publish_extent(candidate));
                    self.gpu.config = config;
                    self.current_extent = request;
                    self.surface_lease.settle_extent(true);
                }
                SurfaceSwitchOutcome::Refused => {
                    self.transition.discard(candidate);
                    let record = self.transition.surface.record();
                    reply.platform.transition_error = Some(format!(
                        "{request:?}: {:?}; restore: {:?}",
                        record.configure_error, record.restore_error,
                    ));
                    // Do not route a refused transition through an automatic
                    // platform retry. Only a new request or a surface epoch can recover it.
                    self.surface_lease.settle_extent(false);
                }
            }
        }
        if let Some(result) =
            self.surface_lease
                .recover(self.transition.busy(), self.host.pending_frames(), || {
                    TransitionProof::configure(
                        self.transition_proof.as_mut(),
                        "platform",
                        &self.recipe.surface,
                        &self.gpu.device,
                        &self.gpu.config,
                    )
                })
        {
            match result {
                Ok(()) => {
                    self.transition.surface.platform_configured();
                    if let Some(observer) = &mut self.observer {
                        observer.replace(&mut self.host)?;
                    }
                }
                Err(error) => {
                    reply.platform.surface_status =
                        SurfaceStatus::ConfigurationRefused(error.to_string());
                }
            }
            ensure_device(&self.gpu.watch)?;
        }
        Ok(())
    }

    fn synchronous_extent(
        &mut self,
        frame: &FrameRequest<G::Edits>,
        request: ExtentRequest,
    ) -> Result<(), FrameError> {
        if self.current_extent == request && !self.surface_lease.pending() {
            return Ok(());
        }
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("synchronous extent build");
        }
        let candidate = self
            .host
            .extent_prepare_inputs(frame.extent)?
            .build(&AtomicBool::new(false))?
            .ok_or(FrameError::InvalidState(
                "synchronous extent control cancelled",
            ))?;
        if let Some(proof) = &mut self.transition_proof {
            proof.stage("switching the synchronous extent");
        }
        let mut config = self.gpu.config.clone();
        [config.width, config.height] = frame.extent;
        self.transition.surface.begin();
        let configure_started = CACurrentMediaTime();
        let outcome =
            self.transition
                .surface
                .take(false, false, &config, &self.gpu.config, |attempt| {
                    TransitionProof::configure(
                        self.transition_proof.as_mut(),
                        "synchronous control",
                        &self.recipe.surface,
                        &self.gpu.device,
                        attempt,
                    )
                });
        if let Some(proof) = &mut self.transition_proof {
            proof.switch(request, self.transition.surface.record(), configure_started);
        }
        if outcome == SurfaceSwitchOutcome::Taken {
            assert!(self.host.publish_extent(candidate));
            self.gpu.config = config;
            self.current_extent = request;
            self.surface_lease.settle_extent(true);
        }
        Ok(())
    }

    fn recover(
        &mut self,
        from: u64,
        generation: u64,
        extent: [u32; 2],
        surface_generation: u64,
    ) -> Result<RecoveryInfo, RecoveryFailure> {
        if from != self.gpu.watch.generation || generation <= from {
            return Err(RecoveryFailure::Fatal(
                "stale replacement generation".to_owned(),
            ));
        }
        ensure_running(&self.stopped).map_err(RecoveryFailure::frame)?;
        self.transition.cancel();
        let inputs = self
            .host
            .device_prepare_inputs()
            .map_err(RecoveryFailure::frame)?;
        let replacement = pollster::block_on(self.recipe.create(
            generation,
            extent,
            &self.wake,
            self.host.is_edr(),
        ))
        .map_err(RecoveryFailure::Unavailable)?;
        ensure_device(&replacement.watch).map_err(RecoveryFailure::frame)?;
        let profile = SurfaceProfile {
            format: replacement.config.format,
            extent: [replacement.config.width, replacement.config.height],
        };
        let mut prepared = pollster::block_on(inputs.prepare(
            &replacement.capabilities,
            &replacement.queue,
            Some(profile),
        ))
        .map_err(RecoveryFailure::frame)?;
        prepared
            .resize_scene_surface(profile.extent)
            .map_err(RecoveryFailure::frame)?;
        // The previous job has dropped its drawable, but the lost device
        // may have prevented wgpu from clearing that surface acquisition.
        let needs_reconfigure = extent.contains(&0)
            || self.surface_generation.load(Ordering::Acquire) != surface_generation;
        ensure_device(&replacement.watch).map_err(RecoveryFailure::frame)?;
        ensure_running(&self.stopped).map_err(RecoveryFailure::frame)?;
        if !needs_reconfigure
            && let Err(error) = retry_surface_configuration(|| {
                TransitionProof::configure(
                    self.transition_proof.as_mut(),
                    "recovery",
                    &self.recipe.surface,
                    &replacement.device,
                    &replacement.config,
                )
            })
            && self.surface_generation.load(Ordering::Acquire) == surface_generation
        {
            return Err(RecoveryFailure::Unavailable(error.to_string()));
        }
        replacement
            .device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| RecoveryFailure::Unavailable(format!("replacement poll: {error}")))?;
        ensure_device(&replacement.watch).map_err(RecoveryFailure::frame)?;
        ensure_running(&self.stopped).map_err(RecoveryFailure::frame)?;
        self.host
            .publish_device(&mut prepared)
            .map_err(RecoveryFailure::frame)?;
        self.gpu = replacement;
        if let Some(observer) = &mut self.observer {
            observer
                .replace_device(&mut self.host)
                .map_err(RecoveryFailure::frame)?;
        }
        self.surface_lease = SurfaceLease::new(
            surface_generation,
            needs_reconfigure
                || self.surface_generation.load(Ordering::Acquire) != surface_generation,
        );
        self.current_extent = ExtentRequest {
            extent: profile.extent,
            format: profile.format,
            device: generation,
            ..self.current_extent
        };
        self.transition
            .recovered(self.current_extent, !self.surface_lease.pending());
        ensure_device(&self.gpu.watch).map_err(RecoveryFailure::frame)?;
        Ok(RecoveryInfo {
            format: self.gpu.config.format,
            adapter_name: self.gpu.adapter_name.clone(),
        })
    }
    fn finish(&mut self) -> Result<Finished, String> {
        self.transition.cancel();
        let mut samples = Vec::with_capacity(SAMPLE_SLOTS * 2);
        loop {
            if self.gpu.watch.is_lost() {
                return Ok(Finished {
                    samples,
                    lost_device: true,
                });
            }
            match self.host.poll_completed() {
                Ok(Some(sample)) => {
                    samples.push(sample);
                    continue;
                }
                Ok(None) => {}
                Err(FrameError::NativeCommandBuffer { .. } | FrameError::DeviceLost(_)) => {
                    return Ok(Finished {
                        samples,
                        lost_device: true,
                    });
                }
                Err(
                    FrameError::Gpu { .. }
                    | FrameError::ContributionDeviceLimit(_)
                    | FrameError::TimingDiscard { .. },
                ) if self.gpu.watch.is_lost() => {
                    return Ok(Finished {
                        samples,
                        lost_device: true,
                    });
                }
                Err(error) => return Err(error.to_string()),
            }
            if self.host.pending_frames() == 0 {
                #[cfg(feature = "creation-counters")]
                if let Some(path) = self
                    .transition_proof
                    .as_ref()
                    .and_then(|proof| proof.creation_record.as_ref())
                {
                    std::fs::write(path, format!("{:#?}", self.host.creation_report()))
                        .map_err(|error| error.to_string())?;
                }
                if let Some(proof) = &mut self.display_proof {
                    proof.finish(&mut self.host)?;
                }
                if let Some(mut observer) = self.observer.take() {
                    observer.finish::<G>(&mut self.host)?;
                }
                return Ok(Finished {
                    samples,
                    lost_device: self.gpu.watch.is_lost(),
                });
            }
            let waited = self.gpu.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            });
            if drain_wait(waited, self.gpu.watch.is_lost())? {
                return Ok(Finished {
                    samples,
                    lost_device: true,
                });
            }
        }
    }
}
fn drain_wait(
    result: Result<wgpu::PollStatus, wgpu::PollError>,
    lost: bool,
) -> Result<bool, String> {
    match result {
        Ok(_) => Ok(lost),
        Err(wgpu::PollError::Timeout) if lost => Ok(true),
        Err(error) => Err(format!("GPU drain: {error:?}")),
    }
}
fn check_encoded_device<'a, W: FrameWorld, V: SceneSource<W::Page>>(
    watch: &DeviceWatch,
    encoded: EncodedFrame<'a, W, V>,
) -> Result<EncodedFrame<'a, W, V>, FrameError> {
    if let Err(error) = ensure_device(watch) {
        encoded.abandon_lost_device();
        return Err(error);
    }
    Ok(encoded)
}

fn ensure_device(watch: &DeviceWatch) -> Result<(), FrameError> {
    if watch.is_lost() {
        Err(FrameError::DeviceLost(
            watch
                .loss_message()
                .unwrap_or("device generation retired")
                .to_owned(),
        ))
    } else {
        Ok(())
    }
}
trait SlotWaitHost {
    fn ask_tick_stage(&mut self) -> Result<(), FrameError>;
    fn has_free_slot(&self) -> bool;
    fn poll_completed(&mut self) -> Result<Option<FrameSample>, FrameError>;
}

impl<W: FrameWorld, V: SceneSource<W::Page>> SlotWaitHost for FrameHost<W, V> {
    fn ask_tick_stage(&mut self) -> Result<(), FrameError> {
        FrameHost::ask_tick_stage(self)
    }

    fn has_free_slot(&self) -> bool {
        FrameHost::has_free_slot(self)
    }

    fn poll_completed(&mut self) -> Result<Option<FrameSample>, FrameError> {
        FrameHost::poll_completed(self)
    }
}

fn wait_for_frame_slot(
    host: &mut impl SlotWaitHost,
    samples: &mut [Option<FrameSample>],
    stopped: &AtomicBool,
    mut remaining: impl FnMut() -> Duration,
    mut sleep: impl FnMut(Duration),
) -> Result<(), FrameError> {
    host.ask_tick_stage()?;
    let mut samples = samples.iter_mut();
    while !host.has_free_slot() && !remaining().is_zero() && samples.len() != 0 {
        ensure_running(stopped)?;
        if let Some(completed) = host.poll_completed()? {
            *samples.next().expect("space for a completed sample") = Some(completed);
        } else if !host.has_free_slot() {
            sleep(Duration::from_micros(250).min(remaining()));
        }
    }
    Ok(())
}

fn ensure_running(stopped: &AtomicBool) -> Result<(), FrameError> {
    if stopped.load(Ordering::Acquire) {
        Err(FrameError::InvalidState("render owner is stopping"))
    } else {
        Ok(())
    }
}
fn frame_failure(error: FrameError, lost: bool) -> RenderOutcome {
    match error {
        FrameError::NativeCommandBuffer { .. } | FrameError::DeviceLost(_) => {
            RenderOutcome::DeviceLost
        }
        FrameError::Gpu { .. }
        | FrameError::ContributionDeviceLimit(_)
        | FrameError::TimingDiscard { .. }
            if lost =>
        {
            RenderOutcome::DeviceLost
        }
        other => RenderOutcome::Frame(Err(other.to_string())),
    }
}

#[cfg(test)]
mod tests;
