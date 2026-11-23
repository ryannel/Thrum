//! Resize and rotation keep the surface consumer in the ordinary host graph.
mod support;

use std::time::Duration;
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::{PASS_NAME, SurfaceProfile},
    registry_audit::surface_graph_compiles,
    runtime::{FrameConfig, SurfaceOutput},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
};

#[test]
fn resize_and_rotation_compile_only_host_graphs_and_present_once() {
    let Some(gpu) = thrum_test_support::gpu("surface graph resize", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    pollster::block_on(async {
        let (device, queue) = gpu.portable(None);
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 7);
        let profile = SurfaceProfile {
            format: wgpu::TextureFormat::Rgba8Unorm,
            extent: [32, 32],
        };
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.sun_shadows = false;
        let initial = surface_graph_compiles();
        let mut host = support::host_with_config(
            &capabilities,
            &queue,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            FrameConfig {
                render_extent: profile.extent,
                surface: Some(profile),
            },
        )
        .await;
        assert_eq!(surface_graph_compiles().surface_only, initial.surface_only);
        for extent in [[48, 24], [24, 48]] {
            let before = surface_graph_compiles();
            assert!(host.resize_scene_surface(extent).unwrap());
            let after = surface_graph_compiles();
            assert_eq!(after.surface_only, before.surface_only);
            assert!(after.scene_and_surface > before.scene_and_surface);
            for capture in [false, true] {
                let graph = host.scene_graph_variant(capture).unwrap();
                assert_eq!(
                    graph.pass_names().filter(|&name| name == PASS_NAME).count(),
                    1
                );
            }

            let view = support::drawable(&device, profile.format, extent);
            let encoded = host
                .advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap();
            let frame = encoded.frame_extent().expect("host surface");
            assert_eq!((frame.source, frame.destination), (extent, extent));
            let report = encoded
                .submit(Some(&SurfaceOutput {
                    view: &view,
                    extent,
                }))
                .unwrap();
            assert!(report.surface_presented);
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            assert!(host.poll_completed().unwrap().is_none());
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let sample = host.poll_completed().unwrap().unwrap();
            let timings = sample.gpu_pass_ns.unwrap();
            assert_eq!(
                timings
                    .passes()
                    .iter()
                    .filter(|pass| pass.name.as_ref() == PASS_NAME)
                    .count(),
                1
            );
            assert_eq!(sample.gpu_present_ns, timings.get(PASS_NAME));
            assert_eq!(
                surface_graph_compiles(),
                after,
                "frame encoding compiles no graph"
            );
            println!("{extent:?}: {before:?} -> {after:?}");
        }
    });
}
