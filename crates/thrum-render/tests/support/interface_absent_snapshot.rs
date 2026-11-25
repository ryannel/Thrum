//! Snapshot collector that also compiles against the unmodified prerequisite.

use std::fmt::Write as _;

use super::{sprite_fixture as fixture, sprite_graph_snapshot};
use thrum_render::{
    sprite::{SpriteRect, SpriteRow, SpriteScene},
    synthetic::CaptureSource,
};

pub(crate) fn snapshot(gpu: &thrum_test_support::Gpu) -> String {
    let mut record = String::new();
    for recipe in fixture::RECIPES {
        let (_device, queue, caps) = fixture::metal(gpu, recipe, 1);
        for format in [fixture::FORMAT, fixture::FLOAT] {
            let extent = [16, 12];
            let sheet = fixture::sheet(
                [1, 1],
                &[255; 4],
                &[SpriteRect {
                    origin: [0; 2],
                    size: [1; 2],
                }],
            );
            let scene = SpriteScene::new(sheet, 1);
            let source = fixture::Scene::new(vec![SpriteRow::new(0, [0.0; 2])], 1);
            let mut host = fixture::host_for_format(&caps, &queue, extent, source, scene, format);
            for capture in [
                None,
                Some(CaptureSource::Sdr),
                Some(CaptureSource::SceneLinear),
            ] {
                if let Some(capture) = capture {
                    host.enable_submitted_capture_from(
                        1,
                        thrum_render::runtime::CaptureLimits::default(),
                        capture,
                    )
                    .expect("prepare the captured graph");
                }
                let graph = host
                    .scene_graph_variant(capture.is_some())
                    .expect("prepared 2D graph");
                writeln!(record, "2D {recipe} {format:?} {capture:?}").expect("write 2D selector");
                writeln!(
                    record,
                    "{:?} order={:?} slots={}",
                    graph.pass_names().collect::<Vec<_>>(),
                    graph.pass_order,
                    graph.transient_slots
                )
                .expect("write 2D pass order");
                writeln!(
                    record,
                    "storage={:?}\nlifetimes={:?}",
                    graph.attachment_storage, graph.lifetimes
                )
                .expect("write 2D attachment lifetimes");
                writeln!(record, "{}", host.storage_report()).expect("write 2D resident storage");
                let counts = host.creation_report();
                assert!(
                    counts
                        .rows
                        .iter()
                        .all(|row| !row.label.contains("interface"))
                );
                assert!(
                    counts
                        .pipelines
                        .iter()
                        .all(|row| !row.label.contains("interface"))
                );
                host.disable_submitted_capture();
            }
        }
    }
    record + &sprite_graph_snapshot::snapshot(gpu)
}
