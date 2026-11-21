//! The dense flythrough field: Frustum and Occlusion camera culling must each
//! match the Direct oracle byte for byte along the scripted path, through snap
//! turns, cuts, near-plane crossings and drifting props, for the sprite field
//! and for the mixed-mesh field (M1's every-tick oracle target). Set
//! `THRUM_FIELD_FULL_PATH=1` to compare every tick through the first cut, and
//! `THRUM_FIELD_SEQUENCE=<dir>` to write each culled mode's frames within 40
//! ticks of each event, under `<dir>/<mode>/`, for motion review.
//! Run it with `--release` to reach the cut; debug builds cover the first turn.
#![cfg(not(target_arch = "wasm32"))]

mod field_fly;

#[test]
#[ignore = "long: `./dev long-tests` runs it on main, from .config/long-tests.txt"]
fn field_culling_matches_direct_along_the_flythrough() {
    field_fly::fly(
        "field_culling_matches_direct_along_the_flythrough",
        false,
        true,
    );
}

/// Builds the Direct, Frustum and Occlusion hosts of the mixed-mesh field and
/// holds both culled hosts to Direct, so it also makes every Frustum assertion
/// of the frustum-only mixed-mesh flythrough it replaced.
#[test]
#[ignore = "long: `./dev long-tests` runs it on main, from .config/long-tests.txt"]
fn mixed_mesh_occlusion_matches_bucket_direct_through_first_cut() {
    field_fly::fly(
        "mixed_mesh_occlusion_matches_bucket_direct_through_first_cut",
        true,
        true,
    );
}
