//! Bind-group layout entries for the standard binding kinds, so a layout is written as a list of calls rather than
//! of `wgpu::BindGroupLayoutEntry` literals.
//!
//! Each constructor gives an entry with no count. A buffer entry here has no dynamic offset, and a sampled texture
//! is not multisampled; an entry that needs a count, a dynamic offset or multisampling is written as a literal.
//! A minimum binding size of 0 means none, as `wgpu::BufferSize::new` reads it.

/// A uniform buffer at `binding`, at least `min_size` bytes (0 for no minimum).
pub const fn uniform(
    binding: u32,
    visibility: wgpu::ShaderStages,
    min_size: u64,
) -> wgpu::BindGroupLayoutEntry {
    buffer(
        binding,
        visibility,
        wgpu::BufferBindingType::Uniform,
        min_size,
    )
}

/// A storage buffer at `binding`, read-only or read-write, at least `min_size` bytes (0 for no minimum).
pub const fn storage(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
    min_size: u64,
) -> wgpu::BindGroupLayoutEntry {
    buffer(
        binding,
        visibility,
        wgpu::BufferBindingType::Storage { read_only },
        min_size,
    )
}

/// A buffer of either kind at `binding`, for a layout that picks the kind at run time; at least `min_size` bytes
/// (0 for no minimum).
pub const fn buffer(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
    min_size: u64,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(min_size),
        },
        count: None,
    }
}

/// A single-sampled texture at `binding`, read through `sample_type` with views of `view_dimension`.
pub const fn texture(
    binding: u32,
    visibility: wgpu::ShaderStages,
    sample_type: wgpu::TextureSampleType,
    view_dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension,
            multisampled: false,
        },
        count: None,
    }
}

/// A sampler of kind `ty` at `binding`.
pub const fn sampler(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::SamplerBindingType,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Sampler(ty),
        count: None,
    }
}

/// A storage texture at `binding` of `format`, with views of `view_dimension`, accessed as `access`.
pub const fn storage_texture(
    binding: u32,
    visibility: wgpu::ShaderStages,
    access: wgpu::StorageTextureAccess,
    format: wgpu::TextureFormat,
    view_dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::StorageTexture {
            access,
            format,
            view_dimension,
        },
        count: None,
    }
}

/// A ray-tracing acceleration structure at `binding`, without vertex return.
pub const fn acceleration_structure(
    binding: u32,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::AccelerationStructure {
            vertex_return: false,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests;
