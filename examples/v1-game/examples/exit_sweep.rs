//! Capture-off 1080p sizing sweep on the normal exit frame host.
use std::time::Duration;

use thrum_render::{synthetic::CullMode, timing::SceneGpuTimings};
use v1_game::{FrameConfig, FrameHost, FrameSample, GameContent, SurfaceOutput};

#[path = "../tests/support.rs"]
#[allow(dead_code, unreachable_pub)]
mod support;

const EXTENT: [u32; 2] = [1920, 1080];
const STOPS: [&str; 6] = [
    "sunrise",
    "noon",
    "sunset-clear-sun",
    "sunset-sun",
    "twilight",
    "night-upslope",
];

fn frame(host: &mut FrameHost, device: &wgpu::Device, surface: &SurfaceOutput<'_>) -> FrameSample {
    host.drive(Duration::ZERO, Some(surface)).expect("draw");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("finish");
        if let Some(sample) = host.poll_completed().expect("poll") {
            return sample;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "frame sample timed out"
        );
    }
}

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let recipe = std::env::var("THRUM_RECIPE").expect("explicit recipe");
    let (_, device, queue, caps) = support::device(&recipe, true).await;
    let (view, profile) = support::presentation(&device, EXTENT);
    let surface = SurfaceOutput {
        view: &view,
        extent: EXTENT,
    };
    let setup = v1_game::selected_setup().expect("exit setup");
    assert_eq!(setup.content, GameContent::Exit);
    let mut host = FrameHost::new_with_setup(
        &device,
        &queue,
        &caps,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(profile),
        },
        setup,
    )
    .await
    .expect("exit host");
    assert_eq!(host.camera_culling(), CullMode::Frustum);
    let stats = v1_game::attach_cooked_scene(&mut host)
        .expect("attach")
        .expect("cooked exit");
    host.pause().expect("hold");
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let _ = frame(&mut host, &device, &surface);
        let stats = stats.lock().expect("cook stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        if stats.drawn == Some(1) {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "resources never drew");
    }
    println!(
        "{}",
        serde_json::json!({
            "recipe": format!("{:?}", caps.recipe().expect("resolved recipe")),
            "extent": EXTENT, "culling": "frustum", "capture": "off",
        })
    );
    let presets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("params");
    let mut heaviest = None;
    let apply = |host: &mut FrameHost, stop: &str| {
        let data =
            std::fs::read(presets.join(format!("showcase-{stop}.params.json"))).expect("stop");
        let mut controls = v1_game::controls::GameControls::new().expect("controls");
        let report = controls
            .model_mut()
            .apply_file(std::str::from_utf8(&data).unwrap())
            .expect("preset");
        assert!(report.problems.is_empty());
        let state = controls.replay();
        host.set_time_of_day(state.time_of_day.unwrap())
            .expect("solar edit");
        host.set_camera_yaw(state.camera_yaw.unwrap())
            .expect("yaw edit");
        host.set_camera_pitch(state.camera_pitch.unwrap())
            .expect("pitch edit");
    };
    for stop in STOPS {
        apply(&mut host, stop);
        let sample = frame(&mut host, &device, &surface);
        let counts = sample.camera_culling.expect("GPU culling readback");
        assert_eq!(counts.active, 10_000);
        println!(
            "{}",
            serde_json::json!({"stop":stop,"in_view":counts.drawn})
        );
        if heaviest.is_none_or(|(_, drawn)| counts.drawn > drawn) {
            heaviest = Some((stop, counts.drawn));
        }
    }
    let (stop, drawn) = heaviest.expect("route stops");
    apply(&mut host, stop);
    for _ in 0..24 {
        let _ = frame(&mut host, &device, &surface);
    }
    for _ in 0..120 {
        let sample = frame(&mut host, &device, &surface);
        let gpu = sample.gpu_pass_ns.as_ref().expect("whole scene timing");
        assert!(gpu.shadow_near() > 0.0 && gpu.shadow_far() > 0.0);
        assert!(sample.surface_presented && !sample.timing_fault);
        let gpu_ns = SceneGpuTimings::sum(gpu) + sample.gpu_present_ns.expect("present timing");
        println!(
            "{}",
            serde_json::json!({"measured_stop":stop,"in_view":drawn,"gpu_ns":gpu_ns})
        );
    }
}
