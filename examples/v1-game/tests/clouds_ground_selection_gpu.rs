//! V1 selects a production ground provider while preserving the upper cloud products.
#![cfg(feature = "proving")]

use std::time::Duration;
use thrum_render::runtime::{FrameConfig, SurfaceOutput};
use thrum_test_support::{Opened, Restriction, readback};
use v1_game::{CloudSelection, GameHost, GameSetup, GroundWeather};

#[test]
fn metal_ground_declares_only_its_own_products_and_preserves_upper_cloud_shade() {
    let Some(Opened {
        device,
        queue,
        capabilities,
        ..
    }) = thrum_test_support::open(
        "V1 Ground selection",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &Restriction::default(),
    )
    else {
        return;
    };
    let extent = [32, 18];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Ground selection output"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent,
    };
    let frame = FrameConfig {
        render_extent: extent,
        surface: Some(thrum_render::present::SurfaceProfile {
            format: texture.format(),
            extent,
        }),
    };
    pollster::block_on(async {
        let mut before = std::collections::BTreeMap::new();
        let mut upper_bytes = None;
        for (has_ground, clouds) in [
            (false, CloudSelection::WorldLight),
            (true, CloudSelection::Ground(GroundWeather::Mist)),
        ] {
            let mut host = GameSetup {
                clouds,
                ..GameSetup::default()
            }
            .prepare(&device, &queue, &capabilities, frame)
            .await
            .expect("V1 cloud selection");
            assert!(
                host.drive(Duration::ZERO, Some(&output))
                    .expect("render")
                    .submitted
            );
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("Metal recording");
            let graph = host.scene_graph().expect("prepared graph");
            let passes: Vec<_> = graph.pass_names().collect();
            let resources = host.cloud_resource_bytes();
            let creations = host.creation_report().named_pipelines;
            for pass in ["cloud ground band", "cloud medium coefficients"] {
                let count = creations.get(pass).copied().unwrap_or(0)
                    - before.get(pass).copied().unwrap_or(0);
                assert_eq!(count > 0, has_ground, "{pass}: {count} creations");
                assert_eq!(
                    passes.iter().any(|name| name.ends_with(pass)),
                    has_ground,
                    "{pass}"
                );
            }
            before = creations;
            assert_eq!(
                resources
                    .iter()
                    .any(|(name, _)| name == "cloud surface field"),
                has_ground
            );
            assert_eq!(host.view_volume_storage().is_some(), has_ground);
            assert!(!passes.iter().any(|name| name.contains("test bank")));
            let (upper, _) = host
                .cloud_ground_product()
                .expect("upper cloud ground field");
            let bytes = readback::texture(&device, &queue, &upper, readback::Mip::level(0), ..);
            if let Some(previous) = upper_bytes.replace(bytes) {
                assert_eq!(
                    upper_bytes.as_ref(),
                    Some(&previous),
                    "original upper cloud shade"
                );
            }
        }
    });
}
