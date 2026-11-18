//! Convert the shared prop meshes into the proving ground's renderer inputs.

use std::sync::Arc;

use thrum_fixtures::mixed_field::{
    ENTITY_COUNT,
    meshes::{self, PropMesh},
};
use thrum_render::synthetic::{
    SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes, SyntheticVertex,
};

pub(crate) use meshes::mirrored;

pub(crate) fn selection() -> Arc<SyntheticMeshes> {
    selection_for(ENTITY_COUNT, true)
}

pub(crate) fn selection_for(count: usize, keep_walls: bool) -> Arc<SyntheticMeshes> {
    Arc::new(SyntheticMeshes {
        updates: None,
        meshes: meshes::prop_meshes().into_iter().map(render_mesh).collect(),
        slots: (0..count)
            .map(|index| {
                meshes::mesh_slot(index, keep_walls)
                    .map_or(SyntheticMeshChoice::Cube, SyntheticMeshChoice::Mesh)
            })
            .collect(),
    })
}

pub(crate) fn rock() -> SyntheticMesh {
    render_mesh(meshes::rock())
}

fn render_mesh(mesh: PropMesh) -> SyntheticMesh {
    SyntheticMesh {
        vertices: mesh
            .vertices
            .into_iter()
            .map(|vertex| SyntheticVertex {
                position: vertex.position,
                normal: vertex.normal,
                uv: vertex.uv,
                tangent: vertex.tangent,
            })
            .collect(),
        indices: mesh.indices,
    }
}
