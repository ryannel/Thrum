//! The public mesh selection: every invalid slot, index or mesh is refused
//! before GPU work, and a valid selection prepares the bucket route.

use std::sync::Arc;

use super::gpu;
use crate::synthetic::{
    FrameLighting, PreparedSynthetic, RenderError, SyntheticLight, SyntheticMesh,
    SyntheticMeshChoice, SyntheticMeshes, SyntheticScene, SyntheticVertex,
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
    let gpu = gpu();
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
        scene.meshes = Some(Arc::new(SyntheticMeshes { meshes, slots }));
        scene
    };
    let mut out_of_range = triangle();
    out_of_range.indices[2] = 3;
    let mut not_triangles = triangle();
    not_triangles.indices.pop();
    let mut non_finite = triangle();
    non_finite.vertices[1].uv[0] = f32::INFINITY;
    let mut occlusion = with(vec![triangle()], vec![Cube, Mesh(0), Cube]);
    occlusion.camera_culling = CullMode::Occlusion;
    for (case, scene) in [
        (
            "short slot list",
            with(vec![triangle()], vec![Cube, Mesh(0)]),
        ),
        ("long slot list", with(vec![], vec![Cube; 4])),
        (
            "unknown mesh",
            with(vec![triangle()], vec![Cube, Mesh(1), Cube]),
        ),
        (
            "index past vertices",
            with(vec![out_of_range], vec![Cube; 3]),
        ),
        ("partial triangle", with(vec![not_triangles], vec![Cube; 3])),
        ("non-finite vertex", with(vec![non_finite], vec![Cube; 3])),
        ("occlusion", occlusion),
    ] {
        assert!(
            matches!(prepare(&scene), Err(RenderError::SceneCapacity)),
            "{case} is refused"
        );
    }
    assert_eq!(
        prepare(&base).ok(),
        Some(false),
        "no selection: legacy route"
    );
    let valid = with(vec![triangle()], vec![Cube, Mesh(0), Cube]);
    assert_eq!(prepare(&valid).ok(), Some(true), "the bucket route");
}
