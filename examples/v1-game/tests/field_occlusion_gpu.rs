//! M1's every-tick mixed-mesh oracle target.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/field_fly.rs"]
mod field_fly;

#[test]
fn mixed_mesh_occlusion_matches_bucket_direct_through_first_cut() {
    field_fly::fly(true, true);
}
