//! Shared native devices and state-script runner for the effect diagnostics.
use std::path::Path;

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
pub async fn device(
    recipe: &str,
    timing: bool,
) -> (
    wgpu::Instance,
    wgpu::Device,
    wgpu::Queue,
    thrum_render::capabilities::DeviceCapabilities,
) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("real target adapter");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: if timing {
                thrum_render::capabilities::DeviceCapabilities::requested_timing_features(&adapter)
            } else {
                wgpu::Features::empty()
            },
            ..Default::default()
        })
        .await
        .expect("real target device");
    let caps = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1)
        .with_recipe_override(recipe)
        .expect("target recipe");
    (instance, device, queue, caps)
}

/// Run the state, cadence and recovery script on every native recipe.
/// # Panics
/// Panics if target setup, a submitted result or evidence storage fails.
pub fn run(record_name: &str) {
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
            let (instance, device, queue, caps) = device(recipe, true).await;
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
