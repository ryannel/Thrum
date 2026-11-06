//! Game callback bound to the engine's bounded native frame worker.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use objc2_quartz_core::CACurrentMediaTime;
use thrum_render::frame::{FrameWorker, TargetLeadStatus, display_target_lead};
use v1_game::{DriveReport, FrameHost, FrameSample, SurfaceOutput, ThreadedFrameHost};
use wgpu::{SurfaceTexture, TextureViewDescriptor};
use winit::event_loop::EventLoopProxy;

use super::UserEvent;

pub(super) enum RenderJob {
    Frame {
        sequence: u64,
        target_media_seconds: f64,
        frame: Option<(SurfaceTexture, [u32; 2])>,
        time_of_day: Option<f32>,
        camera_pitch: Option<f32>,
    },
    Finish,
}

#[derive(Clone, Copy, Default)]
pub(super) struct ReadyState {
    pub(super) ready_view: bool,
    pub(super) current_scene: bool,
    pub(super) reusable_view: bool,
    pub(super) pending_frames: usize,
    pub(super) dropped_views: u64,
    pub(super) dropped_stale_views: u64,
    pub(super) reused_views: u64,
    pub(super) dropped_surface: u64,
    pub(super) dropped_reports: u64,
}

impl ReadyState {
    pub(super) fn read(host: &ThreadedFrameHost) -> Self {
        Self {
            ready_view: host.has_ready_view() || host.has_serial_view(),
            current_scene: host.has_current_scene_for_display(),
            reusable_view: host.has_reusable_view(),
            pending_frames: host.pending_frames(),
            dropped_views: host.dropped_views(),
            dropped_stale_views: host.dropped_stale_views(),
            reused_views: host.reused_views(),
            dropped_surface: host.dropped_surface(),
            dropped_reports: host.dropped_reports(),
        }
    }

    pub(super) fn can_present(self) -> bool {
        self.ready_view || self.current_scene || self.reusable_view
    }
}

pub(super) enum RenderOutcome {
    Frame(Result<DriveReport, String>),
    Finish(Result<Vec<FrameSample>, String>),
}

pub(super) struct RenderReply {
    pub(super) sequence: u64,
    pub(super) outcome: RenderOutcome,
    pub(super) samples: [Option<FrameSample>; FrameHost::MAX_IN_FLIGHT],
    pub(super) ready: ReadyState,
    pub(super) worker_wall: Duration,
    pub(super) target_status: Option<TargetLeadStatus>,
}

pub(super) fn start(
    mut host: ThreadedFrameHost,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_owner: Arc<wgpu::Surface<'static>>,
    initial_extent: [u32; 2],
    origin: Instant,
    wake: &EventLoopProxy<UserEvent>,
) -> Result<FrameWorker<RenderJob, RenderReply>, String> {
    let signal = wake.clone();
    let mut current_extent = initial_extent;
    FrameWorker::start(
        "thrum-v1-render",
        move |job| {
            let job_start = Instant::now();
            // The actor may be detached during a platform teardown. Keep the
            // surface registered for any drawable still inside its job.
            let _surface_owner = &surface_owner;
            match job {
                RenderJob::Frame {
                    sequence,
                    target_media_seconds,
                    frame,
                    time_of_day,
                    camera_pitch,
                } => {
                    let mut target_status = None;
                    let report = (|| {
                        if let Some(time_of_day) = time_of_day {
                            host.set_time_of_day(time_of_day)?;
                        }
                        if let Some(camera_pitch) = camera_pitch {
                            host.set_camera_pitch(camera_pitch)?;
                        }
                        let view = frame.as_ref().map(|(frame, _)| {
                            frame.texture.create_view(&TextureViewDescriptor::default())
                        });
                        let surface =
                            view.as_ref()
                                .zip(frame.as_ref())
                                .map(|(view, (_, extent))| SurfaceOutput {
                                    view,
                                    extent: *extent,
                                });
                        if let Some((_, extent)) = &frame
                            && *extent != current_extent
                        {
                            host.resize_surface(*extent)?;
                            current_extent = *extent;
                        }
                        // Sample both clocks after preparation. The target may
                        // expire while this bounded job waits for its worker.
                        let now = origin.elapsed();
                        let lead = display_target_lead(target_media_seconds, CACurrentMediaTime());
                        target_status = Some(lead.status);
                        host.set_presentation_lead(lead.duration);
                        let report = host.drive(now, surface.as_ref())?;
                        if report.surface_presented {
                            queue.present(frame.ok_or("surface pass without drawable")?.0);
                        }
                        Ok::<_, String>(report)
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
                                    ready: ReadyState::read(&host),
                                    worker_wall: job_start.elapsed(),
                                    target_status,
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
                        ready: ReadyState::read(&host),
                        worker_wall: job_start.elapsed(),
                        target_status,
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
                        ready: ReadyState::read(&host),
                        worker_wall: job_start.elapsed(),
                        target_status: None,
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
