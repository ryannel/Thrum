use super::device;
use std::fmt::Write;
use thrum_render::atmosphere::AtmosphereFrame;
use thrum_render::synthetic::{
    CaptureSource, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView,
};
use thrum_test_support::{BeforeChartSnapshot, BeforeLightSplit};

pub(crate) fn prove_scenes(gpu: &thrum_test_support::Gpu) {
    let (device, queue, caps) = device(gpu, 11);
    let mut records = String::new();
    for stars in [false, true] {
        let mut scene = SyntheticScene::grid(1).expect("valid graph-kind fixture");
        scene.sun_shadows = true;
        scene.view = SyntheticView::perspective(
            [0.0, 2.0, 5.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            100.0,
        )
        .expect("perspective view")
        .with_celestial_basis([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
        .expect("celestial rotation");
        if stars {
            scene.star_catalogue = Some(&[[0.0, 1.0, 0.0, 0.0001]]);
        }
        let air = AtmosphereFrame::new(
            [0.2, 0.8, 0.1],
            if stars { 0.0 } else { 100_000.0 },
            [1.0; 3],
            2.0,
            1,
        )
        .expect("valid graph-kind fixture")
        .with_nightglow();
        let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(air),
            1.0,
        ))
        .expect("valid graph-kind fixture");
        for source in [CaptureSource::Sdr, CaptureSource::SceneLinear] {
            let target = OffscreenTarget::with_capture_source(&device, &prepared, 16, 16, source)
                .expect("valid graph-kind fixture");
            for capture in [false, true] {
                let graph = prepared
                    .graph(&target, capture)
                    .expect("valid graph-kind fixture");
                assert_eq!(
                    graph.pass_names().any(|name| name == "atmosphere stars"),
                    stars,
                    "fixture must select its named variant"
                );
                writeln!(records, "stars={stars} source={source:?} capture={capture}")
                    .expect("snapshot string");
                writeln!(
                    records,
                    "names={:?}\norder={:?}\nslots={}",
                    graph.pass_names().collect::<Vec<_>>(),
                    graph.pass_order,
                    graph.transient_slots
                )
                .expect("snapshot string");
                let light = BeforeLightSplit::new(&graph);
                let chart = BeforeChartSnapshot::new(&graph);
                let mut resources = String::new();
                for life in chart.lifetimes(light.lifetimes(&graph)) {
                    writeln!(
                        resources,
                        "life={:?}:{}..{}:{:?}",
                        life.resource, life.first, life.last, life.transient_slot
                    )
                    .expect("snapshot string");
                }
                for storage in &graph.attachment_storage {
                    writeln!(
                        resources,
                        "storage={:?}:{}",
                        storage.resource, storage.reason
                    )
                    .expect("snapshot string");
                }
                records.push_str(&light.resource_indices(&chart.resource_indices(&resources)));
            }
        }
    }
    assert_eq!(records, include_str!("scene-baseline.txt"));
}
