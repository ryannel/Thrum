//! Authored transforms and closed paths shared by the native and browser game.

#[cfg(feature = "proving")]
mod shadow_proof;
#[cfg(feature = "proving")]
pub use shadow_proof::SunProofGeometry;

use std::sync::{Arc, OnceLock};

use serde::Deserialize;
use thrum_render::{
    abi::MaterialRecord,
    synthetic::{SyntheticMeshChoice, SyntheticMeshes, SyntheticScene},
};

use crate::scene::{ENTITY_COUNT, MOVING_COUNT};

pub(crate) const MESH_NAMES: [&str; 5] = [
    "exit-boulder",
    "exit-scrub",
    "exit-cactus",
    "exit-standing-rock",
    "exit-balloon",
];
const BALLOON_MESH_ROW: u32 = 4;

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) struct Prop(u32, u64, f64, f64, f64, f32, f32);

impl Prop {
    pub(crate) fn mesh_row(self) -> u32 {
        self.0
    }

    pub(crate) fn orientation(self) -> (f32, f32) {
        (self.5, self.6)
    }

    pub(crate) fn position(self) -> [f64; 3] {
        [self.2, self.3, self.4]
    }
    pub(crate) fn material(self) -> u64 {
        u64::from(self.0) * 4 + self.1
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) struct Drift(
    pub(crate) [f64; 3],
    pub(crate) [f64; 3],
    pub(crate) u32,
    pub(crate) u32,
);

impl Drift {
    pub(crate) fn at(self, tick: u64) -> crate::simulation::Position {
        let Self(center, extent, period, offset) = self;
        let ticks_per_loop = u64::from(period);
        let phase = (tick % ticks_per_loop + u64::from(offset)) % ticks_per_loop;
        let phase = u32::try_from(phase).expect("phase bounded by the u32 period");
        let (sin, cos) = (std::f64::consts::TAU * f64::from(phase) / f64::from(period)).sin_cos();
        crate::simulation::Position(std::array::from_fn(|axis| {
            center[axis] + extent[axis] * if axis == 0 { cos } else { sin }
        }))
    }
}

#[derive(Deserialize)]
pub(crate) struct Layout {
    pub(crate) entities: Vec<Prop>,
    pub(crate) loops: Vec<Drift>,
}

pub(crate) fn layout() -> &'static Layout {
    static LAYOUT: OnceLock<Layout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        let layout: Layout =
            serde_json::from_str(include_str!("../assets/exit/layout.json")).expect("exit layout");
        assert_eq!(layout.entities.len(), ENTITY_COUNT);
        assert_eq!(layout.loops.len(), MOVING_COUNT);
        layout
    })
}

pub(crate) fn scene() -> Result<SyntheticScene, thrum_render::error::RenderError> {
    let count = u32::try_from(ENTITY_COUNT).expect("exit scene slot count");
    let mut scene = SyntheticScene::grid(count)?;
    let palette = [
        [
            [0.26, 0.173, 0.106],
            [0.32, 0.237, 0.153],
            [0.21, 0.154, 0.104],
            [0.37, 0.274, 0.183],
        ],
        [
            [0.14, 0.147, 0.068],
            [0.18, 0.18, 0.087],
            [0.20, 0.168, 0.076],
            [0.117, 0.136, 0.07],
        ],
        [
            [0.092, 0.155, 0.076],
            [0.076, 0.125, 0.061],
            [0.136, 0.189, 0.093],
            [0.16, 0.18, 0.08],
        ],
        [
            [0.34, 0.189, 0.105],
            [0.30, 0.18, 0.099],
            [0.27, 0.184, 0.114],
            [0.385, 0.213, 0.124],
        ],
        [
            [0.60, 0.101, 0.039],
            [0.70, 0.46, 0.18],
            [0.087, 0.26, 0.272],
            [0.72, 0.65, 0.49],
        ],
    ];
    scene.materials = palette
        .into_iter()
        .flatten()
        .map(|[r, g, b]| MaterialRecord {
            base_rgba: [r, g, b, 1.0],
            surface: [1.0, 0.85, 0.0, 1.0],
            ..Default::default()
        })
        .collect();
    let props = &layout().entities;
    for (instance, prop) in scene.instances.iter_mut().zip(props) {
        let (sin, cos) = prop.5.sin_cos();
        let basis = [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]];
        for (axis, direction) in basis.into_iter().enumerate() {
            instance.current[axis][..3].copy_from_slice(&direction.map(|v| v * prop.6));
            instance.normal[axis][..3].copy_from_slice(&direction.map(|v| v / prop.6));
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "authored metre positions enter f32 render space"
        )]
        let position = prop.position().map(|value| value as f32);
        instance.current[3][..3].copy_from_slice(&position);
        instance.previous = instance.current;
        instance.set_material(u32::try_from(prop.material()).expect("authored material"));
    }
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        updates: None,
        meshes: vec![crate::field_meshes::rock(); MESH_NAMES.len()],
        slots: props
            .iter()
            .map(|prop| SyntheticMeshChoice::Mesh(prop.0))
            .collect(),
    }));
    scene.aloft_mesh_rows = vec![BALLOON_MESH_ROW];
    scene.sun_shadows = true;
    scene.local_lights.clear();
    Ok(scene)
}
