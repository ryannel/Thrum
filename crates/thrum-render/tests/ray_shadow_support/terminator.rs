//! Coarse receiver and sheet fixtures shared by the Metal Sun guards.
use glam::Vec3;
use std::sync::Arc;
use thrum_render::{
    abi::InstanceRecord,
    synthetic::{
        SunShadowMethod, SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes, SyntheticScene,
        SyntheticVertex, SyntheticView,
    },
};

fn quad(mesh: &mut SyntheticMesh, points: [Vec3; 4], normals: [Vec3; 4], reverse: bool) {
    let start = u32::try_from(mesh.vertices.len()).expect("small fixture");
    mesh.vertices
        .extend(
            points
                .into_iter()
                .zip(normals)
                .map(|(position, normal)| SyntheticVertex {
                    position: position.to_array(),
                    normal: normal.to_array(),
                    uv: [0.0; 2],
                    tangent: [0.0, 0.0, 1.0, 1.0],
                }),
        );
    let indices = if reverse {
        [0, 2, 1, 1, 2, 3]
    } else {
        [0, 1, 2, 1, 3, 2]
    };
    mesh.indices.extend(indices.map(|index| start + index));
}

fn empty_mesh() -> SyntheticMesh {
    SyntheticMesh {
        vertices: vec![],
        indices: vec![],
    }
}

fn prism(open_rim: bool) -> SyntheticMesh {
    let radial = |side: u16| {
        let (y, x) = (30.0 * f32::from(side) - 15.0).to_radians().sin_cos();
        Vec3::new(x, y, 0.0)
    };
    let mut mesh = empty_mesh();
    for side in 0..12 {
        // Side 1 is the far face that the receiver's Sun ray crosses.
        if open_rim && side == 1 {
            continue;
        }
        let a = radial(side);
        let b = radial((side + 1) % 12);
        quad(
            &mut mesh,
            [a - Vec3::Z, b - Vec3::Z, a + Vec3::Z, b + Vec3::Z],
            [a, b, a, b],
            false,
        );
    }
    mesh
}

#[derive(Clone, Copy, Debug)]
/// An optional opaque sheet along the receiver's solar ray.
pub struct Sheet {
    /// Metres between receiver and sheet.
    pub distance: f32,
    /// Add the sheet to the receiving mesh itself.
    pub joined: bool,
    /// Present the back face to the Sun ray.
    pub reverse: bool,
}

/// A coarse prism facing the camera, with a known dipped centre ray.
///
/// # Panics
/// The proof geometry and camera must be admissible.
pub fn scene(receiver: Vec3, sun: Vec3, sheet: Option<Sheet>, open_rim: bool) -> SyntheticScene {
    let mut meshes = vec![prism(open_rim)];
    if let Some(sheet) = sheet {
        let centre = receiver + sun * sheet.distance;
        let width = sun.cross(Vec3::Z) * 0.08;
        let depth = Vec3::Z * 0.12;
        let points = [
            centre - width - depth,
            centre + width - depth,
            centre - width + depth,
            centre + width + depth,
        ];
        let normals = [if sheet.reverse { sun } else { -sun }; 4];
        if !sheet.joined {
            meshes.push(empty_mesh());
        }
        quad(
            meshes.last_mut().expect("mesh"),
            points,
            normals,
            sheet.reverse,
        );
    }
    let count = u32::try_from(meshes.len()).expect("fixture");
    let mut scene = SyntheticScene::grid(count).expect("scene");
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        slots: (0..count).map(SyntheticMeshChoice::Mesh).collect(),
        meshes,
        updates: None,
    }));
    scene.instances = (0..count)
        .map(|slot| InstanceRecord::identity(slot, 0))
        .collect();
    scene.view = SyntheticView::perspective(
        (receiver + Vec3::X * 2.5).to_array(),
        receiver.to_array(),
        Vec3::Y.to_array(),
        0.42,
        1.0,
        0.05,
        12.0,
    )
    .expect("terminator view");
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene
}
