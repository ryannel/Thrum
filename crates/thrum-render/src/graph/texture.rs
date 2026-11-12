//! Texture-kind and binding validation, performed only during preparation.
use super::{
    Access, AccessKind, CompiledGraph, FrameGraph, GraphError, PipelineKey, ResourceDescriptor,
};
use crate::capabilities::DeviceCapabilities;
use std::collections::BTreeSet;

/// Physical texture kind. Cube faces are layers; volume depth shrinks per mip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureKind {
    /// Two-dimensional texture or array, selected by the descriptor's layers.
    D2,
    /// Three-dimensional texture with one logical layer.
    D3 {
        /// Base-level depth.
        depth: u32,
    },
    /// Six square faces, sampled as a cube and written through array views.
    Cube,
}
impl TextureKind {
    pub(super) fn depth(self) -> u32 {
        match self {
            Self::D3 { depth } => depth,
            Self::D2 | Self::Cube => 1,
        }
    }
    pub(super) fn physical(self) -> wgpu::TextureDimension {
        match self {
            Self::D3 { .. } => wgpu::TextureDimension::D3,
            Self::D2 | Self::Cube => wgpu::TextureDimension::D2,
        }
    }
    fn view(self, layers: u32) -> wgpu::TextureViewDimension {
        match self {
            Self::D3 { .. } => wgpu::TextureViewDimension::D3,
            Self::Cube => wgpu::TextureViewDimension::Cube,
            Self::D2 if layers == 1 => wgpu::TextureViewDimension::D2,
            Self::D2 => wgpu::TextureViewDimension::D2Array,
        }
    }
}

pub(super) fn validate_descriptor(descriptor: &ResourceDescriptor) -> Result<(), GraphError> {
    if let ResourceDescriptor::Texture {
        dimension,
        width,
        height,
        layers,
        samples,
        ..
    } = descriptor
    {
        match dimension {
            TextureKind::D3 { depth } if *depth == 0 || *layers != 1 || *samples != 1 => {
                return Err(GraphError::InvalidVolume);
            }
            TextureKind::Cube if *width != *height || *layers != 6 || *samples != 1 => {
                return Err(GraphError::InvalidCube);
            }
            _ => (),
        }
    }
    Ok(())
}

fn storage(kind: AccessKind) -> bool {
    matches!(
        kind,
        AccessKind::StorageRead | AccessKind::StorageWrite | AccessKind::StorageReadWrite
    )
}

pub(super) fn validate_access(
    descriptor: &ResourceDescriptor,
    access: &Access,
    pass: usize,
) -> Result<(), GraphError> {
    let ResourceDescriptor::Texture {
        dimension,
        layers,
        samples,
        ..
    } = descriptor
    else {
        return if access.view.is_some() {
            Err(GraphError::InvalidTextureView { pass })
        } else {
            Ok(())
        };
    };
    if matches!(access.kind, AccessKind::CopyRead | AccessKind::CopyWrite) {
        return Ok(());
    }
    let attachment = matches!(
        access.kind,
        AccessKind::AttachmentClear | AccessKind::AttachmentLoad
    );
    if attachment && matches!(dimension, TextureKind::D3 { .. }) {
        return Err(GraphError::VolumeAttachment { pass });
    }
    let view = access.view.unwrap_or_else(|| dimension.view(*layers));
    if storage(access.kind)
        && matches!(
            view,
            wgpu::TextureViewDimension::Cube | wgpu::TextureViewDimension::CubeArray
        )
    {
        return Err(GraphError::CubeStorageView { pass });
    }
    let count = access.range.end - access.range.start;
    let one_mip =
        access.range.start / u64::from(*layers) == (access.range.end - 1) / u64::from(*layers);
    let compatible = match view {
        wgpu::TextureViewDimension::D3 => matches!(dimension, TextureKind::D3 { .. }),
        wgpu::TextureViewDimension::D2 => {
            !matches!(dimension, TextureKind::D3 { .. })
                && (*layers == 1 || (one_mip && count == 1))
        }
        wgpu::TextureViewDimension::D2Array => {
            !matches!(dimension, TextureKind::D3 { .. })
                && (one_mip
                    || (access.range.start.is_multiple_of(u64::from(*layers))
                        && count.is_multiple_of(u64::from(*layers))))
        }
        wgpu::TextureViewDimension::Cube => {
            *dimension == TextureKind::Cube
                && count.is_multiple_of(6)
                && access.range.start.is_multiple_of(6)
        }
        _ => false,
    };
    if !compatible
        || (storage(access.kind) && (*samples != 1 || (access.view.is_some() && !one_mip)))
        || (attachment
            && (!one_mip
                || !matches!(
                    view,
                    wgpu::TextureViewDimension::D2 | wgpu::TextureViewDimension::D2Array
                )))
    {
        return Err(GraphError::InvalidTextureView { pass });
    }
    Ok(())
}

impl FrameGraph {
    /// Compile against the immutable capability record of the target generation.
    /// Use this for device-bound graphs; `compile` alone validates topology only.
    /// # Errors
    /// Refuses unsupported extents, storage formats/counts and invalid declarations.
    pub fn compile_with_capabilities(
        &self,
        capabilities: &DeviceCapabilities,
        prepared: &BTreeSet<PipelineKey>,
    ) -> Result<CompiledGraph, GraphError> {
        self.validate_capabilities(capabilities)?;
        self.compile(capabilities.generation(), prepared)
    }

    /// Validate texture accesses against enabled generation facts, without probing
    /// the device. Surface variants share this preparation-time validation.
    /// # Errors
    /// Refuses invalid declarations and unsupported texture extents/storage access.
    pub fn validate_capabilities(
        &self,
        capabilities: &DeviceCapabilities,
    ) -> Result<(), GraphError> {
        self.validate_declarations()?;
        let limits = capabilities.limits();
        for resource in &self.resources {
            if let ResourceDescriptor::Texture {
                dimension,
                width,
                height,
                layers,
                ..
            } = resource.descriptor
            {
                let limit = if matches!(dimension, TextureKind::D3 { .. }) {
                    limits.max_texture_dimension_3d
                } else {
                    limits.max_texture_dimension_2d
                };
                if width > limit
                    || height > limit
                    || dimension.depth() > limit
                    || layers > limits.max_texture_array_layers
                {
                    return Err(GraphError::TextureExtentLimit);
                }
            }
        }
        for (pass, declaration) in self.passes.iter().enumerate() {
            let mut count = 0;
            for access in &declaration.accesses {
                if !storage(access.kind) {
                    continue;
                }
                if let ResourceDescriptor::Texture { format, .. } =
                    self.resources[access.resource.index].descriptor
                {
                    count += 1;
                    let facts = capabilities.texture_format(format);
                    let flag = match access.kind {
                        AccessKind::StorageRead => {
                            wgpu::TextureFormatFeatureFlags::STORAGE_READ_ONLY
                        }
                        AccessKind::StorageWrite => {
                            wgpu::TextureFormatFeatureFlags::STORAGE_WRITE_ONLY
                        }
                        _ => wgpu::TextureFormatFeatureFlags::STORAGE_READ_WRITE,
                    };
                    if !facts
                        .allowed_usages
                        .contains(wgpu::TextureUsages::STORAGE_BINDING)
                        || !facts.flags.contains(flag)
                    {
                        return Err(GraphError::StorageFormat { pass });
                    }
                }
            }
            if count > limits.max_storage_textures_per_shader_stage {
                return Err(GraphError::StorageTextureLimit { pass });
            }
        }
        Ok(())
    }
}
