//! V1 construction diagnostic: one committed 10k-entity simulation endpoint
//! feeds the real offscreen renderer. No window or shipping runtime is implied.

use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

use thrum_core::{
    StableId, WorldEpoch,
    sim::{ComponentEntity, Position, SimConfig, Simulation, Velocity},
};
use thrum_render::{
    extract::{RenderColumns, RenderExtractor, SourceTick},
    synthetic::{OffscreenTarget, PreparedSynthetic, SyntheticScene},
    timing::GpuTimings,
};

const ENTITY_COUNT: usize = 10_000;
const MOVING_COUNT: usize = 1_500;

fn simulation() -> Result<Simulation, String> {
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("nonzero fixture tick rate"),
            entity_capacity: ENTITY_COUNT,
            peer_capacity: 4,
            actions_per_frame: 8,
            movement_speed: 6.0,
        },
    )
    .map_err(|error| format!("simulation setup: {error:?}"))?;
    for index in 0..ENTITY_COUNT {
        let x = -0.9 + (index % 100) as f64 * 0.018 + 0.009;
        let y = -0.9 + (index / 100) as f64 * 0.018 + 0.009;
        sim.seed(ComponentEntity {
            id: StableId(index as u128),
            position: Position([x, y, 0.5]),
            velocity: Velocity([if index < MOVING_COUNT { 0.05 } else { 0.0 }, 0.0, 0.0]),
            health: 100,
            ownership: None,
            visual: Some((index % 2) as u64),
        })
        .map_err(|error| format!("seed entity {index}: {error:?}"))?;
    }
    Ok(sim)
}

fn extract(extractor: &mut RenderExtractor, sim: &Simulation) -> usize {
    let report = extractor.extract(
        SourceTick {
            epoch: 1,
            tick: sim.tick(),
        },
        [0.0; 3],
        |sink| {
            sim.visit_components(|view| {
                if let Some(visual) = view.visual {
                    sink.push(view.id.0, view.position.0, visual);
                }
            });
        },
    );
    assert_eq!(report.accepted, ENTITY_COUNT, "all fixture visuals fit");
    assert_eq!(
        report.dropped_invalid + report.dropped_capacity + report.dropped_duplicate,
        0
    );
    report.history_reused
}

fn scene_from_frame(frame: &RenderColumns) -> Result<(SyntheticScene, usize), String> {
    let mut scene = SyntheticScene::grid(u32::try_from(frame.len()).expect("10k fits u32"))
        .map_err(|error| format!("synthetic scene: {error:?}"))?;
    let mut moved = 0;
    for row in 0..frame.len() {
        let index = usize::try_from(frame.ids()[row])
            .map_err(|_| "stable fixture ID exceeds usize".to_owned())?;
        let instance = scene
            .instances
            .get_mut(index)
            .ok_or_else(|| format!("stable fixture ID {index} outside scene"))?;
        let current = frame.current()[row];
        let previous = frame.previous()[row];
        instance.current[3][..3].copy_from_slice(&current);
        instance.previous[3][..3].copy_from_slice(&previous);
        instance.ids[1] = u32::try_from(frame.visuals()[row])
            .map_err(|_| "fixture material index exceeds u32".to_owned())?;
        moved += usize::from((current[0] - previous[0]).abs() > 1e-6);
    }
    Ok((scene, moved))
}

async fn run() -> Result<String, String> {
    let mut sim = simulation()?;
    let mut extractor = RenderExtractor::new(ENTITY_COUNT);
    for _ in 0..59 {
        sim.step(&[])
            .map_err(|error| format!("warmup tick: {error:?}"))?;
    }
    extract(&mut extractor, &sim);
    let start = Instant::now();
    sim.step(&[])
        .map_err(|error| format!("measured tick: {error:?}"))?;
    let tick_time = start.elapsed();
    let start = Instant::now();
    let history_reused = extract(&mut extractor, &sim);
    let extract_time = start.elapsed();
    let (scene, moved) = scene_from_frame(extractor.frame())?;
    if history_reused != ENTITY_COUNT || moved != MOVING_COUNT {
        return Err(format!(
            "history/motion mismatch: {history_reused} reused, {moved} moved"
        ));
    }

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|error| format!("native GPU adapter: {error:?}"))?;
    let adapter_name = adapter.get_info().name;
    let features = GpuTimings::requested_features(&adapter);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("GPU device: {error:?}"))?;
    let prepared = PreparedSynthetic::prepare(&device, &queue, 1, &scene)
        .await
        .map_err(|error| format!("prepare: {error:?}"))?;
    let target =
        OffscreenTarget::new(&device, 1, 640, 360).map_err(|error| format!("target: {error:?}"))?;
    let mut pixels = target.readback(&device);
    let mut timings = GpuTimings::new(&device, &queue, 1, 4)
        .map_err(|error| format!("GPU timings: {error:?}"))?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let start = Instant::now();
    prepared
        .encode(&mut encoder, 1, &target, Some(&mut timings))
        .map_err(|error| format!("encode scene: {error:?}"))?;
    pixels
        .encode(&mut encoder, &target)
        .map_err(|error| format!("encode readback: {error:?}"))?;
    timings
        .finish_render(1, &encoder)
        .map_err(|error| format!("finish timings: {error:?}"))?;
    let encode_time = start.elapsed();
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .map_err(|error| format!("render poll: {error:?}"))?;
    let rgba = pixels
        .try_collect(1)
        .map_err(|error| format!("pixel collection: {error:?}"))?
        .ok_or_else(|| "pixel readback incomplete".to_owned())?;
    let background = rgba.get(..4).ok_or_else(|| "empty image".to_owned())?;
    let foreground = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel.as_slice() != background)
        .count();
    if foreground == 0 {
        return Err("simulation-driven scene produced no visible geometry".to_owned());
    }
    if let Some(path) = std::env::var_os("THRUM_V1_CAPTURE") {
        let mut ppm = Vec::with_capacity(640 * 360 * 3 + 32);
        ppm.extend_from_slice(b"P6\n640 360\n255\n");
        for pixel in rgba.as_chunks::<4>().0 {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(path, ppm).map_err(|error| format!("capture: {error}"))?;
    }
    let gpu = if timings.is_enabled() {
        let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if !timings
            .resolve_if_ready(1, &mut resolve)
            .map_err(|error| format!("resolve timings: {error:?}"))?
        {
            return Err("GPU timestamps were not ready after render completion".to_owned());
        }
        let submission = queue.submit([resolve.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|error| format!("timestamp poll: {error:?}"))?;
        let samples = timings
            .try_collect(1)
            .map_err(|error| format!("collect timings: {error:?}"))?
            .ok_or_else(|| "GPU timestamp readback incomplete".to_owned())?;
        format!(
            "{} ns",
            samples.iter().map(|sample| sample.nanoseconds).sum::<f64>()
        )
    } else {
        "unavailable".to_owned()
    };
    Ok(format!(
        "adapter={adapter_name}; entities={ENTITY_COUNT}; moving={moved}; tick={:.3} us; extract={:.3} us; encode={:.3} us; GPU passes={gpu}; foreground_pixels={foreground}",
        tick_time.as_secs_f64() * 1e6,
        extract_time.as_secs_f64() * 1e6,
        encode_time.as_secs_f64() * 1e6,
    ))
}

#[expect(
    clippy::print_stdout,
    reason = "this diagnostic CLI reports its measured scene to the caller"
)]
#[expect(
    clippy::print_stderr,
    reason = "this diagnostic CLI reports setup or GPU failures to the caller"
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
