use crate::{binding, records::VolumeHistory, shaders::wgsl};

/// Scene-owned target for a provider's medium coefficients. Its RGBA16F
/// channels hold grey extinction in inverse metres, the grey solar kernel
/// and grey ambient source per unit solar illuminance per metre, and padding.
/// Every frame the provider writes all cells using the leased sample points.
#[derive(Clone)]
pub struct ViewVolumeMediumTarget {
    /// A sampled/storage 2D array, in view-volume cell order.
    pub coefficients: wgpu::Texture,
    /// The bind group zero inputs consumed by the sampling WGSL.
    pub observer: wgpu::Buffer,
    /// The submitted-history and current jitter record consumed by the sampling WGSL.
    pub history: wgpu::Buffer,
}

impl ViewVolumeMediumTarget {
    /// Provider ABI for group zero. Append a compute entry point and write
    /// coefficients with `textureStore(medium_out, ...)` for every cell.
    /// `medium_sample_point` returns world XYZ and sample distance in metres.
    #[must_use]
    pub fn shader_source(&self) -> String {
        format!(
            "{}{}{}{}",
            wgsl!("records/observer.wgsl"),
            wgsl!("atmosphere/volume_history.wgsl"),
            super::view_volume::grid_source_with_near(self.coefficients.depth_or_array_layers()),
            wgsl!("atmosphere/volume_medium_target.wgsl"),
        )
    }

    /// Layout for the sampler ABI at bind group zero.
    #[must_use]
    pub fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 3] {
        let stages = wgpu::ShaderStages::COMPUTE;
        [
            binding::uniform(0, stages, super::UNIFORM_BYTES),
            binding::uniform(1, stages, size_of::<VolumeHistory>() as u64),
            storage(2),
        ]
    }

    /// Bind the current prepared generation's target and sample records.
    #[must_use]
    pub fn bind(&self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view volume medium target"),
            layout,
            entries: &[
                super::view_volume::buffer_entry(0, &self.observer),
                super::view_volume::buffer_entry(1, &self.history),
                super::view_volume::texture_entry(2, &array_view(&self.coefficients)),
            ],
        })
    }
}

pub(super) fn array_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

fn storage(index: u32) -> wgpu::BindGroupLayoutEntry {
    binding::storage_texture(
        index,
        wgpu::ShaderStages::COMPUTE,
        wgpu::StorageTextureAccess::WriteOnly,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureViewDimension::D2Array,
    )
}

pub(super) fn coefficients(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    layers: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("view volume medium coefficients"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
