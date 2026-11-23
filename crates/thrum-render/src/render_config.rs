//! Scene target formats, pipeline revision and exposure scale shared by renderer features.

/// Scene pipeline source revision.
pub const SYNTHETIC_REVISION: u64 = 18;
/// Portable reversed-Z opaque depth.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Explicit shader sRGB encoding after exposure, tone and gamut mapping.
pub const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub(crate) const PRE_EXPOSURE: f32 = 1.0 / 1024.0;
pub(crate) const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
