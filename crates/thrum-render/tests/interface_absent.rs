//! No-interface closure comparison with the untouched prerequisite and a live opt-in control.

#[expect(
    dead_code,
    reason = "this proof uses the graph and configuration subset of the shared fixture"
)]
mod support {
    pub(crate) mod interface_absent_snapshot;
    pub(crate) mod interface_presented_snapshot;
    pub(crate) mod sprite_fixture;
    pub(crate) mod sprite_graph_snapshot;
}

#[test]
fn presented_metalfx_and_its_twin_match_before_the_interface() {
    let Some(gpu) =
        thrum_test_support::gpu("no-interface 3D presentation", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (revision, expected) = include_str!("fixtures/interface_presented.txt")
        .split_once('\n')
        .expect("baseline revision and graph records");
    assert_eq!(revision, "91d2cf2880314e6446721cb9f396d4deb3203ee9");
    let actual = support::interface_presented_snapshot::snapshot(&gpu);
    for (line, (before, after)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(before, after, "no-interface presented graph line {line}");
    }
    assert_eq!(expected.lines().count(), actual.lines().count());
}

use support::sprite_fixture as fixture;
use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, FrameHost, SceneFrame, SceneSource, SpriteSceneRequests},
    sprite::{InterfaceConfig, NamedSheet, SpriteFrame, SpriteRect, SpriteScene},
    synthetic::FrameLighting,
};

#[test]
fn unchanged_graphs_are_the_main_prerequisite_and_only_the_control_allocates_interface_objects() {
    let Some(gpu) = thrum_test_support::gpu("no-interface closure", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let baseline = include_str!("fixtures/interface_absent.txt");
    let (_, expected) = baseline
        .split_once('\n')
        .expect("baseline has a commit and graph records");
    let actual = support::interface_absent_snapshot::snapshot(&gpu);
    for (index, (before, after)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(before, after, "no-interface graph record line {index}");
    }
    assert_eq!(expected.lines().count(), actual.lines().count());
    for recipe in fixture::RECIPES {
        let (device, queue, caps) = fixture::metal(&gpu, recipe, 2);
        let sheet = || {
            fixture::sheet(
                [1, 1],
                &[255; 4],
                &[SpriteRect {
                    origin: [0; 2],
                    size: [1; 2],
                }],
            )
        };
        let settle = || {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(queue.submit([])),
                    timeout: Some(std::time::Duration::from_secs(5)),
                })
                .expect("retire preparation uploads");
            device.get_internal_counters().hal.buffers.read()
        };
        let before = settle();
        let absent = fixture::configured_host(
            &caps,
            &queue,
            [16, 12],
            fixture::Scene::new(Vec::new(), 1),
            SpriteScene::new(sheet(), 1),
        );
        let without = settle() - before;
        drop(absent);
        let before = settle();
        let source = Active(
            fixture::Scene::new(Vec::new(), 1),
            InterfaceConfig {
                sheets: vec![NamedSheet::pixel("interface control", sheet())],
                row_capacity: 1,
            },
        );
        let host = pollster::block_on(FrameHost::new_2d(
            &caps,
            &queue,
            fixture::scene_config([16, 12]),
            fixture::World::default(),
            source,
            SpriteScene::new(sheet(), 1),
            SpriteSceneRequests::default(),
        ))
        .expect("prepare the positive control");
        assert!(
            settle() - before > without,
            "the control adds row, order, frame and view buffers"
        );
        let graph = host.scene_graph().expect("compiled control");
        assert!(graph.pass_names().any(|pass| pass == "interface stage"));
        let creation = host.creation_report();
        assert!(
            creation
                .pipelines
                .iter()
                .any(|row| row.label == "interface" && row.render > 0)
        );
        assert!(creation.rows.iter().any(|row| row.label == "interface"
            && row.counts.texture_views > 0
            && row.counts.textures > 0));
    }
}

struct Active(fixture::Scene, InterfaceConfig);

impl SceneSource<SourceTick> for Active {
    fn extract(&self, extractor: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        self.0.extract(extractor, page)
    }

    fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        self.0.frame(source, alpha, extent, lighting)
    }

    fn sprites(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
    ) -> Result<SpriteFrame<'_>, FrameError> {
        self.0.sprites(source, alpha, extent)
    }

    fn interface_config(&self) -> Option<InterfaceConfig> {
        Some(self.1.clone())
    }
}
