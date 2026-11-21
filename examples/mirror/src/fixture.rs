//! Native fixtures for knob isolation and capture-off pass costs.
use std::sync::Arc;

use thrum_render::{
    abi::InstanceRecord,
    atmosphere::AtmosphereFrame,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{SceneFrame, SceneSource},
    synthetic::{
        CullMode, FrameLighting, SyntheticMeshChoice, SyntheticMeshes, SyntheticScene,
        SyntheticView,
    },
};

use crate::PoolPlane;

/// Flat test pool and camera shared by the field workload and knob proof.
pub const POOL: PoolPlane = PoolPlane {
    level: 50.0,
    min: [-25.0, -19.0],
    max: [25.0, 23.0],
};

/// Static frame, with every row left at its prepared transform.
pub struct Source {
    frame: SceneFrame,
    rows: Vec<([f64; 3], u64)>,
}

impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            for (index, &(centre, material)) in self.rows.iter().enumerate() {
                sink.push(
                    u32::try_from(index).expect("fixture fits u32"),
                    1,
                    index as u128,
                    centre,
                    material,
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
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: self.frame.view,
            lighting: self.frame.lighting,
        })
    }
}

fn block(slot: u32, centre: [f32; 3], half: [f32; 3]) -> InstanceRecord {
    let mut instance = InstanceRecord::identity(slot, slot % 2);
    for axis in 0..3 {
        instance.current[axis][axis] = half[axis];
        instance.normal[axis][axis] = 1.0 / half[axis];
        instance.current[3][axis] = centre[axis];
    }
    instance.previous = instance.current;
    instance
}

/// Build a pool with nearby cull and shadow witnesses and a scalable field
/// behind the bank. Use exactly the requested count of entity instances.
#[must_use]
pub fn scene(count: u32, extent: [u32; 2]) -> (SyntheticScene, Source) {
    assert!(count >= 6);
    let mut scene = SyntheticScene::grid(count).expect("valid fixture count");
    scene.view = SyntheticView::perspective(
        [0.0, 55.0, 24.0],
        [0.0, 47.5, -19.0],
        [0.0, 1.0, 0.0],
        52.0_f32.to_radians(),
        extent[0] as f32 / extent[1] as f32,
        0.1,
        32_000.0,
    )
    .expect("fixture camera");
    scene.sun_shadows = true;
    scene.camera_culling = CullMode::Frustum;
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        updates: None,
        meshes: Vec::new(),
        slots: vec![SyntheticMeshChoice::Cube; count as usize],
    }));
    for (slot, (centre, half)) in [
        ([0.0, 48.0, 2.0], [50.0, 1.0, 50.0]),
        ([0.0, 49.8, -33.0], [26.0, 1.0, 13.0]),
        ([-5.0, 54.0, -25.0], [2.5, 4.0, 2.0]),
        ([3.0, 53.0, -18.0], [0.27, 0.27, 0.27]),
        ([18.0, 87.0, -165.0], [12.0, 28.0, 8.0]),
        ([6.0, 52.0, -28.0], [2.0, 2.0, 3.0]),
    ]
    .into_iter()
    .enumerate()
    {
        scene.instances[slot] = block(
            u32::try_from(slot).expect("six fixture slots"),
            centre,
            half,
        );
    }
    for slot in 6..count {
        let index = slot - 6;
        let x = index % 317;
        let z = index / 317;
        scene.instances[slot as usize] = block(
            slot,
            [
                1.2 * x as f32 - 189.6,
                52.0 + (index % 7) as f32 * 0.27,
                -45.0 - 1.2 * z as f32,
            ],
            [0.39, 0.6, 0.39],
        );
    }
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.65, 0.62, 0.44], 100_000.0, [1.0; 3], 55.0, 1)
            .expect("fixture Sun"),
    );
    let source = Source {
        frame: SceneFrame {
            view: scene.view,
            lighting,
        },
        rows: scene
            .instances
            .iter()
            .map(|row| {
                (
                    std::array::from_fn(|axis| f64::from(row.current[3][axis])),
                    u64::from(row.material()),
                )
            })
            .collect(),
    };
    (scene, source)
}
