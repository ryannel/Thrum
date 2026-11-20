//! Compile Rust-owned geometry and buffer constants into composable WGSL preludes.

use std::fmt::Write as _;
use std::path::PathBuf;

mod constants {
    include!("src/shader_constants.rs");
}
use constants::*;

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
        &[
            ("AIR_SHADOW_SIZE", AIR_SHADOW_SIZE),
            ("SHADOW_HEIGHT_LAYER", SHADOW_HEIGHT_LAYER),
        ],
        &[],
    );
    // Preserve the original shader's rounding to nine decimal places before
    // conversion to f32. The CPU shadow fit uses the unrounded tangent.
    let sun_tangent = f64::from(SUN_ANGULAR_RADIUS).tan();
    let shader_tangent = ((sun_tangent * 1e9).round() / 1e9) as f32;
    emit(
        &out,
        "scene_shadows.wgsl",
        &[("SHADOW_HEIGHT_LAYER", SHADOW_HEIGHT_LAYER)],
        &[("SUN_TAN_RADIUS", shader_tangent)],
    );
    for (file, name, size) in [
        ("history.wgsl", "HISTORY_WORKGROUP", HISTORY_WORKGROUP),
        (
            "reconstruction.wgsl",
            "RECONSTRUCTION_WORKGROUP_EDGE",
            RECONSTRUCTION_WORKGROUP_EDGE,
        ),
        (
            "pyramid.wgsl",
            "PYRAMID_WORKGROUP_EDGE",
            PYRAMID_WORKGROUP_EDGE,
        ),
    ] {
        emit(&out, file, &[(name, size)], &[]);
    }
    emit(
        &out,
        "display_selectors.wgsl",
        &[
            ("DISPLAY_OVERLAY_SHIFT", DISPLAY_OVERLAY_SHIFT),
            ("TONE_MAP_MASK", (1 << DISPLAY_OVERLAY_SHIFT) - 1),
            ("TONE_MAP_NEUTRAL_HIGHLIGHTS", TONE_MAP_NEUTRAL_HIGHLIGHTS),
            ("OVERLAY_EXPOSURE_ZONES", OVERLAY_EXPOSURE_ZONES),
            ("OVERLAY_MOTION", OVERLAY_MOTION),
            ("OVERLAY_TEMPORAL_INPUTS", OVERLAY_TEMPORAL_INPUTS),
        ],
        &[],
    );
    emit(
        &out,
        "exposure_modes.wgsl",
        &[
            ("EXPOSURE_INCIDENT", EXPOSURE_INCIDENT),
            ("EXPOSURE_LOW_LIGHT", EXPOSURE_LOW_LIGHT),
        ],
        &[(
            "CAMERA_EXPOSURE_NORMALIZATION",
            CAMERA_EXPOSURE_NORMALIZATION as f32,
        )],
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
            ("AERIAL_WORKGROUP_EDGE", AERIAL_WORKGROUP_EDGE),
            ("LUT_WORKGROUP_EDGE", LUT_WORKGROUP_EDGE),
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
