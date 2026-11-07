//! The render owner: device preparation, scene encoding, late acquisition and
//! presentation on one resident thread behind the engine's bounded handoff.

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use thrum_render::{
    capabilities::DeviceCapabilities,
    frame::{FrameWorker, TargetLeadStatus, display_target_lead},
    present::SurfaceProfile,
    runtime::{DriveReport, FrameError},
};
use v1_game::{FrameConfig, FrameHost, SurfaceOutput, ThreadedFrameHost, WorldControls};
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};

use crate::lease::{Leased, SurfaceLease};

pub(super) const THREAD: &str = "thrum-v1-ios-render";
/// Completed samples polled per job; the host retains at most this many frames.
const SAMPLE_SLOTS: usize = 2;
const RENDER_EXTENT: [u32; 2] = [640, 360];

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
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent per-frame witnesses, not a state machine"
)]
pub(super) struct FrameReply {
    pub(super) epoch: u64,
    pub(super) status: SurfaceStatus,
    pub(super) report: Option<DriveReport>,
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
}

pub(super) enum Reply {
    Prepared(Result<Prepared, String>),
    Frame(Result<FrameReply, String>),
    Actor(Option<[f64; 3]>),
}

pub(super) struct Reported {
    pub(super) reply: Reply,
    /// Thread that produced the reply.
    pub(super) thread: Option<String>,
}

struct Gpu {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    host: ThreadedFrameHost,
    format: wgpu::TextureFormat,
    host_extent: [u32; 2],
}

struct Owner {
    lease: Arc<Lease>,
    gpu: Option<Gpu>,
    origin: Instant,
    /// Qualification only: tells the shell the owner now holds the section.
    stalling: Box<dyn Fn() + Send>,
}

pub(super) fn start(
    lease: Arc<Lease>,
    wake: impl FnMut() + Send + 'static,
    stalling: impl Fn() + Send + 'static,
) -> Result<RenderWorker, String> {
    let mut owner = Owner {
        lease,
        gpu: None,
        origin: Instant::now(),
        stalling: Box::new(stalling),
    };
    FrameWorker::start(
        THREAD,
        move |job| Reported {
            reply: owner.work(job),
            thread: thread::current().name().map(str::to_owned),
        },
        wake,
    )
    .map_err(|error| format!("iOS render owner: {error}"))
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
            Job::Frame(request) => Reply::Frame(self.frame(&request).map_err(|e| e.to_string())),
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
        let host = pollster::block_on(FrameHost::new(
            &device,
            &queue,
            FrameConfig {
                render_extent: RENDER_EXTENT,
                surface: Some(SurfaceProfile { format, extent }),
            },
        ))?;
        let (host, controls) = host.into_threaded(self.origin, world_advanced)?;
        let name = adapter.get_info().name;
        self.gpu = Some(Gpu {
            adapter,
            device,
            queue,
            host,
            format,
            host_extent: extent,
        });
        Ok(Prepared {
            controls,
            adapter: name,
            format,
        })
    }

    fn frame(&mut self, request: &FrameRequest) -> Result<FrameReply, FrameError> {
        let lease = Arc::clone(&self.lease);
        let now = self.origin.elapsed();
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(FrameError::InvalidState("iOS render owner is not prepared"))?;
        let lead = display_target_lead(request.target_media_seconds, media_time_now());
        gpu.host.set_presentation_lead(lead.duration);
        let mut reply = FrameReply {
            epoch: request.epoch,
            status: SurfaceStatus::NoView,
            report: None,
            completed: 0,
            surface_samples: 0,
            pending_frames: 0,
            target: lead.status,
            acquired: false,
            presentable_encode: false,
            configured: false,
            stalled: false,
            acquire: Duration::ZERO,
            raced_present: false,
        };
        let usable = !request.extent.contains(&0);
        let ready = if !request.present || !usable {
            reply.status = SurfaceStatus::Denied;
            false
        } else {
            match lease.section(request.epoch, |leased| gpu.configure(leased, request)) {
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
        reply.report = Some(if ready {
            gpu.present(now, &lease, request, &mut reply, &*self.stalling)?
        } else {
            gpu.host.drive(now, None)?
        });
        for _ in 0..SAMPLE_SLOTS {
            let Some(sample) = gpu.host.poll_completed()? else {
                break;
            };
            reply.completed += 1;
            reply.surface_samples += u32::from(sample.surface_presented);
        }
        reply.pending_frames = gpu.host.pending_frames();
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
    ) -> Result<Option<bool>, FrameError> {
        if leased.configured == Some((request.epoch, request.extent)) {
            return Ok(Some(false));
        }
        if self.host.pending_frames() != 0 {
            return Ok(None);
        }
        if request.extent != self.host_extent {
            self.host.resize_surface(request.extent)?;
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

    fn present(
        &mut self,
        now: Duration,
        lease: &Lease,
        request: &FrameRequest,
        reply: &mut FrameReply,
        stalling: &dyn Fn(),
    ) -> Result<DriveReport, FrameError> {
        let advanced = self.host.advance(now)?;
        if !advanced.can_acquire_serial() {
            return advanced.submit(None);
        }
        let encoded = advanced.encode_scene()?;
        if !encoded.can_acquire_surface() {
            return Ok(encoded.skip());
        }
        reply.presentable_encode = true;
        let queue = &self.queue;
        let mut pending = Some(encoded);
        let result = lease.section(request.epoch, |leased| {
            let encoded = pending
                .take()
                .expect("encoded frame enters the section once");
            let acquiring = Instant::now();
            let texture = leased.surface.get_current_texture();
            reply.acquire = acquiring.elapsed();
            let frame = match texture {
                CurrentSurfaceTexture::Success(frame) => frame,
                CurrentSurfaceTexture::Suboptimal(frame) => {
                    leased.configured = None;
                    frame
                }
                CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                    reply.status = SurfaceStatus::Missing;
                    return Ok(encoded.skip());
                }
                CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                    leased.configured = None;
                    reply.status = SurfaceStatus::Reconfigure;
                    return Ok(encoded.skip());
                }
                CurrentSurfaceTexture::Validation => {
                    encoded.skip();
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
                    thread::sleep(stall);
                }
            };
            hold(StallPoint::AfterAcquire, reply);
            if !lease.is_current(request.epoch) {
                reply.status = SurfaceStatus::StaleDropped;
                drop(frame);
                return Ok(encoded.skip());
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
                hold(StallPoint::BeforePresent, reply);
                queue.present(frame);
                reply.status = SurfaceStatus::Presented;
                reply.raced_present = !lease.is_current(request.epoch);
            }
            Ok(report)
        });
        if let Some(report) = result {
            return report;
        }
        reply.status = SurfaceStatus::NoLease;
        Ok(pending
            .take()
            .expect("a refused section leaves the encoded frame")
            .skip())
    }
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
