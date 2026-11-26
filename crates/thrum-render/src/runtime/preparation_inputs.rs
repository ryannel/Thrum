//! Preserve the creation descriptors needed to key a stage before calling wgpu.

use super::DeviceSite;

impl DeviceSite {
    /// Create a shader module and remember its WGSL identity during Metal preparation.
    pub fn create_shader_module(
        &self,
        desc: wgpu::ShaderModuleDescriptor<'_>,
    ) -> wgpu::ShaderModule {
        let preparation = self.metal_preparation();
        let source = preparation.and_then(|_| match &desc.source {
            wgpu::ShaderSource::Wgsl(source) => Some(blake3::hash(source.as_bytes())),
            _ => None,
        });
        let shader = wgpu::Device::create_shader_module(self, desc);
        if let (Some(preparation), Some(source)) = (preparation, source) {
            preparation.shader(shader.clone(), source);
        }
        shader
    }

    /// Create a bind group layout and remember the entries used by the pipeline layout.
    pub fn create_bind_group_layout(
        &self,
        desc: &wgpu::BindGroupLayoutDescriptor<'_>,
    ) -> wgpu::BindGroupLayout {
        let layout = wgpu::Device::create_bind_group_layout(self, desc);
        if let Some(preparation) = self.metal_preparation() {
            preparation.bindings(layout.clone(), desc.entries);
        }
        layout
    }

    /// Create a pipeline layout and remember the groups used by the translator.
    pub fn create_pipeline_layout(
        &self,
        desc: &wgpu::PipelineLayoutDescriptor<'_>,
    ) -> wgpu::PipelineLayout {
        let layout = wgpu::Device::create_pipeline_layout(self, desc);
        if let Some(preparation) = self.metal_preparation() {
            preparation.layout(layout.clone(), desc);
        }
        layout
    }
}
