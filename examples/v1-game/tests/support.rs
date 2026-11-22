//! Shared native devices and state-script runner for the effect diagnostics.
use std::path::Path;

/// The cooked game the cooked tests of the `it` target build.
pub(crate) mod cooked;

/// Engine-owned presentation texture for submitted display-frame tests.
pub fn presentation(
    device: &wgpu::Device,
    extent: [u32; 2],
) -> (wgpu::TextureView, thrum_render::present::SurfaceProfile) {
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test engine presentation"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    (
        texture.create_view(&wgpu::TextureViewDescriptor::default()),
        thrum_render::present::SurfaceProfile { format, extent },
    )
}

/// Prepare a real diagnostic device using the existing state-test request policy.
/// # Panics
/// Panics when the target adapter, device or selected recipe is unavailable.
pub fn device(
    gpu: &thrum_test_support::Gpu,
    recipe: &str,
    timing: bool,
) -> (
    wgpu::Instance,
    wgpu::Device,
    wgpu::Queue,
    thrum_render::capabilities::DeviceCapabilities,
) {
    let (device, queue, caps) = gpu
        .device_with_hardware(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction {
                features: (!timing).then_some(wgpu::Features::empty()),
                ..Default::default()
            },
            1,
            &[],
        )
        .expect("admitted effect test device");
    let caps = caps.with_recipe_override(recipe).expect("target recipe");
    (gpu.instance.clone(), device, queue, caps)
}

/// Run the state, cadence and recovery script on every native recipe.
/// # Panics
/// Panics if target setup, a submitted result or evidence storage fails.
pub fn run(record_name: &str) {
    let Some(gpu) = thrum_test_support::gpu(record_name, None) else {
        return;
    };
    pollster::block_on(async {
        let recipes: &[&str] = if cfg!(target_os = "macos") {
            &["portable", "apple-tile"]
        } else {
            &["portable"]
        };
        let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m2");
        std::fs::create_dir_all(&output).expect("evidence directory");
        let mut records = Vec::new();
        for recipe in recipes {
            let (instance, device, queue, caps) = device(&gpu.fresh_adapter(), recipe, true);
            let record = Box::pin(v1_game::effect::qualification::run(
                &device, &queue, &caps, &instance,
            ))
            .await
            .expect("every-texel state, cadence and recovery script");
            assert!(
                record.contains("cadence-run:14:exact"),
                "runtime cadence boundary was exercised"
            );
            std::fs::write(output.join(format!("{record_name}-{recipe}.txt")), &record)
                .expect("recipe record");
            records.push(format!("{recipe}: {record}"));
        }
        std::fs::write(
            output.join(format!("{record_name}-record.txt")),
            records.join("\n"),
        )
        .expect("target record");
    });
}
