//! Record submitted SDR frames and their own timestamps without a window.
//! Capture I/O never drives the clock. Video plays at the world rate even
//! when readback or the encoder takes longer than a tick.

use std::{
    fs::File,
    io::{self, Write},
    num::NonZeroU32,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use thrum_core::ClockConfig;
use thrum_render::present::SurfaceProfile;
use v1_game::{FrameHost, FrameSample, SurfaceOutput, controls::GameControls};

pub(super) const PROFILE: SurfaceProfile = SurfaceProfile {
    format: wgpu::TextureFormat::Bgra8Unorm,
    extent: [1920, 1080],
};
const TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn requested_frames() -> Result<Option<NonZeroU32>, String> {
    match std::env::var("THRUM_CAPTURE_MOTION_FRAMES") {
        Ok(value) => value.parse().map(Some).map_err(|error| {
            format!("THRUM_CAPTURE_MOTION_FRAMES needs a positive integer: {error}")
        }),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(super) fn record(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    host: &mut FrameHost,
    directory: &Path,
    frames: NonZeroU32,
    cooked_generation: Option<u64>,
) -> Result<(), String> {
    apply_initial_parameters(host)?;
    let pan = CameraPan::from_env(host.camera_orientation()[1], frames)?;
    let rate = ClockConfig::default().tick_hz;
    let target = CaptureTarget::new(device, PROFILE.extent);
    let surface = SurfaceOutput {
        view: &target.view,
        extent: PROFILE.extent,
    };
    let mut log =
        File::create(directory.join("frames.jsonl")).map_err(|error| error.to_string())?;
    write_json_line(
        &mut log,
        &serde_json::json!({
            "extent": PROFILE.extent, "pixel_format": "bgra", "frames_requested": frames,
            "simulation_hz": rate, "video_hz": rate, "display_presented": false,
            "initial_cooked_generation": cooked_generation,
        }),
    )?;
    let mut pixels = io::stdout().lock();
    let mut time = Duration::ZERO;
    host.resume(time)?;
    for index in 0..frames.get() {
        if let Some(pan) = pan {
            host.set_camera_yaw(pan.yaw(index))?;
        }
        let tick = host.tick();
        // Use the existing clock's nanosecond rounding at the next boundary.
        time += host.next_due_in().ok_or("motion capture clock stopped")?;
        let frame = host.drive(time, Some(&surface))?;
        if !frame.surface_presented
            || frame.dropped_view
            || frame.overload.is_some()
            || frame.completed_ticks != 1
            || frame.queued_ticks != 0
            || frame.tick != tick + 1
        {
            return Err(format!(
                "motion frame {index} did not submit one fresh update: {frame:?}"
            ));
        }
        let sample = completed_sample(host, index, frame.tick)?;
        let (Some(pass_ns), Some(blit_ns)) = (sample.gpu_pass_ns, sample.gpu_present_ns) else {
            return Err(format!("motion frame {index} lacks GPU timestamps"));
        };
        target.write_pixels(device, queue, &mut pixels)?;
        write_json_line(
            &mut log,
            &serde_json::json!({
                "frame": index, "tick": sample.tick, "submission": sample.submission_order,
                "simulation_ns": time.as_nanos(), "alpha": sample.presentation_alpha,
                "committed_solar_hours": host.time_of_day() * 24.0,
                "rendered_solar_hours": host.rendered_time_of_day(sample.presentation_alpha) * 24.0,
                "camera_yaw_radians": host.camera_orientation()[1], "gpu_pass_ns": pass_ns,
                "gpu_blit_ns": blit_ns, "gpu_pass_sum_ns": pass_ns.iter().sum::<f64>() + blit_ns,
                "visual_revision": sample.visual_revision,
            }),
        )?;
    }
    pixels.flush().map_err(|error| error.to_string())
}

#[derive(Clone, Copy)]
struct CameraPan {
    start: f32,
    end: f32,
    intervals: NonZeroU32,
}

impl CameraPan {
    fn from_env(start: f32, frames: NonZeroU32) -> Result<Option<Self>, String> {
        let text = match std::env::var("THRUM_CAPTURE_PAN_TO_DEGREES") {
            Ok(text) => text,
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let degrees: f32 = text
            .parse()
            .map_err(|error| format!("invalid motion pan endpoint: {error}"))?;
        if !(-180.0..=180.0).contains(&degrees) {
            return Err("motion pan endpoint needs degrees in [-180, 180]".to_owned());
        }
        let intervals =
            NonZeroU32::new(frames.get() - 1).ok_or("motion pan needs at least two frames")?;
        Ok(Some(Self {
            start,
            end: degrees.to_radians(),
            intervals,
        }))
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "camera yaw uses f32, and subpixel rounding at very long capture lengths is acceptable"
    )]
    fn yaw(self, index: u32) -> f32 {
        if index == self.intervals.get() {
            return self.end;
        }
        let fraction = index as f32 / self.intervals.get() as f32;
        (self.end - self.start).mul_add(fraction, self.start)
    }
}

fn apply_initial_parameters(host: &mut FrameHost) -> Result<(), String> {
    let Some(path) = std::env::var_os("THRUM_PARAMS") else {
        return Ok(());
    };
    let path = Path::new(&path);
    let refused = |error: String| format!("{}: {error}", path.display());
    let text = std::fs::read_to_string(path).map_err(|error| refused(error.to_string()))?;
    let mut controls = GameControls::new().map_err(|error| error.to_string())?;
    controls.observe_camera(host.camera_orientation());
    let report = controls
        .model_mut()
        .apply_file(&text)
        .map_err(|error| refused(error.to_string()))?;
    if !report.problems.is_empty() {
        let problems: Vec<String> = report.problems.iter().map(ToString::to_string).collect();
        return Err(refused(problems.join(", ")));
    }
    let edits = controls.take_edits();
    if edits.paused == Some(true) {
        return Err("motion capture requires a running world".to_owned());
    }
    if let Some(value) = edits.time_of_day {
        host.set_time_of_day(value)?;
    }
    if let Some(value) = edits.camera_yaw {
        host.set_camera_yaw(value)?;
    }
    if let Some(value) = edits.camera_pitch {
        host.set_camera_pitch(value)?;
    }
    if let Some(overlay) = edits.display_overlay() {
        host.set_display_overlay(overlay);
    }
    Ok(())
}

pub(super) fn completed_sample(
    host: &mut FrameHost,
    index: u32,
    tick: u64,
) -> Result<FrameSample, String> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(sample) = host.poll_completed()? {
            if sample.tick != tick || sample.timing_fault {
                return Err(format!(
                    "invalid GPU timing at motion frame {index}: {sample:?}"
                ));
            }
            return Ok(sample);
        }
        if Instant::now() >= deadline {
            return Err(format!("motion frame {index} did not complete"));
        }
        std::thread::sleep(Duration::from_micros(200));
    }
}

pub(super) fn write_json_line(log: &mut File, value: &serde_json::Value) -> Result<(), String> {
    serde_json::to_writer(&mut *log, value).map_err(|error| error.to_string())?;
    log.write_all(b"\n").map_err(|error| error.to_string())
}

/// An offscreen SDR output target the game submits to, with a readback buffer.
pub(super) struct CaptureTarget {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    staging: wgpu::Buffer,
    extent: [u32; 2],
}

impl CaptureTarget {
    pub(super) fn new(device: &wgpu::Device, extent: [u32; 2]) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("motion capture SDR output"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PROFILE.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion capture readback"),
            size: u64::from(extent[0] * 4) * u64::from(extent[1]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            staging,
            extent,
        }
    }

    /// Copy the submitted output and write its BGRA rows to `output`.
    pub(super) fn write_pixels(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output: &mut impl Write,
    ) -> Result<(), String> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.extent[0] * 4),
                    rows_per_image: Some(self.extent[1]),
                },
            },
            self.texture.size(),
        );
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&self.staging, wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(TIMEOUT),
            })
            .map_err(|error| error.to_string())?;
        receive
            .try_recv()
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        let mapped = self
            .staging
            .slice(..)
            .get_mapped_range()
            .map_err(|error| error.to_string())?;
        let result = output.write_all(&mapped).map_err(|error| error.to_string());
        drop(mapped);
        self.staging.unmap();
        result
    }
}
