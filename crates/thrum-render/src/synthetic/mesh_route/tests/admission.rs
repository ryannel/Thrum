//! The public mesh selection: every invalid slot, index or mesh is refused
//! before GPU work, and a valid selection prepares the bucket route.

use crate::error::RenderError;
use std::sync::Arc;

use super::gpu;
use crate::synthetic::{
    FrameLighting, PreparedSynthetic, SyntheticLight, SyntheticMesh, SyntheticMeshChoice,
    SyntheticMeshes, SyntheticScene, SyntheticVertex,
};
use crate::visibility::CullMode;
use SyntheticMeshChoice::{Cube, Mesh};

fn triangle() -> SyntheticMesh {
    let vertex = |position| SyntheticVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        uv: [0.0; 2],
        tangent: [1.0, 0.0, 0.0, 1.0],
    };
    SyntheticMesh {
        vertices: vec![
            vertex([0.0, 0.0, 0.0]),
            vertex([1.0, 0.0, 0.0]),
            vertex([0.0, 1.0, 0.0]),
        ],
        indices: vec![0, 1, 2],
    }
}

#[test]
fn invalid_selections_are_refused_and_a_valid_one_prepares() {
    let Some(gpu) = gpu(
        "invalid_selections_are_refused_and_a_valid_one_prepares",
        None,
    ) else {
        return;
    };
    let prepare = |scene: &SyntheticScene| {
        pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
            &gpu.capabilities,
            &gpu.queue,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        ))
        .map(|prepared| prepared.bucket_route.is_some())
    };
    let base = SyntheticScene::grid(3).expect("grid");
    let with = |meshes: Vec<SyntheticMesh>, slots: Vec<SyntheticMeshChoice>| {
        let mut scene = base.clone();
        scene.meshes = Some(Arc::new(SyntheticMeshes {
            meshes,
            slots,
            updates: None,
        }));
        scene
    };
    let mut out_of_range = triangle();
    out_of_range.indices[2] = 3;
    let mut not_triangles = triangle();
    not_triangles.indices.pop();
    let mut non_finite = triangle();
    non_finite.vertices[1].uv[0] = f32::INFINITY;
    // Finite, but its extent (4e30) and any scaled centre would leave the
    // range the Sun fit can represent.
    let mut extreme = triangle();
    extreme.vertices[0].position = [-2e30, 0.0, 0.0];
    extreme.vertices[1].position = [2e30, 0.0, 0.0];
    let mut large = triangle();
    large.vertices[1].position = [1e29, 0.0, 0.0];
    let mut occlusion = with(vec![triangle()], vec![Cube, Mesh(0), Cube]);
    occlusion.camera_culling = CullMode::Occlusion;
    use crate::error::ArenaError;
    let short = with(vec![triangle()], vec![Cube, Mesh(0)]);
    let long = with(vec![], vec![Cube; 4]);
    assert!(matches!(prepare(&short), Err(RenderError::MeshSlotCount)));
    assert!(matches!(prepare(&long), Err(RenderError::MeshSlotCount)));
    let unknown = with(vec![triangle()], vec![Cube, Mesh(1), Cube]);
    assert!(matches!(
        prepare(&unknown),
        Err(RenderError::InvalidMeshSlot)
    ));
    for (mesh, expected) in [
        (
            out_of_range,
            ArenaError::IndexOutOfRange {
                index: 3,
                vertex_count: 3,
            },
        ),
        (not_triangles, ArenaError::NotTriangles),
        (non_finite, ArenaError::NonFinite),
        (extreme, ArenaError::CoordinateLimit),
    ] {
        assert!(
            matches!(
                prepare(&with(vec![mesh], vec![Cube; 3])),
                Err(RenderError::Arena(actual)) if actual == expected
            ),
            "mesh must be refused with {expected}"
        );
    }
    assert_eq!(
        prepare(&base).ok(),
        Some(false),
        "no selection: legacy route"
    );
    assert_eq!(
        prepare(&occlusion).ok(),
        Some(true),
        "Occlusion bucket route"
    );
    let valid = with(vec![triangle()], vec![Cube, Mesh(0), Cube]);
    assert_eq!(prepare(&valid).ok(), Some(true), "the bucket route");
    let large = with(vec![large], vec![Cube, Mesh(0), Cube]);
    assert_eq!(
        prepare(&large).ok(),
        Some(true),
        "within the coordinate limit"
    );
}
