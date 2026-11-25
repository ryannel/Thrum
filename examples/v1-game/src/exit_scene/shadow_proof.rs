//! Opt-in casters and receivers for the valley's soft Sun inspections.
use glam::Vec3;
use std::sync::Arc;
use thrum_render::{
    abi::InstanceRecord,
    synthetic::{SyntheticMesh, SyntheticMeshChoice, SyntheticScene, SyntheticVertex},
};

use super::Drift;

/// Optional geometry added to the populated valley for a shadow capture.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SunProofGeometry {
    /// The authored valley.
    #[default]
    Off,
    /// A low opaque caster on a short, fast drift.
    Fast,
    /// A low caster that clears a still ambient receiver for the rest of the take.
    Ambient,
    /// An open shell and a panel in one mesh.
    Joined,
    /// The same open shell and panel in separate instances.
    Separate,
}

const FAST_SLOT: usize = 1499;
const SHELL_SLOT: usize = 5080;
const PANEL_SLOT: usize = SHELL_SLOT + 1;
const SHELL_POSITION: [f32; 3] = [0.0, 242.0, -5.0];

impl SunProofGeometry {
    fn shell(self) -> bool {
        matches!(self, Self::Joined | Self::Separate)
    }

    pub(crate) fn position(self, index: usize) -> Option<[f64; 3]> {
        (self.shell() && (index == SHELL_SLOT || (self == Self::Separate && index == PANEL_SLOT)))
            .then(|| SHELL_POSITION.map(f64::from))
    }

    pub(crate) fn drift(self, index: usize, authored: Option<Drift>) -> Option<Drift> {
        if index != FAST_SLOT {
            return authored;
        }
        match self {
            Self::Fast => Some(Drift([-0.8, 241.4, -13.4], [2.8, 0.0, 0.45], 42, 0)),
            Self::Ambient => Some(Drift([-29.0, 241.4, -13.4], [31.0, 0.0, 0.0], 360, 0)),
            _ => authored,
        }
    }

    pub(crate) fn prepare(self, scene: &mut SyntheticScene) {
        if matches!(self, Self::Fast | Self::Ambient) {
            let mut caster = instance(scene, FAST_SLOT);
            for (axis, value) in [0.30_f32, 0.14, 0.24].into_iter().enumerate() {
                caster.current[axis][axis] = value;
                caster.normal[axis][axis] = value.recip();
            }
            let position = self
                .drift(FAST_SLOT, None)
                .expect("authored proof drift")
                .at(0)
                .0;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "authored valley positions fit f32"
            )]
            let position = position.map(|value| value as f32);
            caster.current[3][..3].copy_from_slice(&position);
            caster.previous = caster.current;
            scene.instances[FAST_SLOT] = caster;
            Arc::make_mut(scene.meshes.as_mut().expect("valley meshes")).slots[FAST_SLOT] =
                SyntheticMeshChoice::Cube;
        } else if self.shell() {
            self.prepare_shell(scene);
        }
    }

    fn prepare_shell(self, scene: &mut SyntheticScene) {
        // Keep the cooked starting meshes but do not publish updates that
        // contain only the ordinary rows while the proof geometry is present.
        Arc::make_mut(scene.meshes.as_mut().expect("valley meshes")).updates = None;
        let phase =
            crate::environment::DayCycle::offset_for_time(0, 17.4 / 24.0).expect("low Sun time");
        let sun = Vec3::from_array(crate::environment::DayCycle::sample(0, 1.0, phase).toward_sun);
        let plane_normal = (Vec3::Z - sun * sun.z).normalize();
        let (sin, cos) = 4.0_f32.to_radians().sin_cos();
        let normal = plane_normal * cos - sun * sin;
        let tangent = (sun - normal * sun.dot(normal)).normalize();
        let axis = normal.cross(tangent);
        let mut shell = empty_mesh();
        let radial = |angle: f32| {
            let (sine, cosine) = angle.to_radians().sin_cos();
            normal * cosine + tangent * sine
        };
        for (a, b) in [(-45.0, -15.0), (-15.0, 15.0)] {
            let a = radial(a);
            let b = radial(b);
            quad(
                &mut shell,
                [a - axis, b - axis, a + axis, b + axis],
                [a, b, a, b],
            );
        }
        let receiver = normal * 15.0_f32.to_radians().cos() + tangent * 0.23;
        let centre = receiver + sun * 12.0;
        let across = axis.cross(sun) * 2.0;
        let panel = |mesh: &mut SyntheticMesh| {
            quad(
                mesh,
                [
                    centre - across,
                    centre + across,
                    centre - across + axis * 2.0,
                    centre + across + axis * 2.0,
                ],
                [sun; 4],
            );
        };
        if self == Self::Joined {
            panel(&mut shell);
        } else {
            let mut separate = empty_mesh();
            panel(&mut separate);
            add_mesh(scene, PANEL_SLOT, separate);
        }
        add_mesh(scene, SHELL_SLOT, shell);
    }
}

fn instance(scene: &SyntheticScene, index: usize) -> InstanceRecord {
    let old = scene.instances[index];
    let mut row = InstanceRecord::identity(old.logical_slot(), old.material());
    row.current[3][..3].copy_from_slice(&SHELL_POSITION);
    row.previous = row.current;
    row
}

fn add_mesh(scene: &mut SyntheticScene, index: usize, mesh: SyntheticMesh) {
    scene.instances[index] = instance(scene, index);
    let meshes = Arc::make_mut(scene.meshes.as_mut().expect("valley meshes"));
    let row = u32::try_from(meshes.meshes.len()).expect("valley mesh row");
    meshes.meshes.push(mesh);
    meshes.slots[index] = SyntheticMeshChoice::Mesh(row);
}

fn empty_mesh() -> SyntheticMesh {
    SyntheticMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    }
}

fn quad(mesh: &mut SyntheticMesh, positions: [Vec3; 4], normals: [Vec3; 4]) {
    let start = u32::try_from(mesh.vertices.len()).expect("shadow proof mesh");
    mesh.vertices.extend(
        positions
            .into_iter()
            .zip(normals)
            .map(|(position, normal)| SyntheticVertex {
                position: position.to_array(),
                normal: normal.to_array(),
                uv: [0.0; 2],
                tangent: normal.any_orthonormal_vector().extend(1.0).to_array(),
            }),
    );
    mesh.indices
        .extend([0, 1, 2, 1, 3, 2].map(|index| start + index));
}
