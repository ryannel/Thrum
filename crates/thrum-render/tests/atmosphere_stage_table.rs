//! Each atmosphere stage is one typed key. The charts its graph pass reads
//! are the charts its table lists, which its bind group is built from, and
//! its role, cache bit and timing bucket come from the key.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use support::{AIR_EXTENT, AirProbe, host_with_extent};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
    graph::CompiledGraph,
    registry_audit::{self, StageAudit},
    synthetic::{
        CaptureSource, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene,
        SyntheticView,
    },
};

fn check(label: &str, graph: &CompiledGraph, stages: &mut BTreeMap<String, StageAudit>) {
    for stage in registry_audit::atmosphere_stages(graph) {
        let name = &stage.name;
        let table: Vec<_> = stage
            .table_reads
            .iter()
            .map(|(chart, _)| chart.clone())
            .collect();
        assert_eq!(
            stage.graph_reads, table,
            "{label}: {name} reads charts its bind group does not sample"
        );
        assert_eq!(
            stage.role,
            Some(stage.key_role),
            "{label}: {name}'s role is not its key's"
        );
        assert_eq!(
            stage.timing, stage.key_timing,
            "{label}: {name}'s timing bucket is not its key's"
        );
        assert_eq!(
            stage.validity == 0,
            name == "atmosphere chart overlay lighting",
            "{label}: {name} has an unexpected cache bit"
        );
        stages.insert(name.clone(), stage);
    }
}

#[test]
fn stage_registry_keys_list_chart_bindings_roles_and_cache_bits() {
    let mut table = BTreeMap::new();
    let mut roles = BTreeSet::new();
    for stage in registry_audit::atmosphere_stage_keys() {
        let name = stage.name;
        assert!(
            table.insert(name, stage.table_reads).is_none(),
            "duplicate stage {name}"
        );
        let role = stage
            .role
            .unwrap_or_else(|| panic!("{name} has no pipeline role"));
        assert!(roles.insert(role), "{name} reuses role {role}");
        assert_eq!(
            stage.validity == 0,
            name == "atmosphere chart overlay lighting",
            "{name} has an unexpected cache bit"
        );
    }
    assert_eq!(table.len(), 15, "stages seen: {:?}", table.keys());
    // The table, as the bind groups are built from it.
    let reads = |charts: &[(&str, u32)]| -> Vec<(String, u32)> {
        charts
            .iter()
            .map(|(chart, binding)| ((*chart).to_owned(), *binding))
            .collect()
    };
    assert_eq!(table["atmosphere transmittance"], reads(&[]));
    assert_eq!(table["atmosphere chart overlay lighting"], reads(&[]));
    assert_eq!(
        table["atmosphere multiple scattering"],
        reads(&[("Transmittance", 1)])
    );
    assert_eq!(
        table["atmosphere ground irradiance"],
        reads(&[("Transmittance", 1), ("MultipleScattering", 2)])
    );
    assert_eq!(
        table["atmosphere sky view"],
        reads(&[
            ("Transmittance", 1),
            ("MultipleScattering", 2),
            ("Ground", 5)
        ])
    );
    assert_eq!(
        table["atmosphere aerial perspective"],
        reads(&[("Transmittance", 1), ("MultipleScattering", 2)])
    );
    assert_eq!(
        table["atmosphere diffuse irradiance"],
        reads(&[("SkyView", 1)])
    );
    assert_eq!(table["atmosphere DFG integration"], reads(&[]));
    for mip in 0..7 {
        assert_eq!(
            table[format!("atmosphere reflection mip {mip}").as_str()],
            reads(&[("SkyView", 1)])
        );
    }
}

#[test]
fn stage_registry_prepared_graph_reads_its_keys_and_has_timestamp_slots() {
    let Some(thrum_test_support::Opened {
        gpu,
        device,
        queue,
        capabilities,
    }) = thrum_test_support::open(
        "atmosphere stage registry prepared graph",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    assert_eq!(gpu.adapter.get_info().backend, wgpu::Backend::Metal);

    // The production graph reads prepared textures and pipeline keys, so bind it on Metal.
    // A single lit instance exposes all stages without recording a frame.
    let capabilities = capabilities
        .with_recipe_override("portable")
        .expect("portable recipe");
    let scene = SyntheticScene::grid(1).expect("atmosphere fixture");
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.5, 0.707, 0.5], 100_000.0, [1.0; 3], 2.0, 1).expect("air"),
    );
    let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
        &capabilities,
        &queue,
        &scene,
        lighting,
        1.0,
    ))
    .expect("prepare atmosphere fixture");
    let scene_slots = usize::try_from(prepared.scene_pass_count()).expect("scene slot count");
    let expected: BTreeSet<_> = registry_audit::atmosphere_stage_keys()
        .map(|stage| stage.name)
        .filter(|name| *name != "atmosphere chart overlay lighting")
        .collect();
    let size = thrum_test_support::target::SIZE;
    for source in [CaptureSource::Sdr, CaptureSource::SceneLinear] {
        let target = OffscreenTarget::with_capture_source(&device, &prepared, size, size, source)
            .expect("bind atmosphere graph");
        let target_slots = usize::try_from(registry_audit::target_timed_pass_count(&target))
            .expect("target slot count");
        for readback in [false, true] {
            let label = format!("{source:?} readback={readback}");
            let graph = prepared
                .graph(&target, readback)
                .expect("compiled atmosphere graph");
            let timed = registry_audit::pipeline_pass_count(&graph);
            assert!(
                scene_slots >= timed,
                "{label}: scene has {scene_slots} slots for {timed} timed passes"
            );
            assert!(
                target_slots >= timed,
                "{label}: target has {target_slots} slots for {timed} timed passes"
            );

            let mut stages = BTreeMap::new();
            check(&label, &graph, &mut stages);
            let seen: BTreeSet<_> = stages.keys().map(String::as_str).collect();
            assert_eq!(
                seen, expected,
                "{label}: atmosphere stages in the compiled graph"
            );
        }
    }
}

#[test]
fn a_secondary_view_aerial_volume_reads_the_aerial_stage_table() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary aerial stage table",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let view = |height: f32| {
            SyntheticView::perspective(
                [0.0, height, 0.0],
                [0.0, height, -1.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                5000.0,
            )
            .expect("camera")
        };
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = view(100.0);
        let light = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.5, 0.707, 0.5], 100_000.0, [1.0; 3], 100.0, 1).expect("air"),
        );
        let mut host = host_with_extent(&capabilities, &queue, scene, light, AIR_EXTENT).await;
        let (probe, _outputs) = AirProbe::new(Some(
            &SecondaryViewRequest::new(
                &view(60.0),
                AIR_EXTENT,
                Some(ViewClipPlane::new([0.0, 1.0, 0.0, -100.0]).expect("plane")),
                ViewAir::Plane,
            )
            .expect("ask"),
        ));
        host.set_scene_contribution(Box::new(probe))
            .expect("attach the secondary view");
        let graph = host.scene_graph().expect("inspect the graph");
        // The frame records and validates on Metal with the table's reads.
        host.capture_pixels().expect("secondary frame");
        let mut stages = BTreeMap::new();
        check("secondary view", &graph, &mut stages);
        let secondary = stages
            .get("secondary view aerial perspective 0")
            .unwrap_or_else(|| panic!("no secondary aerial pass: {:?}", stages.keys()));
        assert_eq!(
            secondary.graph_reads,
            ["Transmittance", "MultipleScattering"]
        );
    });
}
