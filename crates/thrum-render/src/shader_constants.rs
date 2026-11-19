// Rust values shared by the host and the WGSL preludes emitted by build.rs.

pub(crate) const CLUSTER_X: usize = 32;
pub(crate) const CLUSTER_Y: usize = 18;
pub(crate) const CLUSTER_Z: usize = 16;
pub(crate) const CLUSTER_WORKGROUP: u32 = 64;
pub(crate) const AIR_SHADOW_SIZE: u32 = 512;

pub(crate) const PLANET_RADIUS: f32 = 6_371_000.0;
pub(crate) const ATMOSPHERE_TOP: f32 = 100_000.0;
pub(crate) const SUN_ANGULAR_RADIUS: f32 = 0.004_654_21;
pub(crate) const ATMOSPHERE_EXTENTS: [[u32; 2]; 4] = [[256, 64], [512, 32], [192, 108], [128, 1]];
const SPECTRAL_PLANES: u32 = 2;
pub(crate) const ATMOSPHERE_PLANES: [u32; 4] =
    [SPECTRAL_PLANES + 1, SPECTRAL_PLANES, 1, SPECTRAL_PLANES];
pub(crate) const AERIAL_EXTENT: [u32; 3] = [32, 32, 33];

pub(crate) const VISIBILITY_WORKGROUP: u32 = 256;
pub(crate) const MAX_BUCKETS: usize = 64;
pub(crate) const MEMBER_PADDING: u32 = u32::MAX;
pub(crate) const PHASE_FRUSTUM: u32 = 0;
pub(crate) const PHASE_PREPASS: u32 = 1;
pub(crate) const PHASE_FINAL: u32 = 2;
