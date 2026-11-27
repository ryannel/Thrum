//! Capture and cost driver for the shipped water weather selection.
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use thrum_app::{
    offscreen::{
        Offscreen,
        writer::{ImageWriter, SpoolEncoding},
    },
    sky::{ContentStatus, SkyGame},
};
use thrum_render::runtime::FrameSample;
use water_scene::clouds::Selection;

pub(super) fn run(gpu: &thrum_test_support::Gpu, mode: &OsStr) {
    let mode = mode.to_str().expect("capture mode");
    let directory = PathBuf::from(std::env::var_os("THRUM_WATER_OUTPUT").expect("output folder"));
    std::fs::create_dir_all(&directory).expect("capture directory");
    let Some(selection) = Selection::from_variables(|name| std::env::var(name).ok()) else {
        panic!("select the weather first");
    };
    let commit = std::env::var("THRUM_WATER_COMMIT").expect("captured revision");
    let extent = match mode {
        "cost" | "fixed" => [2560, 1440],
        "realtime" => [1280, 720],
        "scout" => [960, 540],
        _ => panic!("unknown weather capture mode {mode}"),
    };
    let enabled = std::env::var_os("THRUM_WATER_CLOUD_CONTROL").is_none();
    let mut record = if mode == "cost" {
        json!({"arms":([false, true].map(|enabled| cost(gpu, selection, extent, enabled)))})
    } else {
        capture(
            gpu,
            selection,
            extent,
            mode == "realtime",
            enabled,
            &directory,
        )
    };
    record["target"] = json!("mac-metal");
    record["extent"] = json!(extent);
    record["commit"] = json!(commit);
    record["mode"] = json!(if mode == "scout" { "fixed" } else { mode });
    record["selection"] = json!(format!("{selection:?}"));
    let record = serde_json::to_vec_pretty(&record).expect("capture report");
    let file = if mode == "scout" { "fixed" } else { mode };
    std::fs::write(directory.join(format!("{file}.json")), record).expect("save report");
}

fn cost(
    gpu: &thrum_test_support::Gpu,
    selection: Selection,
    extent: [u32; 2],
    enabled: bool,
) -> Value {
    let scene = selection.build(true, enabled).expect("paired scene");
    let weather = scene.weather;
    let mut fixture = super::support::Fixture::from_game(gpu, extent, scene.game, scene.report);
    fixture.warm_cooked();
    for i in 0..8 {
        fixture.frame(1.0 + f64::from(i) / 60.0);
    }
    let mut missing = Vec::new();
    let frames: Vec<_> = (8..32)
        .filter_map(|i| {
            let sample = fixture.frame(1.0 + f64::from(i) / 60.0);
            if sample.gpu_pass_ns.is_none() {
                missing.push(format!("{:?}", sample.timing_fault));
                return None;
            }
            Some(timings(&sample))
        })
        .collect();
    assert!(
        frames.len() >= 16,
        "named Metal timings: {missing:?}; {:?}",
        fixture.host.timing_stats()
    );
    json!({
        "enabled":enabled, "capture":false, "frames":frames, "timing_refusals":missing,
        "storage":fixture.host.storage_report(),
        "trace":weather.as_ref().map(|weather| trace_work(&fixture, weather)),
        "cloud_resources":weather.map_or_else(Vec::new, |weather| weather.resource_bytes())
    })
}

fn trace_work(fixture: &super::support::Fixture, weather: &thrum_clouds::WeatherInput) -> Value {
    use thrum_test_support::readback::{self, Mip};

    let sky = weather.sky_products().expect("submitted sky");
    assert_eq!(sky.trace_stride, 1);
    let pixels = readback::texture(
        &fixture.device,
        &fixture.queue,
        &sky.fresh_front,
        Mip::level(0),
        ..,
    );
    let samples: Vec<_> = pixels
        .as_chunks::<8>()
        .0
        .iter()
        .map(|pixel| super::support::half_value(u16::from_le_bytes([pixel[4], pixel[5]])))
        .collect();
    let [width, height] = sky.active_extent;
    assert_eq!(samples.len(), (width * height) as usize);
    let rays = width * height;
    let nonempty = samples.iter().filter(|&&value| value > 0.0).count();
    json!({"rays":rays, "sampled_rays":nonempty,
        "samples_total":samples.iter().copied().map(f64::from).sum::<f64>()})
}

fn timings(sample: &FrameSample) -> Value {
    let gpu = sample.gpu_pass_ns.as_ref().expect("named Metal timings");
    let passes: Vec<_> = gpu
        .passes()
        .iter()
        .map(|pass| json!([pass.name.as_ref(), pass.nanoseconds / 1e6]))
        .collect();
    json!({"tick":sample.tick,"submission":sample.submission_order,"gpu_ms":(gpu.sum()+sample.gpu_present_ns.unwrap_or(0.0))/1e6,
        "passes_ms":passes})
}

fn capture(
    gpu: &thrum_test_support::Gpu,
    selection: Selection,
    extent: [u32; 2],
    realtime: bool,
    enabled: bool,
    directory: &Path,
) -> Value {
    let scene = selection
        .build(true, enabled)
        .expect("selected weather scene");
    let (device, queue, caps) = super::support::device(gpu);
    let mut runtime =
        Offscreen::new(scene.game, &device, &caps, &queue, extent).expect("ordinary game scene");
    runtime.set_timing_requested(false);
    thrum_test_support::wait::until(
        "ordinary cooked water scene",
        Duration::from_secs(40),
        || {
            runtime.step_at(Duration::ZERO).expect("warm-up frame");
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("warm-up completion");
            while runtime.completed().expect("warm-up completion").is_some() {}
            match runtime.game().content_status() {
                Some(ContentStatus::Drawn(_)) => Some(()),
                Some(ContentStatus::Failed(error)) => panic!("{error}"),
                _ => None,
            }
        },
    );
    runtime.enable_capture(1).expect("submitted tap");
    if !realtime {
        runtime.step_at(Duration::ZERO).expect("held submission");
        let image =
            thrum_test_support::wait::until("held water frame", Duration::from_secs(30), || {
                device.poll(wgpu::PollType::Poll).expect("Metal readback");
                runtime.image().expect("submitted image")
            });
        std::fs::write(directory.join("water.rgba"), &image.rgba).expect("capture pixels");
        return json!({"images":[{"file":"water.rgba","submission":image.sample.submission_order,"tick":image.sample.tick,"time":0}]});
    }
    let reconstructor = format!("{:?}", runtime.host().resolved_reconstructor());
    let mut recording = capture_motion(&mut runtime, &device, directory);
    recording["reconstructor"] = json!(reconstructor);
    recording
}

fn capture_motion(
    runtime: &mut Offscreen<SkyGame>,
    device: &wgpu::Device,
    directory: &Path,
) -> Value {
    let spool = std::fs::File::create(directory.join("motion.spool.zst")).expect("lossless spool");
    let writer =
        ImageWriter::start(spool, 12, 4 << 30, SpoolEncoding::ZstdDelta).expect("lossless writer");
    let begin = Instant::now();
    let interval = Duration::from_millis(20_150);
    let mut submitted = 0;
    let mut frames = Vec::new();
    let mut next = Duration::ZERO;
    loop {
        let elapsed = begin.elapsed();
        if elapsed >= next {
            submitted += usize::from(
                runtime
                    .step_at(elapsed)
                    .expect("live water frame")
                    .submitted,
            );
            next = elapsed + Duration::from_millis(34);
        }
        drain(runtime, device, &writer, &mut frames);
        if elapsed >= interval {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    thrum_test_support::wait::until("all submitted images", Duration::from_secs(30), || {
        drain(runtime, device, &writer, &mut frames);
        (frames.len() >= submitted).then_some(())
    });
    let written = writer
        .finish(Duration::from_secs(60))
        .expect("encoded interval");
    assert_eq!(runtime.omissions(), 0);
    let span = frames.last().expect("last frame")["elapsed"]
        .as_f64()
        .expect("elapsed")
        - frames.first().expect("first frame")["elapsed"]
            .as_f64()
            .expect("elapsed");
    assert!(span >= 20.0, "submitted only {span}s");
    assert_eq!(written.frames, frames.len() as u64);
    json!({"frames":frames,"hashes":written.frame_hashes,"encoding":"ZstdDelta",
        "omissions":runtime.omissions(),"observation_seconds":begin.elapsed().as_secs_f64()})
}

fn drain(
    runtime: &mut Offscreen<SkyGame>,
    device: &wgpu::Device,
    writer: &ImageWriter,
    frames: &mut Vec<Value>,
) {
    device.poll(wgpu::PollType::Poll).expect("Metal poll");
    while runtime.completed().expect("completed frame").is_some() {}
    while let Some(image) = writer.recycled() {
        runtime.recycle(image);
    }
    while let Some(image) = runtime.image().expect("water output") {
        let elapsed = image.sample.observation.as_secs_f64();
        let submission = image.sample.submission_order;
        if let Some(previous) = frames.last() {
            assert_eq!(
                previous["submission"].as_u64().expect("serial") + 1,
                submission
            );
        }
        frames.push(
            json!({"elapsed":elapsed,"submission":submission,"tick":image.sample.tick,
            "phase":image.sample.display_phase, "interpolating":image.sample.clock_interpolating}),
        );
        assert!(
            writer.submit(image).is_none(),
            "water image writer exhausted"
        );
    }
}
