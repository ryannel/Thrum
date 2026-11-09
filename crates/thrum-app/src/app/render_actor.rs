//! One render owner serializes frames, configuration and GPU replacement.
use super::{
    UserEvent,
    device::{DeviceRecipe, DeviceWatch, PlatformGpu, configure_surface},
};
use crate::Game;
use objc2_quartz_core::CACurrentMediaTime;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use thrum_render::{
    frame::{FrameWorker, TargetLeadStatus, display_target_lead},
    present::SurfaceProfile,
    runtime::{
        DriveReport, EncodedFrame, FrameError, FrameHost, FrameSample, FrameWorld, SceneSource,
        SurfaceOutput,
    },
};
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};
use winit::event_loop::EventLoopProxy;

/// Completed GPU samples polled per job; the runtime retains at most this many frames.
pub(super) const SAMPLE_SLOTS: usize = 2;
pub(super) struct FrameRequest<E> {
    pub(super) sequence: u64,
    pub(super) device_generation: u64,
    pub(super) target_media_seconds: f64,
    pub(super) extent: [u32; 2],
    pub(super) surface_generation: u64,
    pub(super) edits: E,
}
pub(super) enum RenderJob<E> {
    Frame(FrameRequest<E>),
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
}
impl FrameCounters {
    pub(super) fn read<W: FrameWorld, V: SceneSource<W::Page>>(host: &FrameHost<W, V>) -> Self {
        Self {
            skipped_views: host.dropped_views(),
            expired_views: host.dropped_stale_views(),
            missed_surfaces: host.dropped_surface(),
            lost_reports: host.dropped_reports(),
        }
    }
}
#[derive(Debug, Default)]
pub(super) enum SurfaceStatus {
    #[default]
    NotAttempted,
    Presented,
    Unavailable,
    Occluded,
    NeedsReconfigure,
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
            FrameError::Gpu(message) => Self::Unavailable(message),
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
pub(super) struct RenderReply<R> {
    pub(super) sequence: u64,
    pub(super) device_generation: u64,
    pub(super) outcome: RenderOutcome,
    pub(super) samples: [Option<FrameSample>; SAMPLE_SLOTS],
    pub(super) counters: FrameCounters,
    pub(super) worker_wall: Duration,
    pub(super) target_status: Option<TargetLeadStatus>,
    pub(super) acquire_wall: Option<Duration>,
    pub(super) surface_status: SurfaceStatus,
    pub(super) surface_extent: [u32; 2],
    /// The game's readback after this job, for its controls to show.
    pub(super) readback: R,
}
struct Actor<G: Game> {
    host: FrameHost<G::World, G::Scene>,
    gpu: PlatformGpu,
    recipe: DeviceRecipe,
    surface_generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    needs_reconfigure: bool,
    refused_surface_generation: Option<u64>,
    origin: Instant,
    wake: EventLoopProxy<UserEvent>,
    observer: Option<super::qualification::Observer>,
}
pub(super) type RenderWorker<G> =
    FrameWorker<RenderJob<<G as Game>::Edits>, RenderReply<<G as Game>::Readback>>;
#[expect(
    clippy::too_many_arguments,
    reason = "the render owner takes every platform handle it serializes in one move"
)]
pub(super) fn start<G: Game + 'static>(
    name: &str,
    mut host: FrameHost<G::World, G::Scene>,
    gpu: PlatformGpu,
    recipe: DeviceRecipe,
    surface_generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    origin: Instant,
    wake: &EventLoopProxy<UserEvent>,
) -> Result<RenderWorker<G>, String> {
    const {
        assert!(FrameHost::<G::World, G::Scene>::MAX_IN_FLIGHT == SAMPLE_SLOTS);
    }
    let signal = wake.clone();
    let observer = super::qualification::Observer::start(&mut host, origin)?;
    let mut actor = Actor::<G> {
        observer,
        host,
        gpu,
        recipe,
        surface_generation,
        stopped,
        needs_reconfigure: false,
        refused_surface_generation: None,
        origin,
        wake: wake.clone(),
    };
    FrameWorker::start(
        name,
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
        let mut reply = RenderReply {
            sequence: 0,
            device_generation: self.gpu.watch.generation,
            outcome: RenderOutcome::DeviceLost,
            samples: [None; SAMPLE_SLOTS],
            counters: FrameCounters::read(&self.host),
            worker_wall: Duration::ZERO,
            target_status: None,
            acquire_wall: None,
            surface_status: SurfaceStatus::NotAttempted,
            surface_extent: [self.gpu.config.width, self.gpu.config.height],
            readback: G::readback(&self.host),
        };
        reply.outcome = match job {
            RenderJob::Frame(request) => {
                reply.sequence = request.sequence;
                match self.frame(&request, &mut reply) {
                    Ok(report) => {
                        let mut failure = None;
                        if !self.gpu.watch.is_lost() {
                            for sample in &mut reply.samples {
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
        if !self.gpu.watch.is_lost()
            && let Some(observer) = &mut self.observer
            && let Err(error) = observer.collect::<G>(&mut self.host)
        {
            reply.outcome = RenderOutcome::Frame(Err(error));
        }
        reply.device_generation = self.gpu.watch.generation;
        reply.surface_extent = [self.gpu.config.width, self.gpu.config.height];
        reply.counters = FrameCounters::read(&self.host);
        reply.readback = G::readback(&self.host);
        reply.worker_wall = start.elapsed();
        reply
    }
    fn frame(
        &mut self,
        request: &FrameRequest<G::Edits>,
        reply: &mut RenderReply<G::Readback>,
    ) -> Result<DriveReport, FrameError> {
        if request.device_generation != self.gpu.watch.generation {
            return Err(FrameError::InvalidState("stale frame device generation"));
        }
        // CPU policy errors remain fatal even when a loss is concurrent.
        G::apply(&mut self.host, request.edits).map_err(FrameError::Scene)?;
        ensure_device(&self.gpu.watch)?;
        ensure_running(&self.stopped)?;
        let current = self.surface_generation.load(Ordering::Acquire) == request.surface_generation;
        if !current {
            self.needs_reconfigure = true;
        }
        let extent = [self.gpu.config.width, self.gpu.config.height];
        let mut ready = current
            && !request.extent.contains(&0)
            && self.refused_surface_generation != Some(request.surface_generation)
            && (request.extent == extent && !self.needs_reconfigure
                || self.host.pending_frames() == 0);
        if ready && (request.extent != extent || self.needs_reconfigure) {
            let mut config = self.gpu.config.clone();
            config.width = request.extent[0];
            config.height = request.extent[1];
            let result = configure_surface(&self.recipe.surface, &self.gpu.device, &config);
            ensure_device(&self.gpu.watch)?;
            if let Err(error) = result {
                self.needs_reconfigure = true;
                self.refused_surface_generation = Some(request.surface_generation);
                reply.surface_status = SurfaceStatus::ConfigurationRefused(error);
                ready = false;
            } else {
                if let Some(observer) = &mut self.observer {
                    observer
                        .replace::<G>(&mut self.host)
                        .map_err(FrameError::Scene)?;
                }
                self.host.resize_surface(request.extent)?;
                self.gpu.config = config;
                self.needs_reconfigure = false;
                self.refused_surface_generation = None;
            }
        }
        let now = self.origin.elapsed();
        let lead = display_target_lead(request.target_media_seconds, CACurrentMediaTime());
        reply.target_status = Some(lead.status);
        self.host.set_presentation_lead(lead.duration);
        if !ready {
            if matches!(reply.surface_status, SurfaceStatus::NotAttempted) {
                reply.surface_status = if current {
                    SurfaceStatus::Unavailable
                } else {
                    SurfaceStatus::NeedsReconfigure
                };
            }
            return self.host.drive(now, None);
        }
        let advanced = self.host.advance(now)?;
        if !advanced.can_acquire_serial() {
            return advanced.submit(None);
        }
        let encoded = check_encoded_device(&self.gpu.watch, advanced.encode_scene()?)?;
        if !encoded.can_acquire_surface() {
            return Ok(encoded.skip());
        }
        if self.surface_generation.load(Ordering::Acquire) != request.surface_generation {
            self.needs_reconfigure = true;
            reply.surface_status = SurfaceStatus::NeedsReconfigure;
            return Ok(encoded.skip());
        }
        ensure_running(&self.stopped)?;
        let acquire = Instant::now();
        let acquired = self.recipe.surface.get_current_texture();
        reply.acquire_wall = Some(acquire.elapsed());
        let encoded = check_encoded_device(&self.gpu.watch, encoded)?;
        let frame = match acquired {
            CurrentSurfaceTexture::Success(frame) => frame,
            CurrentSurfaceTexture::Suboptimal(frame) => {
                self.needs_reconfigure = true;
                frame
            }
            CurrentSurfaceTexture::Timeout => {
                reply.surface_status = SurfaceStatus::Unavailable;
                return Ok(encoded.skip());
            }
            CurrentSurfaceTexture::Occluded => {
                reply.surface_status = SurfaceStatus::Occluded;
                return Ok(encoded.skip());
            }
            CurrentSurfaceTexture::Outdated
            | CurrentSurfaceTexture::Lost
            | CurrentSurfaceTexture::Validation => {
                self.needs_reconfigure = true;
                reply.surface_status = SurfaceStatus::NeedsReconfigure;
                return Ok(encoded.skip());
            }
        };
        if self.surface_generation.load(Ordering::Acquire) != request.surface_generation {
            self.needs_reconfigure = true;
            reply.surface_status = SurfaceStatus::NeedsReconfigure;
            return Ok(encoded.skip());
        }
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent: request.extent,
        };
        let report = encoded.submit(Some(&output))?;
        ensure_device(&self.gpu.watch)?;
        if report.surface_presented {
            self.gpu.queue.present(frame);
            reply.surface_status = SurfaceStatus::Presented;
        } else {
            reply.surface_status = SurfaceStatus::Unavailable;
        }
        if report.submitted
            && let Some(observer) = &mut self.observer
        {
            observer
                .frame::<G>(
                    &self.host,
                    request.sequence,
                    request.surface_generation,
                    report.surface_presented,
                )
                .map_err(FrameError::Scene)?;
            if observer.replace_after_present() {
                self.surface_generation.fetch_add(1, Ordering::AcqRel);
                self.needs_reconfigure = true;
            }
        }
        Ok(report)
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
        let inputs = self
            .host
            .device_prepare_inputs()
            .map_err(RecoveryFailure::frame)?;
        let replacement = pollster::block_on(self.recipe.create(generation, extent, &self.wake))
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
        // The previous job has dropped its drawable, but the lost device
        // may have prevented wgpu from clearing that surface acquisition.
        let needs_reconfigure = extent.contains(&0)
            || self.surface_generation.load(Ordering::Acquire) != surface_generation;
        ensure_device(&replacement.watch).map_err(RecoveryFailure::frame)?;
        ensure_running(&self.stopped).map_err(RecoveryFailure::frame)?;
        if !needs_reconfigure
            && let Err(error) = configure_surface(
                &self.recipe.surface,
                &replacement.device,
                &replacement.config,
            )
            && self.surface_generation.load(Ordering::Acquire) == surface_generation
        {
            return Err(RecoveryFailure::Unavailable(error));
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
        self.refused_surface_generation = None;
        self.needs_reconfigure = needs_reconfigure
            || self.surface_generation.load(Ordering::Acquire) != surface_generation;
        ensure_device(&self.gpu.watch).map_err(RecoveryFailure::frame)?;
        Ok(RecoveryInfo {
            format: self.gpu.config.format,
            adapter_name: self.gpu.adapter_name.clone(),
        })
    }
    fn finish(&mut self) -> Result<Finished, String> {
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
                Err(FrameError::Gpu(_)) if self.gpu.watch.is_lost() => {
                    return Ok(Finished {
                        samples,
                        lost_device: true,
                    });
                }
                Err(error) => return Err(error.to_string()),
            }
            if self.host.pending_frames() == 0 {
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
        Err(FrameError::Gpu(
            "device loss reported by callback".to_owned(),
        ))
    } else {
        Ok(())
    }
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
        FrameError::Gpu(_) if lost => RenderOutcome::DeviceLost,
        other => RenderOutcome::Frame(Err(other.to_string())),
    }
}

#[cfg(test)]
#[path = "render_actor_tests.rs"]
mod tests;
