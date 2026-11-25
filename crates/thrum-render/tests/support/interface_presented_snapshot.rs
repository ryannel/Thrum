//! Graph and allocation witness for a presented 3D host that requests MetalFX.

use std::{fmt::Write as _, time::Duration};

use super::{sprite_fixture as fixture, sprite_graph_snapshot};
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameHost},
    synthetic::{CullMode, DisplayOverlay, Reconstruction, TwinReason},
};

pub(crate) fn snapshot(gpu: &thrum_test_support::Gpu) -> String {
    let mut records = String::new();
    let extent = [64, 48];
    for recipe in fixture::RECIPES {
        let (device, queue, caps) = fixture::metal(gpu, recipe, 1);
        for format in [fixture::FORMAT, fixture::FLOAT] {
            let (mut scene, lighting) = sprite_graph_snapshot::scene(
                false,
                CullMode::Frustum,
                Reconstruction::Off,
                DisplayOverlay::Off,
            );
            scene.view = scene.view.with_metalfx();
            let mut source = fixture::Scene::new(Vec::new(), 1);
            source.view = scene.view;
            source.lighting = Some(lighting);
            let live_buffers = || {
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(queue.submit([])),
                        timeout: Some(Duration::from_secs(5)),
                    })
                    .expect("retire preparation uploads");
                device.get_internal_counters().hal.buffers.read()
            };
            let before = live_buffers();
            let host = pollster::block_on(FrameHost::new(
                &caps,
                &queue,
                FrameConfig {
                    render_extent: extent,
                    surface: Some(SurfaceProfile { extent, format }),
                },
                fixture::World::default(),
                source,
                scene,
                1.0,
            ))
            .expect("prepare the 3D host with a surface");
            let buffers = live_buffers() - before;
            let reconstruction = host.reconstruction();
            let provider = host.native_provider_report().expect("prepared MetalFX");
            assert_eq!(provider.pass, "MetalFX temporal");
            if recipe == "apple-tile" {
                assert_eq!(reconstruction, Reconstruction::MetalFx);
                assert!(provider.twin.is_none());
            } else {
                assert_eq!(reconstruction, Reconstruction::Taau);
                let twin = provider.twin.expect("portable MetalFX twin");
                assert_eq!(
                    (twin.name, twin.reason),
                    ("TAAU", TwinReason::PortableRecipe)
                );
            }
            let graph = host.scene_graph().expect("prepared presented graph");
            assert!(graph.pass_names().any(|name| name == provider.pass));
            let creation = host.creation_report();
            assert!(
                creation
                    .rows
                    .iter()
                    .all(|row| !row.label.contains("interface"))
            );
            assert!(
                creation
                    .pipelines
                    .iter()
                    .all(|row| !row.label.contains("interface"))
            );

            writeln!(
                records,
                "{recipe} {format:?} {reconstruction:?} buffers={buffers}"
            )
            .expect("write selector and buffer census");
            writeln!(
                records,
                "{:?} order={:?} slots={}",
                graph.pass_names().collect::<Vec<_>>(),
                graph.pass_order,
                graph.transient_slots
            )
            .expect("write pass order");
            let slots: Vec<_> = graph
                .lifetimes
                .iter()
                .filter_map(|life| {
                    Some((life.transient_slot?, &life.resource, life.first, life.last))
                })
                .collect();
            writeln!(
                records,
                "storage={:?}\ntransients={slots:?}",
                graph.attachment_storage
            )
            .expect("write attachment storage and transient slots");
        }
    }
    records
}
