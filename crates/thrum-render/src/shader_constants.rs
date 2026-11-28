// Rust values shared by the host and the WGSL preludes emitted by build.rs.

pub(crate) const CLUSTER_X: usize = 32;
pub(crate) const CLUSTER_Y: usize = 18;
pub(crate) const CLUSTER_Z: usize = 16;
pub(crate) const CLUSTER_WORKGROUP: u32 = 64;
pub(crate) const AIR_SHADOW_SIZE: u32 = 512;
pub(crate) const SUN_CASCADE_COUNT: u32 = 10;
pub(crate) const SUN_OUTLINE_RADIUS: f32 = 16.0;
// Rotated-grid positions relative to the texel centre, in texture XY.
pub(crate) const SUN_OUTLINE_OFFSETS: [[f32; 2]; 4] = [
    [-0.125, -0.375],
    [0.375, -0.125],
    [-0.375, 0.125],
    [0.125, 0.375],
];
pub(crate) const SHADOW_MAP_LAYERS: u32 = SUN_CASCADE_COUNT + 1;
pub(crate) const SHADOW_HEIGHT_SIZE: u32 = 1024;
pub(crate) const SHADOW_HEIGHT_LAYER: u32 = SHADOW_MAP_LAYERS - 1;
pub(crate) const HISTORY_WORKGROUP: u32 = 64;
pub(crate) const RECONSTRUCTION_WORKGROUP_EDGE: u32 = 8;
pub(crate) const PYRAMID_WORKGROUP_EDGE: u32 = 8;

pub(crate) const DISPLAY_OVERLAY_SHIFT: u32 = 8;
pub(crate) const TONE_MAP_NEUTRAL_HIGHLIGHTS: u32 = 1;
pub(crate) const OVERLAY_EXPOSURE_ZONES: u32 = 1;
pub(crate) const OVERLAY_MOTION: u32 = 2;
pub(crate) const OVERLAY_UNDITHERED: u32 = 5;
pub(crate) const OVERLAY_TEMPORAL_INPUTS: u32 = 3;
pub(crate) const EXPOSURE_INCIDENT: u32 = 1;
pub(crate) const EXPOSURE_LOW_LIGHT: u32 = 2;
pub(crate) const CAMERA_EXPOSURE_NORMALIZATION: f64 = 1.2;
pub(crate) const INCIDENT_METER_LUX_SCALE: f64 = 0.4;
pub(crate) const ADAPTED_LUX: f32 = 1_000.0;
pub(crate) const REC709_LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];
pub(crate) const MAX_FINITE_HALF: f32 = 65_504.0;
pub(crate) const SRGB_ENCODED_KNEE: f32 = 0.04045;
pub(crate) const SRGB_OFFSET: f32 = 0.055;
pub(crate) const SRGB_SCALE: f32 = 1.055;
pub(crate) const SRGB_GAMMA: f64 = 2.4;
pub(crate) const SRGB_SLOPE: f64 = 12.92;

#[expect(
    clippy::cast_possible_truncation,
    reason = "The transfer function also has f32 consumers"
)]
pub(crate) const SRGB_GAMMA_F32: f32 = SRGB_GAMMA as f32;
#[expect(
    clippy::cast_possible_truncation,
    reason = "The transfer function also has f32 consumers"
)]
pub(crate) const SRGB_SLOPE_F32: f32 = SRGB_SLOPE as f32;

pub(crate) const PLANET_RADIUS: f32 = 6_371_000.0;
pub(crate) const ATMOSPHERE_TOP: f32 = 100_000.0;
pub(crate) const NIGHT_TERMINAL_MAX_ALTITUDE: f32 = 15_000.0;
pub(crate) const SUN_ANGULAR_RADIUS: f32 = 0.004_654_21;
pub(crate) const ATMOSPHERE_EXTENTS: [[u32; 2]; 4] = [[256, 64], [512, 32], [192, 108], [128, 1]];
const SPECTRAL_PLANES: u32 = 2;
pub(crate) const ATMOSPHERE_PLANES: [u32; 4] =
    [SPECTRAL_PLANES + 1, SPECTRAL_PLANES, 1, SPECTRAL_PLANES];
pub(crate) const AERIAL_EXTENT: [u32; 3] = [32, 32, 33];
pub(crate) const AERIAL_WORKGROUP_EDGE: u32 = 8;
pub(crate) const LUT_WORKGROUP_EDGE: u32 = 8;

pub(crate) const VISIBILITY_WORKGROUP: u32 = 256;
pub(crate) const MAX_BUCKETS: usize = 64;
pub(crate) const MEMBER_PADDING: u32 = u32::MAX;
pub(crate) const PHASE_FRUSTUM: u32 = 0;
pub(crate) const PHASE_PREPASS: u32 = 1;
pub(crate) const PHASE_FINAL: u32 = 2;
