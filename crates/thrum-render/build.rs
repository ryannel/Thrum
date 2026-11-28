//! Compile Rust-owned geometry and buffer constants into composable WGSL preludes.

use std::fmt::Write as _;
use std::path::PathBuf;

mod constants {
    include!("src/shader_constants.rs");
}
use constants::{
    ADAPTED_LUX, AERIAL_EXTENT, AERIAL_WORKGROUP_EDGE, AIR_SHADOW_SIZE, ATMOSPHERE_EXTENTS,
    ATMOSPHERE_PLANES, ATMOSPHERE_TOP, CAMERA_EXPOSURE_NORMALIZATION, CLUSTER_WORKGROUP, CLUSTER_X,
    CLUSTER_Y, CLUSTER_Z, DISPLAY_OVERLAY_SHIFT, EXPOSURE_INCIDENT, EXPOSURE_LOW_LIGHT,
    HISTORY_WORKGROUP, INCIDENT_METER_LUX_SCALE, LUT_WORKGROUP_EDGE, MAX_BUCKETS, MAX_FINITE_HALF,
    MEMBER_PADDING, NIGHT_TERMINAL_MAX_ALTITUDE, OVERLAY_EXPOSURE_ZONES, OVERLAY_MOTION,
    OVERLAY_TEMPORAL_INPUTS, OVERLAY_UNDITHERED, PHASE_FINAL, PHASE_FRUSTUM, PHASE_PREPASS,
    PLANET_RADIUS, PYRAMID_WORKGROUP_EDGE, REC709_LUMA, RECONSTRUCTION_WORKGROUP_EDGE,
    SHADOW_HEIGHT_LAYER, SHADOW_HEIGHT_SIZE, SUN_ANGULAR_RADIUS, SUN_CASCADE_COUNT,
    SUN_OUTLINE_OFFSETS, SUN_OUTLINE_RADIUS, TONE_MAP_NEUTRAL_HIGHLIGHTS, VISIBILITY_WORKGROUP,
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

fn emit_scene_shadows(out: &std::path::Path, shader_tangent: f32) {
    emit(
        out,
        "scene_shadows.wgsl",
        &[
            ("SHADOW_HEIGHT_LAYER", SHADOW_HEIGHT_LAYER),
            ("SHADOW_HEIGHT_SIZE", SHADOW_HEIGHT_SIZE),
        ],
        &[
            ("SUN_TAN_RADIUS", shader_tangent),
            ("SUN_OUTLINE_RADIUS", SUN_OUTLINE_RADIUS),
            (
                "SUN_OUTLINE_X1",
                SUN_OUTLINE_OFFSETS[1][0] - SUN_OUTLINE_OFFSETS[0][0],
            ),
            (
                "SUN_OUTLINE_Y1",
                SUN_OUTLINE_OFFSETS[1][1] - SUN_OUTLINE_OFFSETS[0][1],
            ),
            (
                "SUN_OUTLINE_X2",
                SUN_OUTLINE_OFFSETS[2][0] - SUN_OUTLINE_OFFSETS[0][0],
            ),
            (
                "SUN_OUTLINE_Y2",
                SUN_OUTLINE_OFFSETS[2][1] - SUN_OUTLINE_OFFSETS[0][1],
            ),
            (
                "SUN_OUTLINE_X3",
                SUN_OUTLINE_OFFSETS[3][0] - SUN_OUTLINE_OFFSETS[0][0],
            ),
            (
                "SUN_OUTLINE_Y3",
                SUN_OUTLINE_OFFSETS[3][1] - SUN_OUTLINE_OFFSETS[0][1],
            ),
        ],
    );
}

fn main() {
    println!("cargo:rerun-if-changed=src/shader_constants.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    // WGSL's original 1.0 / 2.4 rounded to f32 only after abstract division.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Round after dividing in f64"
    )]
    let encode_exponent = constants::SRGB_GAMMA.recip() as f32;
    emit(
        &out,
        "srgb.wgsl",
        &[],
        &[
            ("SRGB_ENCODED_KNEE", constants::SRGB_ENCODED_KNEE),
            ("SRGB_OFFSET", constants::SRGB_OFFSET),
            ("SRGB_SCALE", constants::SRGB_SCALE),
            ("SRGB_GAMMA", constants::SRGB_GAMMA_F32),
            ("SRGB_SLOPE", constants::SRGB_SLOPE_F32),
            ("SRGB_ENCODE_EXPONENT", encode_exponent),
        ],
    );
    let [r, g, b] = REC709_LUMA;
    std::fs::write(
        out.join("lighting_constants.wgsl"),
        format!("const REC709_LUMA = vec3<f32>({r},{g},{b});\nconst MAX_FINITE_HALF: f32 = {MAX_FINITE_HALF:?};\n"),
    ).expect("write WGSL lighting constants");
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
            ("SHADOW_HEIGHT_SIZE", SHADOW_HEIGHT_SIZE),
        ],
        &[],
    );
    emit(
        &out,
        "sun_cascade_layout.wgsl",
        &[("SUN_CASCADE_COUNT", SUN_CASCADE_COUNT)],
        &[],
    );
    // Preserve the original shader's rounding to nine decimal places before
    // conversion to f32. The CPU shadow fit uses the unrounded tangent.
    let sun_tangent = f64::from(SUN_ANGULAR_RADIUS).tan();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "The shader constant is f32"
    )]
    let shader_tangent = ((sun_tangent * 1e9).round() / 1e9) as f32;
    emit_scene_shadows(&out, shader_tangent);
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
            ("OVERLAY_UNDITHERED", OVERLAY_UNDITHERED),
        ],
        &[],
    );
    #[expect(
        clippy::cast_possible_truncation,
        reason = "The camera normalization is f32 in WGSL"
    )]
    let shader_camera_normalization = CAMERA_EXPOSURE_NORMALIZATION as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "The incident meter is f32 in WGSL"
    )]
    let shader_incident_meter_scale = INCIDENT_METER_LUX_SCALE as f32;
    emit(
        &out,
        "exposure_modes.wgsl",
        &[
            ("EXPOSURE_INCIDENT", EXPOSURE_INCIDENT),
            ("EXPOSURE_LOW_LIGHT", EXPOSURE_LOW_LIGHT),
        ],
        &[
            ("CAMERA_EXPOSURE_NORMALIZATION", shader_camera_normalization),
            ("INCIDENT_METER_LUX_SCALE", shader_incident_meter_scale),
            ("ADAPTED_LUX", ADAPTED_LUX),
        ],
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
            ("MASK_WORD_BITS", u32::BITS),
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
            ("SKY_ROW_COUNT", ATMOSPHERE_EXTENTS[2][1]),
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
    emit(
        &out,
        "night_profile.wgsl",
        &[],
        &[("NIGHT_TERMINAL_MAX_ALTITUDE", NIGHT_TERMINAL_MAX_ALTITUDE)],
    );
}
