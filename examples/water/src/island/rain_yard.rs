//! Yard placements and inspection views for the rain selection.
use thrum_app::sky::{Camera, CookedMaterial, MeshPlacement};

pub(super) fn camera(index: usize) -> Camera {
    match index {
        0 => Camera::look_at([146.8, 18.25, 44.35], [151.0, 16.47, 47.4], 60.0),
        1 => Camera::look_at([154.8, 18.25, 44.55], [150.0, 16.47, 47.5], 60.0),
        2 => Camera::look_at([152.2, 18.25, 49.3], [151.6, 16.47, 44.7], 60.0),
        _ => Camera::look_at([151.0, 31.55, 45.94], [151.0, 16.55, 46.0], 45.0),
    }
}

pub(super) fn placements() -> [MeshPlacement; 5] {
    [
        ("meshes/rain-yard-paving", [0.42, 0.37, 0.29], 0.88),
        ("meshes/rain-yard-track", [0.23, 0.135, 0.074], 0.97),
        ("meshes/rain-yard-roof", [0.15, 0.12, 0.094], 0.87),
        ("meshes/rain-yard-posts", [0.21, 0.107, 0.048], 0.92),
        ("meshes/rain-yard-wall", [0.39, 0.29, 0.21], 0.95),
    ]
    .map(|(name, [r, g, b], roughness)| {
        MeshPlacement::new(name, "textures/white").material(CookedMaterial {
            base_colour: [r, g, b, 1.0],
            roughness,
            metalness: 0.0,
        })
    })
}
