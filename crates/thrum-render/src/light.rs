//! Shared ABI for CPU-authored direct light and the atmosphere's diffuse output.

#[expect(
    clippy::cast_possible_truncation,
    reason = "The same Rec.709 weights serve f64 validation and f32 rendering"
)]
pub(crate) const LUMA: glam::Vec3 = {
    let [r, g, b] = crate::shader_constants::REC709_LUMA;
    glam::Vec3::new(r as f32, g as f32, b as f32)
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct DirectionalLight {
    /// The final lane enables clustered local lights.
    pub(crate) direction_to_light: [f32; 4],
    pub(crate) irradiance: [f32; 4],
    pub(crate) half_vector_fresnel: [f32; 4],
    pub(crate) ambient_irradiance: [f32; 4],
}

impl DirectionalLight {
    pub(crate) fn with_local_lights(mut self, active: bool) -> Self {
        self.direction_to_light[3] = f32::from(active);
        self
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct EnvironmentLight {
    /// The final lane is zero until valid, then one plus the negative probe count.
    pub(crate) horizontal_irradiance: [f32; 4],
    pub(crate) sky_diffuse: [[f32; 4]; 9],
}

pub(crate) const DIRECTIONAL_BYTES: u64 = size_of::<DirectionalLight>() as u64;
pub(crate) const ENVIRONMENT_BYTES: u64 = size_of::<EnvironmentLight>() as u64;
pub(crate) const ENVIRONMENT_BINDING: u32 = 18;

#[cfg(test)]
mod tests;
