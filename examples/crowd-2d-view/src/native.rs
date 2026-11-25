use std::{
    env,
    fs::{self, File},
    path::Path,
    thread,
    time::{Duration, Instant},
};

use crowd_2d_view::{CrowdView, proof};
use serde_json::json;
use thrum_app::{
    InlineWorld,
    capture_writer::{ImageWriter, SpoolEncoding},
    offscreen::Offscreen,
};
use thrum_render::{
    capabilities::{DeviceRecipe, OpenedDevice},
    runtime::CaptureLimits,
};

mod band;

type Failure = Box<dyn std::error::Error>;

pub(super) fn proof() -> Result<(), Failure> {
    let [action, count, width, height, view, output]: [String; 6] = env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "expected ACTION COUNT WIDTH HEIGHT whole|close OUTPUT")?;
    let count = count.parse()?;
    let extent = [width.parse()?, height.parse()?];
    let output = Path::new(&output);
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let OpenedDevice {
        device,
        queue,
        capabilities,
        ..
    } = pollster::block_on(DeviceRecipe::default().open(&instance, None))?;
    if capabilities.adapter().backend != wgpu::Backend::Metal {
        return Err("Crowd requires a Metal adapter".into());
    }
    let game = CrowdView::new(count, view == "close")?;
    if matches!(action.as_str(), "fixed" | "realtime") {
        let mut runtime = Offscreen::new(game, &device, &capabilities, &queue, extent)?;
        return capture(&mut runtime, &action, output);
    }
    if action == "band" {
        return band::record(&device, &queue, &capabilities, output);
    }
    let costs = match action.as_str() {
        "crowd" => false,
        "crowd-costs" => true,
        _ => return Err(format!("unknown Crowd action {action}").into()),
    };
    let run_id = env::var("THRUM_CROWD_RUN_ID")?;
    let mut runtime = Offscreen::new(game.exact_steps(), &device, &capabilities, &queue, extent)?;
    runtime.set_timing_requested(costs);
    let passes = proof::passes(runtime.host())?;
    let ticks = if costs {
        240
    } else if count == 10_000 {
        36_000
    } else {
        3600
    };
    let mut publications = Vec::with_capacity(ticks);
    let mut samples = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let report = runtime.step()?;
        if !report.submitted {
            return Err(format!("missing submission at tick {}", report.tick).into());
        }
        let digest = !costs || report.tick.is_multiple_of(120);
        let mut publication = proof::publication(runtime.host(), digest);
        publication["run_id"] = json!(run_id);
        publication["submission"] = json!(runtime.last_submission());
        publications.push(publication);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(sample) = runtime.completed()? {
                let mut sample = proof::sample(&sample);
                sample["run_id"] = json!(run_id);
                samples.push(sample);
                break;
            }
            if Instant::now() >= deadline {
                return Err(format!("tick {} did not retire", report.tick).into());
            }
            thread::sleep(Duration::from_micros(200));
        }
    }
    fs::write(
        output,
        serde_json::to_vec(&json!({
            "workload": "retained Crowd", "target": "mac-metal", "count": count, "extent": extent,
            "run_id": run_id, "passes": passes, "adapter": format!("{:?}", capabilities.adapter()),
            "recipe": format!("{:?}", capabilities.recipe()), "features": format!("{:?}", device.features()), "publications": publications, "samples": samples,
        }))?,
    )?;
    Ok(())
}

fn capture(runtime: &mut Offscreen<CrowdView>, mode: &str, output: &Path) -> Result<(), Failure> {
    fs::create_dir_all(output)?;
    let realtime = mode == "realtime";
    let writer = ImageWriter::start_with_frame_limit(
        File::create(output.join("frames.spool"))?,
        24,
        4 * 1024 * 1024 * 1024,
        SpoolEncoding::ZstdDelta,
        1600,
    )?;
    runtime.set_timing_requested(false);
    if !realtime {
        // Select the same held motion phase for both viewpoints.
        runtime.step_at(Duration::ZERO)?;
        for tick in 1..=360 {
            runtime.step_at(Duration::from_nanos(tick * 16_666_667))?;
        }
        runtime.host_mut().world_mut().pause()?;
        while runtime.pending_frames() > 0 {
            runtime.completed()?;
            thread::sleep(Duration::from_micros(200));
        }
    }
    runtime.enable_capture_with_limits(
        1,
        CaptureLimits {
            staging_slots: if realtime { 6 } else { 1 },
            cpu_buffers: if realtime { 24 } else { 2 },
            byte_allowance: 512 * 1024 * 1024,
        },
    )?;
    let start = Instant::now();
    let mut frames = Vec::new();
    let mut opportunities = Vec::new();
    let mut rejected = 0;
    let interval = Duration::from_nanos(16_666_667);
    let opportunities_count = if realtime { 1225 } else { 1 };
    for index in 0..opportunities_count {
        let target = start + interval * index;
        while Instant::now() < target
            || runtime
                .capture_usage()
                .is_some_and(|(gpu, cpu)| gpu >= 5 || cpu >= 22)
        {
            drain(runtime, &writer, &mut frames, &mut rejected)?;
            thread::sleep(Duration::from_micros(350));
            if start.elapsed() > Duration::from_secs(90) {
                return Err("capture admission deadline".into());
            }
        }
        let now = start.elapsed();
        let report = if realtime {
            runtime.step_at(now)?
        } else {
            runtime.step_at(Duration::from_nanos(360 * 16_666_667))?
        };
        opportunities.push(json!({"request_ns": now.as_nanos(), "submitted": report.submitted,
            "submission": report.submitted.then(|| runtime.last_submission()).flatten(), "tick": report.tick}));
        drain(runtime, &writer, &mut frames, &mut rejected)?;
    }
    let expected = opportunities
        .iter()
        .filter(|row| row["submitted"] == true)
        .count();
    let deadline = Instant::now() + Duration::from_secs(15);
    while frames.len() < expected && Instant::now() < deadline {
        drain(runtime, &writer, &mut frames, &mut rejected)?;
        thread::sleep(Duration::from_millis(1));
    }
    let writer = writer.finish(Duration::from_secs(30))?;
    let omitted = runtime.omissions();
    let hashes = writer.frame_hashes;
    fs::write(
        output.join("runtime.json"),
        serde_json::to_vec(&json!({
            "target": "mac-metal", "workload": "retained Crowd", "mode": mode,
            "clock": "monotonic elapsed host time", "encoding": "ZstdDelta",
            "images": frames, "hashes": hashes, "opportunities": opportunities, "omissions": omitted,
            "writer_omissions": rejected, "expected": expected,
        }))?,
    )?;
    if expected == 0
        || frames.len() != expected
        || hashes.len() != expected
        || omitted != 0
        || rejected != 0
    {
        return Err("not every submitted image was retained".into());
    }
    Ok(())
}

fn drain(
    runtime: &mut Offscreen<CrowdView>,
    writer: &ImageWriter,
    frames: &mut Vec<serde_json::Value>,
    rejected: &mut usize,
) -> Result<(), Failure> {
    while runtime.completed()?.is_some() {}
    while let Some(image) = writer.recycled() {
        runtime.recycle(image);
    }
    while let Some(image) = runtime.image()? {
        frames.push(proof::image_sample(&image));
        if let Some(image) = writer.submit(image) {
            *rejected += 1;
            runtime.recycle(image);
        }
    }
    Ok(())
}
