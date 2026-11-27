// 3D graph closure witness. This source also runs against the untouched base.

use std::fmt::Write as _;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    synthetic::{
        CaptureSource, CullMode, DisplayOverlay, FrameLighting, OffscreenTarget, PreparedSynthetic,
        Reconstruction, SyntheticScene, SyntheticView,
    },
};

const RECIPES: [&str; 2] = ["portable", "apple-tile"];

pub(crate) fn scene(
    stars: bool,
    culling: CullMode,
    sampling: Reconstruction,
    overlay: DisplayOverlay,
) -> (SyntheticScene, FrameLighting) {
    let mut scene = SyntheticScene::grid(1).expect("valid fixture input");
    scene.sun_shadows = true;
    scene.camera_culling = culling;
    if stars {
        scene.star_catalogue = Some(&[[0.0, 1.0, 0.0, 0.0001]]);
    }
    scene.view = SyntheticView::perspective(
        [0.0, 2.0, 5.0],
        [0.0; 3],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        100.0,
    )
    .expect("valid fixture input")
    .with_celestial_basis([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    .expect("finite celestial orientation")
    .with_reconstruction(sampling == Reconstruction::Taau)
    .with_reconstruction_reference(sampling == Reconstruction::Reference)
    .with_temporal_inputs(overlay == DisplayOverlay::TemporalInputs)
    .with_display_overlay(overlay);
    let light = AtmosphereFrame::new(
        [0.2, 0.8, 0.1],
        if stars { 0.0 } else { 100_000.0 },
        [1.0; 3],
        2.0,
        1,
    )
    .expect("valid fixture input")
    .with_nightglow();
    (scene, FrameLighting::Atmosphere(light))
}

pub(crate) fn snapshot(gpu: &thrum_test_support::Gpu) -> String {
    let mut records = String::new();
    for recipe in RECIPES {
        let (device, queue, caps) = gpu
            .device_with_hardware(
                thrum_test_support::Profile::Portable,
                &thrum_test_support::Restriction::default(),
                11,
                &[],
            )
            .expect("prepare the Metal test device");
        let caps: DeviceCapabilities = caps
            .with_recipe_override(recipe)
            .expect("Metal supports the requested recipe");
        let arms = [
            (
                "grid",
                false,
                CullMode::Direct,
                Reconstruction::Off,
                DisplayOverlay::Off,
            ),
            (
                "valley day",
                false,
                CullMode::Frustum,
                Reconstruction::Off,
                DisplayOverlay::Off,
            ),
            (
                "valley stars",
                true,
                CullMode::Frustum,
                Reconstruction::Off,
                DisplayOverlay::Off,
            ),
            (
                "field occlusion",
                false,
                CullMode::Occlusion,
                Reconstruction::Off,
                DisplayOverlay::Off,
            ),
            (
                "temporal inputs",
                false,
                CullMode::Direct,
                Reconstruction::Off,
                DisplayOverlay::TemporalInputs,
            ),
            (
                "taau day",
                false,
                CullMode::Frustum,
                Reconstruction::Taau,
                DisplayOverlay::Off,
            ),
            (
                "taau stars",
                true,
                CullMode::Frustum,
                Reconstruction::Taau,
                DisplayOverlay::Off,
            ),
            (
                "reference",
                false,
                CullMode::Direct,
                Reconstruction::Reference,
                DisplayOverlay::Off,
            ),
        ];
        for (label, stars, culling, sampling, overlay) in arms {
            let (scene, lighting) = scene(stars, culling, sampling, overlay);
            let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
                &caps, &queue, &scene, lighting, 1.0,
            ))
            .expect("valid fixture input");
            for source in [CaptureSource::Sdr, CaptureSource::SceneLinear] {
                let target =
                    OffscreenTarget::with_capture_source(&device, &prepared, 32, 32, source)
                        .expect("prepare the 3D target");
                for capture in [false, true] {
                    let graph = prepared
                        .graph(&target, capture)
                        .expect("compile the 3D graph variant");
                    writeln!(records, "{recipe} {label} {source:?} copy={capture}")
                        .expect("write the snapshot string");
                    writeln!(
                        records,
                        "{:?} order={:?} slots={}",
                        graph.pass_names().collect::<Vec<_>>(),
                        graph.pass_order,
                        graph.transient_slots
                    )
                    .expect("write the snapshot string");
                    // ResourceId's graph-local index is stable; its owner is printed as unit.
                    let light = thrum_test_support::BeforeLightSplit::new(&graph);
                    let chart = thrum_test_support::BeforeChartSnapshot::new(&graph);
                    let resources = format!(
                        "storage={:?}\nlifetimes={:?}\n",
                        graph.attachment_storage,
                        chart.lifetimes(light.lifetimes(&graph))
                    );
                    records.push_str(&light.resource_indices(&chart.resource_indices(&resources)));
                }
            }
        }
    }
    records
}
