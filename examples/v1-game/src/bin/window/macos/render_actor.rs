//! Game callback bound to the engine's bounded native frame worker.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use objc2_quartz_core::CACurrentMediaTime;
use thrum_render::frame::{FrameWorker, TargetLeadStatus, display_target_lead};
use v1_game::{DriveReport, FrameHost, FrameSample, SurfaceOutput, ThreadedFrameHost};
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};
use winit::event_loop::EventLoopProxy;

use super::UserEvent;

pub(super) enum RenderJob {
    Frame {
        sequence: u64,
        target_media_seconds: f64,
        extent: [u32; 2],
        surface_generation: u64,
        time_of_day: Option<f32>,
        camera_pitch: Option<f32>,
        camera_yaw: Option<f32>,
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
    pub(super) fn read(host: &ThreadedFrameHost) -> Self {
        Self {
            skipped_views: host.dropped_views(),
            expired_views: host.dropped_stale_views(),
            missed_surfaces: host.dropped_surface(),
            lost_reports: host.dropped_reports(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SurfaceStatus {
    NotAttempted,
    Presented,
    Unavailable,
    Occluded,
    NeedsReconfigure,
}

pub(super) enum RenderOutcome {
    Frame(Result<DriveReport, String>),
    Finish(Result<Vec<FrameSample>, String>),
}

pub(super) struct RenderReply {
    pub(super) sequence: u64,
    pub(super) outcome: RenderOutcome,
    pub(super) samples: [Option<FrameSample>; FrameHost::MAX_IN_FLIGHT],
    pub(super) counters: FrameCounters,
    pub(super) worker_wall: Duration,
    pub(super) target_status: Option<TargetLeadStatus>,
    pub(super) acquire_wall: Option<Duration>,
    pub(super) surface_status: SurfaceStatus,
    pub(super) surface_extent: [u32; 2],
}

pub(super) struct SurfaceState {
    pub(super) surface: Arc<wgpu::Surface<'static>>,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) generation: Arc<AtomicU64>,
}

pub(super) fn start(
    mut host: ThreadedFrameHost,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: SurfaceState,
    origin: Instant,
    wake: &EventLoopProxy<UserEvent>,
) -> Result<FrameWorker<RenderJob, RenderReply>, String> {
    let signal = wake.clone();
    let SurfaceState {
        surface: surface_owner,
        config: mut surface_config,
        generation: active_surface_generation,
    } = surface;
    let mut current_extent = [surface_config.width, surface_config.height];
    let mut needs_reconfigure = false;
    FrameWorker::start(
        "thrum-v1-render",
        move |job| {
            let job_start = Instant::now();
            match job {
                RenderJob::Frame {
                    sequence,
                    target_media_seconds,
                    extent,
                    surface_generation,
                    time_of_day,
                    camera_pitch,
                    camera_yaw,
                } => {
                    let mut target_status = None;
                    let mut acquire_wall = None;
                    let mut surface_status = SurfaceStatus::NotAttempted;
                    let report = (|| {
                        if let Some(time_of_day) = time_of_day {
                            host.set_time_of_day(time_of_day)?;
                        }
                        if let Some(camera_pitch) = camera_pitch {
                            host.set_camera_pitch(camera_pitch)?;
                        }
                        if let Some(camera_yaw) = camera_yaw {
                            host.set_camera_yaw(camera_yaw)?;
                        }
                        let surface_current =
                            active_surface_generation.load(Ordering::Acquire) == surface_generation;
                        if !surface_current {
                            needs_reconfigure = true;
                        }
                        let surface_ready = surface_current
                            && !extent.contains(&0)
                            && (extent == current_extent && !needs_reconfigure
                                || host.pending_frames() == 0);
                        if surface_ready && (extent != current_extent || needs_reconfigure) {
                            host.resize_surface(extent)?;
                            surface_config.width = extent[0];
                            surface_config.height = extent[1];
                            surface_owner.configure(&device, &surface_config);
                            current_extent = extent;
                            needs_reconfigure = false;
                        }
                        // Surface preparation can consume the forecast. Sample
                        // both clocks only after it has finished.
                        let now = origin.elapsed();
                        let lead = display_target_lead(target_media_seconds, CACurrentMediaTime());
                        target_status = Some(lead.status);
                        host.set_presentation_lead(lead.duration);
                        if !surface_ready {
                            surface_status = if surface_current {
                                SurfaceStatus::Unavailable
                            } else {
                                SurfaceStatus::NeedsReconfigure
                            };
                            return host.drive(now, None);
                        }
                        let advanced = host.advance(now)?;
                        if !advanced.can_acquire_serial() {
                            return advanced.submit(None);
                        }
                        // The scene is encoded before requesting a drawable.
                        // Missing presentation sheds only this optional view.
                        let encoded = advanced.encode_scene()?;
                        if active_surface_generation.load(Ordering::Acquire) != surface_generation {
                            needs_reconfigure = true;
                            surface_status = SurfaceStatus::NeedsReconfigure;
                            return Ok(encoded.skip());
                        }
                        let acquire_start = Instant::now();
                        let acquired = surface_owner.get_current_texture();
                        acquire_wall = Some(acquire_start.elapsed());
                        let frame = match acquired {
                            CurrentSurfaceTexture::Success(frame) => frame,
                            CurrentSurfaceTexture::Suboptimal(frame) => {
                                needs_reconfigure = true;
                                frame
                            }
                            CurrentSurfaceTexture::Timeout => {
                                surface_status = SurfaceStatus::Unavailable;
                                return Ok(encoded.skip());
                            }
                            CurrentSurfaceTexture::Occluded => {
                                surface_status = SurfaceStatus::Occluded;
                                return Ok(encoded.skip());
                            }
                            CurrentSurfaceTexture::Outdated
                            | CurrentSurfaceTexture::Lost
                            | CurrentSurfaceTexture::Validation => {
                                needs_reconfigure = true;
                                surface_status = SurfaceStatus::NeedsReconfigure;
                                return Ok(encoded.skip());
                            }
                        };
                        if active_surface_generation.load(Ordering::Acquire) != surface_generation {
                            needs_reconfigure = true;
                            surface_status = SurfaceStatus::NeedsReconfigure;
                            return Ok(encoded.skip());
                        }
                        let view = frame.texture.create_view(&TextureViewDescriptor::default());
                        let surface = SurfaceOutput {
                            view: &view,
                            extent,
                        };
                        let report = encoded.submit(Some(&surface))?;
                        if report.surface_presented {
                            queue.present(frame);
                            surface_status = SurfaceStatus::Presented;
                        } else {
                            surface_status = SurfaceStatus::Unavailable;
                        }
                        Ok(report)
                    })();
                    let mut samples = [None; FrameHost::MAX_IN_FLIGHT];
                    for sample in &mut samples {
                        match host.poll_completed() {
                            Ok(next) => *sample = next,
                            Err(error) => {
                                return RenderReply {
                                    sequence,
                                    outcome: RenderOutcome::Frame(Err(error)),
                                    samples,
                                    counters: FrameCounters::read(&host),
                                    worker_wall: job_start.elapsed(),
                                    target_status,
                                    acquire_wall,
                                    surface_status,
                                    surface_extent: current_extent,
                                };
                            }
                        }
                        if sample.is_none() {
                            break;
                        }
                    }
                    RenderReply {
                        sequence,
                        outcome: RenderOutcome::Frame(report),
                        samples,
                        counters: FrameCounters::read(&host),
                        worker_wall: job_start.elapsed(),
                        target_status,
                        acquire_wall,
                        surface_status,
                        surface_extent: current_extent,
                    }
                }
                RenderJob::Finish => {
                    let result = (|| {
                        let mut samples = Vec::with_capacity(FrameHost::MAX_IN_FLIGHT * 2);
                        loop {
                            while let Some(sample) = host.poll_completed()? {
                                samples.push(sample);
                            }
                            if host.pending_frames() == 0 {
                                break;
                            }
                            device
                                .poll(wgpu::PollType::Wait {
                                    submission_index: None,
                                    timeout: Some(Duration::from_secs(10)),
                                })
                                .map_err(|error| format!("GPU drain: {error:?}"))?;
                        }
                        Ok::<_, String>(samples)
                    })();
                    RenderReply {
                        sequence: 0,
                        outcome: RenderOutcome::Finish(result),
                        samples: [None; FrameHost::MAX_IN_FLIGHT],
                        counters: FrameCounters::read(&host),
                        worker_wall: job_start.elapsed(),
                        target_status: None,
                        acquire_wall: None,
                        surface_status: SurfaceStatus::NotAttempted,
                        surface_extent: current_extent,
                    }
                }
            }
        },
        move || {
            let _ = signal.send_event(UserEvent::RenderDone);
        },
    )
    .map_err(|error| format!("render worker: {error}"))
}
