//! M1's every-tick mixed-mesh oracle target.
#![cfg(not(target_arch = "wasm32"))]
mod field_fly;

#[test]
#[ignore = "long: `./dev long-tests` runs it on main, from .config/long-tests.txt"]
fn mixed_mesh_occlusion_matches_bucket_direct_through_first_cut() {
    field_fly::fly(
        "mixed_mesh_occlusion_matches_bucket_direct_through_first_cut",
        true,
        true,
    );
}
