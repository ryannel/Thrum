//! Whole host records written by cooked drawing, with their shader sources for ABI reflection.
use bytemuck::{Pod, Zeroable};

/// Meshlet cull inputs, column-major matrices.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Cull {
    /// Camera matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Placement matrix.
    pub model: [[f32; 4]; 4],
    /// World camera and largest model scale.
    pub camera_scale: [f32; 4],
    /// Meshlet count and reserved words.
    pub counts: [u32; 4],
}
/// Volume transform and integration parameters.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CookedVolume {
    /// Placement matrix.
    pub model: [[f32; 4]; 4],
    /// World-to-volume matrix.
    pub inverse_model: [[f32; 4]; 4],
    /// Extinction, march steps, light steps, reserved.
    pub params: [f32; 4],
}
/// Instance vertex stream; its stride includes reserved padding.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Instance {
    /// Placement matrix, locations 3 through 6.
    pub model: [[f32; 4]; 4],
    /// History validity at location 7 and reserved words.
    pub history: [u32; 4],
}
/// Renderer material ABI, written whole by the cooked contribution.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Material {
    /// Base colour.
    pub base_rgba: [f32; 4],
    /// Emissive colour and alpha cutoff.
    pub emissive_cutoff: [f32; 4],
    /// Occlusion strength, roughness, metalness and normal scale.
    pub surface: [f32; 4],
    /// Semantic flags.
    pub flags: [u32; 4],
    /// Texture-array layers.
    pub textures: [u32; 4],
}
/// Actual shader read by the cull pipeline.
pub const CULL_SHADER: &str = include_str!("cull.wgsl");
/// Actual contribution appended to the scene's forward shader.
pub const COOKED_SHADER: &str = include_str!("cooked.wgsl");
