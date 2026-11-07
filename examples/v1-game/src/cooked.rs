//! G5 cooked products in the V1 game: where they stand, and the opt-in from the environment.
//!
//! `THRUM_ASSET_STORE=<store>` attaches a store written by `thrum-cook watch` (or `demo`) to the
//! running game. The game draws `meshes/terrain` textured with `textures/checker` and the `cloud`
//! volume in front of the instance wall, lit by the game's selected sky, and follows every new
//! publication. Without the variable the game is unchanged.

use std::time::Duration;

use thrum_asset_gpu::{CookedConfig, CookedSource, MeshPlacement, VolumePlacement};

/// Column-major uniform-scale transform, rotated about +X by `tilt` radians.
fn placement(scale: f32, tilt: f32, at: [f32; 3]) -> [[f32; 4]; 4] {
    let (sin, cos) = tilt.sin_cos();
    [
        [scale, 0.0, 0.0, 0.0],
        [0.0, scale * cos, scale * sin, 0.0],
        [0.0, -scale * sin, scale * cos, 0.0],
        [at[0], at[1], at[2], 1.0],
    ]
}

/// Bytes of staged copies per frame: a 128 MiB volume spreads over 16 frames, and one frame's
/// copies take a small fraction of a frame on current GPUs.
pub const UPLOAD_BUDGET: u64 = 8 << 20;

/// The game's placement of the demo products from `source` (a store path converts).
#[must_use]
pub fn cooked_config(source: impl Into<CookedSource>) -> CookedConfig {
    CookedConfig {
        source: source.into(),
        meshes: vec![{
            let mut terrain = MeshPlacement::new("meshes/terrain", "textures/checker");
            // The grid's +Y tilts toward the camera, which looks down -Z from z = 2.
            terrain.transform =
                placement(0.32, std::f32::consts::FRAC_PI_2 * 0.8, [-0.5, -0.2, 0.9]);
            terrain
        }],
        volume: Some(VolumePlacement {
            name: "cloud".into(),
            transform: placement(0.5, 0.0, [0.05, -0.4, 0.95]),
            extinction: 9.0,
            steps: 64,
            light_steps: 6,
        }),
        upload_budget: UPLOAD_BUDGET,
        poll_interval: Duration::from_millis(2),
        texture_preference: None,
    }
}
