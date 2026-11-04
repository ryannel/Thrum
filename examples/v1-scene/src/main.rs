//! Paced offscreen V1 scene diagnostic. The clock admits every committed
//! simulation update; rendering consumes the latest complete endpoint.

use std::time::{Duration, Instant};

use thrum_render::timing::GpuTimings;
use v1_scene::FrameHost;

const FRAMES: usize = 600;
const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn percentile<T>(samples: &[T], value: impl Fn(&T) -> f64) -> (f64, f64) {
    let mut values: Vec<_> = samples.iter().map(value).collect();
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[values.len() * 95 / 100])
}

async fn run() -> Result<String, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|error| format!("native GPU adapter: {error:?}"))?;
    let adapter_name = adapter.get_info().name;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: GpuTimings::requested_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|error| format!("GPU device: {error:?}"))?;
    let mut host = FrameHost::new(&device, &queue, None).await?;
    let origin = Instant::now();
    let mut last_call = Duration::ZERO;
    let mut samples = Vec::with_capacity(FRAMES);
    let mut callback_walls = Vec::with_capacity(FRAMES);
    let mut completed_ticks = 0_u64;
    let mut catch_up_frames = 0_usize;
    let mut over_deadline = 0_usize;
    while samples.len() < FRAMES {
        if callback_walls.len() == FRAMES * 4 {
            return Err(
                "bounded GPU host failed to complete 600 frames in 2,400 callbacks".to_owned(),
            );
        }
        let due = host
            .next_due_in()
            .ok_or_else(|| "fixed clock stopped before the target frame count".to_owned())?;
        let since_last_call = origin.elapsed().saturating_sub(last_call);
        std::thread::sleep(due.saturating_sub(since_last_call));
        let now = origin.elapsed();
        let callback_start = Instant::now();
        let report = host.drive(now, None)?;
        while let Some(sample) = host.poll_completed()? {
            if samples.len() < FRAMES {
                samples.push(sample);
            }
        }
        let callback_wall = callback_start.elapsed();
        callback_walls.push(callback_wall);
        last_call = now;
        completed_ticks += u64::from(report.completed_ticks);
        catch_up_frames += usize::from(report.completed_ticks > 1);
        over_deadline += usize::from(callback_wall > Duration::from_nanos(16_666_667));
    }
    let elapsed = origin.elapsed();
    let pixels = host.capture_pixels()?;
    let background = pixels.get(..4).ok_or_else(|| "empty capture".to_owned())?;
    let foreground = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel.as_slice() != background)
        .count();
    if foreground == 0 {
        return Err("running scene has no visible geometry".to_owned());
    }
    if let Some(path) = std::env::var_os("THRUM_V1_CAPTURE") {
        let mut ppm = Vec::with_capacity(WIDTH * HEIGHT * 3 + 32);
        ppm.extend_from_slice(b"P6\n640 360\n255\n");
        for pixel in pixels.as_chunks::<4>().0 {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(path, ppm).map_err(|error| format!("capture: {error}"))?;
    }
    let tick = percentile(&samples, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
    let extract = percentile(&samples, |sample| sample.extract_cpu.as_secs_f64() * 1e6);
    let upload = percentile(&samples, |sample| sample.upload_cpu.as_secs_f64() * 1e6);
    let encode = percentile(&samples, |sample| sample.encode_cpu.as_secs_f64() * 1e6);
    let wall = percentile(&callback_walls, |wall| wall.as_secs_f64() * 1e3);
    let gpu = if samples.iter().all(|sample| sample.gpu_pass_ns.is_some()) {
        let shadow_near = percentile(&samples, |sample| {
            sample.gpu_pass_ns.expect("checked GPU timestamps")[0] / 1e6
        });
        let shadow_far = percentile(&samples, |sample| {
            sample.gpu_pass_ns.expect("checked GPU timestamps")[1] / 1e6
        });
        let forward = percentile(&samples, |sample| {
            sample.gpu_pass_ns.expect("checked GPU timestamps")[2] / 1e6
        });
        let display = percentile(&samples, |sample| {
            sample.gpu_pass_ns.expect("checked GPU timestamps")[3] / 1e6
        });
        format!(
            "GPU shadow near p50/p95={:.3}/{:.3} ms; GPU shadow far p50/p95={:.3}/{:.3} ms; GPU forward p50/p95={:.3}/{:.3} ms; GPU display p50/p95={:.3}/{:.3} ms",
            shadow_near.0,
            shadow_near.1,
            shadow_far.0,
            shadow_far.1,
            forward.0,
            forward.1,
            display.0,
            display.1
        )
    } else {
        "GPU timestamps unavailable".to_owned()
    };
    let completed = completed_ticks;
    let dropped: usize = samples
        .iter()
        .map(|sample| sample.dropped_presentation)
        .sum();
    let uploaded: usize = samples.iter().map(|sample| sample.uploaded_bytes).sum();
    let dropped_light_references = samples[0].dropped_light_references;
    let timing_faults = samples.iter().filter(|sample| sample.timing_fault).count();
    let dropped_views = host.dropped_views();
    let dropped_reports = host.dropped_reports();
    let max_wall = callback_walls
        .iter()
        .copied()
        .max()
        .expect("at least 600 callbacks");
    Ok(format!(
        "adapter={adapter_name}; frames={FRAMES}; elapsed={:.3} s; committed_ticks={completed}; final_tick={}; catch_up_frames={catch_up_frames}; over_deadline={over_deadline}; dropped_views={dropped_views}; dropped_reports={dropped_reports}; timing_faults={timing_faults}; foreground_pixels={foreground}; dropped_presentation={dropped}; dropped_light_references={dropped_light_references}; uploaded_bytes={uploaded}; tick p50/p95={:.3}/{:.3} us; extract p50/p95={:.3}/{:.3} us; upload p50/p95={:.3}/{:.3} us; encode p50/p95={:.3}/{:.3} us; callback CPU p50/p95/max={:.3}/{:.3}/{:.3} ms; {gpu}",
        elapsed.as_secs_f64(),
        host.tick(),
        tick.0,
        tick.1,
        extract.0,
        extract.1,
        upload.0,
        upload.1,
        encode.0,
        encode.1,
        wall.0,
        wall.1,
        max_wall.as_secs_f64() * 1e3,
    ))
}

#[expect(
    clippy::print_stdout,
    reason = "the diagnostic CLI reports its assembled frame measurements"
)]
#[expect(
    clippy::print_stderr,
    reason = "the diagnostic CLI reports setup and GPU failures"
)]
fn main() {
    match pollster::block_on(run()) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("v1-scene: {error}");
            std::process::exit(1);
        }
    }
}
