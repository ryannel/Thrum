//! Capture-off 1080p sizing sweep on the normal exit frame host.
use std::{io::Write, time::Duration};

use thrum_render::{
    capabilities::DeviceCapabilities, present::SurfaceProfile, synthetic::CullMode,
    timing::SceneGpuTimings,
};
use thrum_test_support::{Profile, Restriction, gpu};
use v1_game::{FrameConfig, FrameHost, FrameSample, GameContent, SurfaceOutput};

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
    let mut output = std::io::stdout().lock();
    let recipe = std::env::args()
        .nth(1)
        .expect("pass the recipe as the first argument");
    let gpu = gpu("exit sizing sweep", Some(wgpu::Backend::Metal)).expect("real Metal adapter");
    let (device, queue) = gpu.device(Profile::Portable, &Restriction::default());
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, FrameHost::DEVICE_GENERATION)
        .with_recipe_override(&recipe)
        .expect("resolve recipe");
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("exit sizing presentation"),
        size: wgpu::Extent3d {
            width: EXTENT[0],
            height: EXTENT[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let profile = SurfaceProfile {
        format,
        extent: EXTENT,
    };
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
        drop(frame(&mut host, &device, &surface));
        let stats = stats.lock().expect("cook stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        if stats.drawn == Some(1) {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "resources never drew");
    }
    writeln!(
        output,
        "{}",
        serde_json::json!({
            "recipe": format!("{:?}", caps.recipe().expect("resolved recipe")),
            "extent": EXTENT, "culling": "frustum", "capture": "off",
        })
    )
    .expect("sweep header");
    let presets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("params");
    let mut heaviest = None;
    let apply = |host: &mut FrameHost, stop: &str| {
        let data =
            std::fs::read(presets.join(format!("showcase-{stop}.params.json"))).expect("stop");
        let mut controls = v1_game::controls::GameControls::new().expect("controls");
        let report = controls
            .model_mut()
            .apply_file(std::str::from_utf8(&data).expect("UTF-8 preset"))
            .expect("preset");
        assert_eq!(report.problems.len(), 0, "preset refused values");
        let state = controls.replay();
        host.set_time_of_day(state.time_of_day.expect("solar parameter"))
            .expect("solar edit");
        host.set_camera_yaw(state.camera_yaw.expect("yaw parameter"))
            .expect("yaw edit");
        host.set_camera_pitch(state.camera_pitch.expect("pitch parameter"))
            .expect("pitch edit");
    };
    for stop in STOPS {
        apply(&mut host, stop);
        let sample = frame(&mut host, &device, &surface);
        let counts = sample.camera_culling.expect("GPU culling readback");
        assert_eq!(counts.active, 10_000);
        writeln!(
            output,
            "{}",
            serde_json::json!({"stop":stop,"in_view":counts.drawn})
        )
        .expect("stop count");
        if heaviest.is_none_or(|(_, drawn)| counts.drawn > drawn) {
            heaviest = Some((stop, counts.drawn));
        }
    }
    let (stop, drawn) = heaviest.expect("route stops");
    apply(&mut host, stop);
    for _ in 0..24 {
        drop(frame(&mut host, &device, &surface));
    }
    for _ in 0..120 {
        let sample = frame(&mut host, &device, &surface);
        let gpu = sample.gpu_pass_ns.as_ref().expect("whole scene timing");
        assert!(gpu.shadow_near() > 0.0 && gpu.shadow_far() > 0.0);
        assert!(sample.surface_presented && !sample.timing_fault);
        let gpu_ns = SceneGpuTimings::sum(gpu) + sample.gpu_present_ns.expect("present timing");
        writeln!(
            output,
            "{}",
            serde_json::json!({"measured_stop":stop,"in_view":drawn,"gpu_ns":gpu_ns})
        )
        .expect("timed sample");
    }
}
