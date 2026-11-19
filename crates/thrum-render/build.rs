//! Compile Rust-owned geometry and buffer constants into composable WGSL preludes.

use std::fmt::Write as _;
use std::path::PathBuf;

mod constants {
    include!("src/shader_constants.rs");
}
use constants::{
    AERIAL_EXTENT, AIR_SHADOW_SIZE, ATMOSPHERE_EXTENTS, ATMOSPHERE_PLANES, ATMOSPHERE_TOP,
    CLUSTER_WORKGROUP, CLUSTER_X, CLUSTER_Y, CLUSTER_Z, MAX_BUCKETS, MEMBER_PADDING, PHASE_FINAL,
    PHASE_FRUSTUM, PHASE_PREPASS, PLANET_RADIUS, SUN_ANGULAR_RADIUS, VISIBILITY_WORKGROUP,
};

fn emit(out: &std::path::Path, name: &str, integers: &[(&str, u32)], floats: &[(&str, f32)]) {
    let mut prelude = String::new();
    for (name, value) in integers {
        writeln!(prelude, "const {name}: u32 = {value}u;").expect("format WGSL integer");
    }
    for (name, value) in floats {
        writeln!(prelude, "const {name}: f32 = {value:?};").expect("format WGSL float");
    }
    std::fs::write(out.join(name), prelude).expect("write WGSL prelude");
}

fn main() {
    println!("cargo:rerun-if-changed=src/shader_constants.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    let dim = |size| u32::try_from(size).expect("shader dimension fits u32");
    emit(
        &out,
        "cluster.wgsl",
        &[
            ("CLUSTER_X", dim(CLUSTER_X)),
            ("CLUSTER_Y", dim(CLUSTER_Y)),
            ("CLUSTER_Z", dim(CLUSTER_Z)),
            ("CLUSTER_WORKGROUP", CLUSTER_WORKGROUP),
        ],
        &[],
    );
    emit(
        &out,
        "air_shadow.wgsl",
        &[("AIR_SHADOW_SIZE", AIR_SHADOW_SIZE)],
        &[],
    );
    emit(
        &out,
        "visibility.wgsl",
        &[
            ("WORKGROUP", VISIBILITY_WORKGROUP),
            ("MAX_BUCKETS", dim(MAX_BUCKETS)),
            ("MEMBER_PADDING", MEMBER_PADDING),
            ("PHASE_FRUSTUM", PHASE_FRUSTUM),
            ("PHASE_PREPASS", PHASE_PREPASS),
            ("PHASE_FINAL", PHASE_FINAL),
        ],
        &[],
    );
    emit(
        &out,
        "atmosphere_geometry.wgsl",
        &[
            ("TRANS_PLANES", ATMOSPHERE_PLANES[0]),
            ("SPECTRAL_PLANES", ATMOSPHERE_PLANES[1]),
        ],
        &[("R", PLANET_RADIUS), ("H", ATMOSPHERE_TOP)],
    );
    emit(
        &out,
        "atmosphere_storage.wgsl",
        &[("AERIAL_INTERVALS", AERIAL_EXTENT[2] - 1)],
        &[("SKY_LUT_GAIN", 1024.0), ("NIGHT_LUT_GAIN", 65536.0)],
    );
    emit(
        &out,
        "atmosphere_luts.wgsl",
        &[
            ("GROUND_SUN_ENTRIES", ATMOSPHERE_EXTENTS[3][0]),
            ("SKY_HALF_ROWS", ATMOSPHERE_EXTENTS[2][1] / 2),
        ],
        &[
            ("MULTIPLE_LUT_GAIN", 65536.0),
            ("GROUND_IRRADIANCE_GAIN", 1024.0),
        ],
    );
    emit(
        &out,
        "sun.wgsl",
        &[],
        &[("SUN_ANGULAR_RADIUS", SUN_ANGULAR_RADIUS)],
    );
}
