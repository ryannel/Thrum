//! Capture-off GPU timestamps for one held showcase preset.
use std::{collections::BTreeMap, time::Duration};
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, SurfaceOutput},
};
use v1_game::{GameHost as _, controls::GameControls};

const EXTENT: [u32; 2] = [1920, 1080];
const SETTLED_FRAMES: u32 = 32;

fn main() -> Result<(), String> {
    pollster::block_on(measure())
}

#[expect(
    clippy::print_stdout,
    reason = "measurement executable emits a JSON record"
)]
async fn measure() -> Result<(), String> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("expected a route preset")?;
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut controls = GameControls::new().map_err(|error| error.to_string())?;
    let report = controls
        .model_mut()
        .apply_file(&text)
        .map_err(|error| error.to_string())?;
    if !report.problems.is_empty() || controls.take_edits().paused != Some(true) {
        return Err(format!("route preset refused: {report}"));
    }
    let state = controls.replay();
    let gpu = thrum_test_support::gpu("atmosphere route cost", Some(wgpu::Backend::Metal))
        .ok_or("Metal unavailable")?;
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let adapter = gpu.adapter;
    let caps = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
    let profile = SurfaceProfile {
        format: wgpu::TextureFormat::Bgra8Unorm,
        extent: EXTENT,
    };
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("atmosphere route cost offscreen output"),
        size: wgpu::Extent3d {
            width: EXTENT[0],
            height: EXTENT[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: profile.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let surface = SurfaceOutput {
        view: &view,
        extent: EXTENT,
    };
    let mut host = v1_game::selected_setup()?
        .prepare(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(profile),
            },
        )
        .await?;
    let content =
        v1_game::attach_cooked_scene(&mut host)?.ok_or("route needs the cooked valley")?;
    host.world_mut().pause()?;
    host.set_time_of_day(state.time_of_day.ok_or("missing solar time")?)?;
    host.set_camera_yaw(state.camera_yaw.ok_or("missing yaw")?)?;
    host.set_camera_pitch(state.camera_pitch.ok_or("missing pitch")?)?;
    if host.reconstruction() != thrum_render::synthetic::Reconstruction::Taau {
        return Err("route cost requires TAAU".to_owned());
    }
    let mut frames = Vec::new();
    for index in 0..600_u32 {
        let elapsed = Duration::from_secs_f64(f64::from(index) / 60.0);
        let resident = {
            let stats = content.lock().map_err(|error| error.to_string())?;
            if !stats.errors.is_empty() {
                return Err(format!("cooked valley errors: {:?}", stats.errors));
            }
            stats.drawn.is_some()
        };
        let report = host
            .drive(elapsed, Some(&surface))
            .map_err(|error| error.to_string())?;
        if report.submitted {
            if !report.surface_presented {
                return Err("GPU submission omitted offscreen output".to_owned());
            }
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|error| error.to_string())?;
            while let Some(sample) = host.poll_completed().map_err(|error| error.to_string())? {
                let timings = sample.gpu_pass_ns.ok_or("GPU timestamps unavailable")?;
                let pass_ns: BTreeMap<_, _> = timings
                    .passes()
                    .iter()
                    .filter(|pass| {
                        pass.name.starts_with("atmosphere ")
                            || pass.name.as_ref() == "synthetic Sun air shadow heights"
                    })
                    .map(|pass| (pass.name.as_ref(), pass.nanoseconds))
                    .collect();
                frames.push(serde_json::json!({
                    "resident":resident, "frame_gpu_ns":timings.timed_pass_span_nanoseconds(),
                    "pass_gpu_ns":pass_ns, "skipped":timings.not_run_passes(),
                }));
            }
        }
        if frames.iter().filter(|row| row["resident"] == true).count() >= SETTLED_FRAMES as usize {
            let [pitch, yaw] = host.camera_orientation();
            println!(
                "{}",
                serde_json::json!({
                    "completed":true, "backend":"Metal", "adapter":adapter.get_info().name,
                    "extent":EXTENT, "reconstruction":"taau",
                    "recipe":format!("{:?}", caps.recipe().map_err(|error| format!("{error:?}"))?), "capture":false,
                    "solar_hours":host.time_of_day()*24.0, "yaw":yaw.to_degrees(), "pitch":pitch.to_degrees(),
                    "frames":frames,
                })
            );
            return Ok(());
        }
        if !resident {
            std::thread::sleep(Duration::from_millis(4));
        }
    }
    Err("did not complete the cooked valley measurement".to_owned())
}
