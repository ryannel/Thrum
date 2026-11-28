//! Compare recorded membership to independent scene and placement records.

use super::*;
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    abi::InstanceRecord,
    contribution::ShadowBounds,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, FrameHost as EngineHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{
        FrameLighting, SyntheticLight, SyntheticMeshChoice, SyntheticMeshes, SyntheticScene,
        SyntheticView,
    },
};

const ENTITIES: [(usize, [f32; 3]); 2] = [(0, [0.0, 0.0, -3.0]), (2, [50_000.0, 0.0, -3.0])];
const PLACEMENTS: [[f32; 3]; 3] = [[-1.0, 1.0, -4.0], [-50_000.0, 0.0, -3.0], [2.0, 0.0, -2.0]];

pub(crate) fn prove_casters() {
    let Some(metal) = Metal::new(1) else {
        return;
    };
    let fixture = Fixture::new("caster-membership");
    let mut baseline = None;
    for sun_shadows in [false, true] {
        let initial = view([0.0, 0.0, -1.0]);
        let scene = scene(&initial, sun_shadows);
        let mut host = pollster::block_on(EngineHost::new(
            &metal.caps,
            &metal.queue,
            frame_config(),
            World,
            Camera(initial),
            scene.clone(),
            1.0,
        ))
        .expect("caster host");
        let placements =
            PLACEMENTS.map(|position| MeshPlacement::new("placement", "colour").at(position));
        let cooked = CookedAssets::new(CookedConfig::new(
            fixture.cook.store.clone(),
            placements.to_vec(),
        ))
        .expect("cooked placements");
        let stats = cooked.stats();
        host.set_scene_contribution(Box::new(cooked))
            .expect("attach");
        await_drawn(&mut host, &stats, Drawn::Generation(fixture.generation()))
            .unwrap_or_else(|error| panic!("caster frame: {error}"));
        for (look, turned) in [([0.0, 0.0, -1.0], false), ([0.5, 0.0, -1.0], true)] {
            let view = view(look);
            host.continuous_source_mut().0 = view;
            host.capture_pixels()
                .expect("record membership and caster volumes");
            let list = host.ray_structure_instances().expect("scene structure");
            // The scene descriptor and placement recipe own the oracle. Slot 1
            // is deliberately inactive. Neither draw counts nor ray IDs choose it.
            let expected = scene
                .instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| instance.is_active())
                .map(|(slot, instance)| (identity(slot), instance.current[3][..3].to_vec(), 3_u8))
                .chain(PLACEMENTS.iter().enumerate().map(|(index, position)| {
                    (
                        identity(scene.instances.len() + index),
                        position.to_vec(),
                        3,
                    )
                }))
                .collect::<Vec<_>>();
            let actual = list
                .iter()
                .map(|item| {
                    (
                        item.slot,
                        vec![item.transform[3], item.transform[7], item.transform[11]],
                        item.mask,
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
            let key = list
                .into_iter()
                .map(|item| (item.slot, item.mesh_row, item.transform, item.mask))
                .collect::<Vec<_>>();
            if let Some(first) = &baseline {
                assert_eq!(&key, first, "camera or Sun maps altered an instance");
            } else {
                baseline = Some(key);
            }
            let cascades = host.ray_inspection_shadow_volumes();
            assert_eq!(cascades.len(), if sun_shadows { 10 } else { 0 });
            for point in [ENTITIES[1].1, PLACEMENTS[1]] {
                let box_at = ShadowBounds::new(point.map(|v| v - 1.0), point.map(|v| v + 1.0))
                    .expect("fixture box");
                assert!(outside_camera(&view, point));
                assert!(cascades.iter().all(|cascade| !cascade.overlaps(box_at)));
            }
            let rays = [
                Ray {
                    origin: [ENTITIES[1].1[0], 0.0, -6.0, 1.0],
                    direction: [0.0, 0.0, 1.0, 8.0],
                    identity: 3,
                },
                Ray {
                    origin: [PLACEMENTS[1][0] - 0.46, 0.037, -6.0, 1.0],
                    direction: [0.0, 0.0, 1.0, 8.0],
                    identity: 5,
                },
            ];
            let answers = cast(
                &metal,
                |layout| host.ray_structure_probe_group(layout),
                &rays,
            );
            assert_eq!(answers, rays.map(|ray| ray.identity));
            report(&json!({"sun_shadows":sun_shadows,"camera_turned":turned,
                "caster_volumes":cascades.len(),"scene_records":expected,"ray_instances":actual,
                "off_camera_and_outside_every_sun_volume_hits":answers}));
        }
    }
}

fn identity(slot: usize) -> u32 {
    u32::try_from(slot).expect("caster fixture slot")
}

fn outside_camera(view: &SyntheticView, point: [f32; 3]) -> bool {
    let matrix = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
    let corners = std::array::from_fn::<_, 8, _>(|index| {
        let corner = std::array::from_fn::<_, 3, _>(|axis| {
            point[axis] + if index & (1 << axis) == 0 { -1.0 } else { 1.0 }
        });
        matrix * glam::Vec3::from_array(corner).extend(1.0)
    });
    corners.iter().all(|clip| clip.x > clip.w)
        || corners.iter().all(|clip| clip.x < -clip.w)
        || corners.iter().all(|clip| clip.z < 0.0)
        || corners.iter().all(|clip| clip.z > clip.w)
}

fn scene(view: &SyntheticView, sun_shadows: bool) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(3).expect("fixture");
    scene.view = *view;
    scene.ray_structure = true;
    scene.sun_shadows = sun_shadows;
    scene.camera_culling = CullMode::Frustum;
    // Use the bucket route so this scene has per-pass caster culling.
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        meshes: vec![],
        slots: vec![SyntheticMeshChoice::Cube; 3],
        updates: None,
    }));
    for (slot, instance) in scene.instances.iter_mut().enumerate() {
        *instance = InstanceRecord::identity(identity(slot), 0);
        instance.deactivate();
    }
    for (slot, position) in ENTITIES {
        let instance = &mut scene.instances[slot];
        instance.current[3][..3].copy_from_slice(&position);
        instance.previous = instance.current;
        instance.set_generation(1);
    }
    scene
}

fn view(target: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(
        [0.0; 3],
        target,
        [0.0, 1.0, 0.0],
        0.6,
        16.0 / 9.0,
        0.1,
        60.0,
    )
    .expect("frustum")
}

pub(super) struct World;
const TICK: SourceTick = SourceTick {
    epoch: 1,
    timeline: 1,
    tick: 0,
};
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        TICK
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
        Ok(WorldAdvance {
            tick: TICK.tick,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: 0,
            },
            timeline: 1,
            page: TICK,
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

struct Camera(SyntheticView);
impl SceneSource<SourceTick> for Camera {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            for (slot, position) in ENTITIES {
                sink.push(
                    identity(slot),
                    slot as u64 + 1,
                    slot as u128 + 1,
                    position.map(f64::from),
                    0,
                );
            }
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.0,
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        })
    }
}
