//! The dense flythrough field: Frustum and Occlusion camera culling must each
//! match the Direct oracle byte for byte along the scripted path, through snap
//! turns, cuts, near-plane crossings and drifting props. Set
//! `THRUM_FIELD_FULL_PATH=1` to compare every tick through the first cut, and
//! `THRUM_FIELD_SEQUENCE=<dir>` to write each culled mode's frames within 40
//! ticks of each event, under `<dir>/frustum/` and `<dir>/occlusion/`, for
//! motion review.
//! Run it with `--release` to reach the cut; debug builds cover the first turn.
#![cfg(not(target_arch = "wasm32"))]

#[path = "support/field_fly.rs"]
mod field_fly;

#[test]
fn field_culling_matches_direct_along_the_flythrough() {
    field_fly::fly(false, true);
}

#[test]
fn mixed_mesh_field_frustum_matches_bucket_direct_along_the_flythrough() {
    field_fly::fly(true, false);
}
