//! Record submitted SDR frames and their own timestamps without a window.
//! Capture I/O never drives the clock. Video plays at the world rate even
//! when readback or the encoder takes longer than a tick.

use std::{
    fs::File,
    io::{self, Write},
    num::NonZeroU32,
    path::Path,
    time::{Duration, Instant},
};

use thrum_core::ClockConfig;
use thrum_render::{present::SurfaceProfile, runtime::FrameSample};

use super::{Driver, SceneCaptureRequest};
use crate::{GameHost, controls::GameControls, host::Runtime};

pub(super) const PROFILE: SurfaceProfile = SurfaceProfile {
    format: wgpu::TextureFormat::Bgra8Unorm,
    extent: [1920, 1080],
};
const TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn requested_frames(
    request: &SceneCaptureRequest,
) -> Result<Option<NonZeroU32>, String> {
    request
        .motion_frames
        .as_deref()
        .map(|value| {
            value.parse().map_err(|error| {
                format!("THRUM_CAPTURE_MOTION_FRAMES needs a positive integer: {error}")
            })
        })
        .transpose()
}

pub(super) fn record(
    driver: &mut Driver,
    request: &SceneCaptureRequest,
    directory: &Path,
    frames: NonZeroU32,
    cooked_generation: Option<u64>,
) -> Result<(), String> {
    apply_initial_parameters(driver.host_mut(), request.initial_params.as_deref())?;
    let pan = CameraPan::requested(
        request.pan_to_degrees.as_deref(),
        driver.host().camera_orientation()[1],
        frames,
    )?;
    let rate = ClockConfig::default().tick_hz;
    let target = CaptureTarget::new(PROFILE.extent);
    driver.host_mut().enable_submitted_capture(1)?;
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
    driver.host_mut().world_mut().resume(time)?;
    for index in 0..frames.get() {
        if let Some(pan) = pan {
            driver.host_mut().set_camera_yaw(pan.yaw(index))?;
        }
        let tick = driver.host().world().tick();
        // Use the existing clock's nanosecond rounding at the next boundary.
        time += driver
            .host()
            .world()
            .next_due_in()
            .ok_or("motion capture clock stopped")?;
        let frame = driver.step_at(time)?;
        let host = driver.host_mut();
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
        let (Some(pass_ns), Some(blit_ns)) = (sample.gpu_pass_ns.as_ref(), sample.gpu_present_ns)
        else {
            return Err(format!("motion frame {index} lacks GPU timestamps"));
        };
        target.write_pixels(host, sample.submission_order, &mut pixels)?;
        write_json_line(
            &mut log,
            &serde_json::json!({
                "frame": index, "tick": sample.tick, "submission": sample.submission_order,
                "simulation_ns": time.as_nanos(), "alpha": sample.presentation_alpha,
                "committed_solar_hours": host.time_of_day() * 24.0,
                "rendered_solar_hours": host.rendered_time_of_day(sample.presentation_alpha) * 24.0,
                "camera_yaw_radians": host.camera_orientation()[1], "gpu_pass_ns": pass_ns.values(),
                "gpu_blit_ns": blit_ns, "gpu_pass_sum_ns": pass_ns.sum() + blit_ns,
                "visual_revision": sample.visual_revision,
            }),
        )?;
    }
    driver.host_mut().disable_submitted_capture();
    pixels.flush().map_err(|error| error.to_string())
}

#[derive(Clone, Copy)]
struct CameraPan {
    start: f32,
    end: f32,
    intervals: NonZeroU32,
}

impl CameraPan {
    fn requested(
        text: Option<&str>,
        start: f32,
        frames: NonZeroU32,
    ) -> Result<Option<Self>, String> {
        let Some(text) = text else {
            return Ok(None);
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

fn apply_initial_parameters(host: &mut Runtime, path: Option<&Path>) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
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
    host: &mut Runtime,
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

/// The submitted images of an ordinary frame at one render extent. Readback remains
/// renderer-owned.
pub(super) struct CaptureTarget {
    extent: [u32; 2],
}

impl CaptureTarget {
    pub(super) fn new(extent: [u32; 2]) -> Self {
        Self { extent }
    }

    /// Fixed-time tools may wait between explicit samples. This collects the
    /// same submitted graph output; it creates no copy encoder or queue submit.
    pub(super) fn write_pixels(
        &self,
        host: &mut Runtime,
        submission: u64,
        output: &mut impl Write,
    ) -> Result<(), String> {
        let mut image = self.submitted_image(
            host,
            submission,
            thrum_render::synthetic::CaptureSource::Sdr,
        )?;
        // Preserve the legacy BGRA byte stream and route PPM oracles.
        for pixel in image.rgba.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        let result = output
            .write_all(&image.rgba)
            .map_err(|error| error.to_string());
        host.recycle_submitted_image(image);
        result
    }

    pub(super) fn submitted_image(
        &self,
        host: &mut Runtime,
        submission: u64,
        source: thrum_render::synthetic::CaptureSource,
    ) -> Result<thrum_render::runtime::SubmittedImage, String> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(image) = host.try_submitted_image()? {
                if image.sample.submission_order != submission
                    || image.pixel_source != source
                    || image.rgba.len()
                        != self.extent[0] as usize
                            * self.extent[1] as usize
                            * source.bytes_per_pixel() as usize
                {
                    return Err("fixed-time submitted image identity mismatch".to_owned());
                }
                return Ok(image);
            }
            drop(host.poll_completed()?);
            if Instant::now() >= deadline {
                return Err("fixed-time submitted image deadline".to_owned());
            }
            std::thread::sleep(Duration::from_micros(200));
        }
    }
}
