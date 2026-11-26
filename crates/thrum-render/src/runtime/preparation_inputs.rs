//! Preserve the creation descriptors needed to key a stage before calling wgpu.

use super::DeviceSite;

impl DeviceSite {
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
