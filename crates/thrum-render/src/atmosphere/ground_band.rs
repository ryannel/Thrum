//! Height-resolved optical depth along the whole Sun path.
use std::sync::Arc;

use crate::{binding, error::RenderError};

// The forward light group uses 19–21 for traced Sun visibility.
pub(crate) const TEXTURE_BINDING: u32 = 22;
pub(crate) const MAPPING_BINDING: u32 = 23;

/// Mapping of a ground band whose rows are measured from the local surface.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GroundBandMapping {
    /// World XZ bounds, from minimum to maximum. The first and last texels
    /// are centred on these bounds.
    pub region: [f32; 4],
    /// Height above the ground at the top node, in metres.
    pub top_m: f32,
    /// Width of the transition to the 2D field, in metres.
    pub border_m: f32,
    /// Reserved for uniform alignment. Set to zero.
    pub padding: [f32; 2],
}

/// A ground band's whole-path optical depth and separate surface visibility.
#[derive(Clone, Debug)]
pub struct GroundBand {
    extent: [u32; 3],
    mapping: GroundBandMapping,
    texels: Option<Arc<[[u16; 4]]>>,
}

impl GroundBand {
    /// Pack an X-fastest grid with axes X, Z and height above the surface.
    /// Every texel supplies optical depth, local surface height in metres and
    /// surface visibility in 0–1. Surface height is the same at every node of
    /// a column. The top slice supplies the matching 2D optical depth.
    ///
    /// # Errors
    /// Rejects inconsistent dimensions, invalid mapping and out-of-range values.
    pub fn new(
        extent: [u32; 3],
        mapping: GroundBandMapping,
        samples: &[[f32; 3]],
    ) -> Result<Self, RenderError> {
        Self::validate(extent, mapping)?;
        let count = extent.into_iter().try_fold(1_usize, |count, dimension| {
            count.checked_mul(dimension as usize)
        });
        let half_limit = crate::shader_constants::MAX_FINITE_HALF;
        if count != Some(samples.len())
            || samples.iter().any(|[tau, ground, visibility]| {
                !(0.0..=half_limit).contains(tau)
                    || !(-half_limit..=half_limit).contains(ground)
                    || !(0.0..=1.0).contains(visibility)
            })
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            extent,
            mapping,
            texels: Some(
                samples
                    .iter()
                    .map(|[tau, ground, visibility]| {
                        [*tau, *ground, *visibility, 0.0]
                            .map(|value| half::f16::from_f32(value).to_bits())
                    })
                    .collect(),
            ),
        })
    }

    fn validate(extent: [u32; 3], mapping: GroundBandMapping) -> Result<(), RenderError> {
        let [min_x, min_z, max_x, max_z] = mapping.region;
        if extent.iter().any(|size| *size < 2)
            || !mapping.region.iter().all(|value| value.is_finite())
            || min_x >= max_x
            || min_z >= max_z
            || !mapping.top_m.is_finite()
            || mapping.top_m <= 0.0
            || !mapping.border_m.is_finite()
            || mapping.border_m <= 0.0
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(())
    }

    /// Declare a scene-allocated GPU target filled by a contribution.
    ///
    /// # Errors
    /// Rejects invalid dimensions and mapping. Device limits are checked at preparation.
    pub fn gpu_target(extent: [u32; 3], mapping: GroundBandMapping) -> Result<Self, RenderError> {
        Self::validate(extent, mapping)?;
        Ok(Self {
            extent,
            mapping,
            texels: None,
        })
    }

    pub(super) fn accepts(&self, other: &Self) -> bool {
        self.extent == other.extent && self.texels.is_some() == other.texels.is_some()
    }
}

#[derive(Clone)]
pub(crate) struct GroundBandPrepared {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    mapping: wgpu::Buffer,
}

impl GroundBandPrepared {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        band: &GroundBand,
    ) -> Result<Self, RenderError> {
        if band
            .extent
            .iter()
            .any(|&size| size > device.limits().max_texture_dimension_3d)
        {
            return Err(RenderError::CloudTextureExtent);
        }
        let [width, height, depth_or_array_layers] = band.extent;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sun-path ground band"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | if band.texels.is_some() {
                    wgpu::TextureUsages::COPY_DST
                } else {
                    wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC
                },
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mapping = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sun-path ground band mapping"),
            size: size_of::<GroundBandMapping>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let result = Self {
            texture,
            view,
            mapping,
        };
        result.update(queue, band, None);
        Ok(result)
    }

    pub(super) fn update(&self, queue: &wgpu::Queue, band: &GroundBand, old: Option<&GroundBand>) {
        if let Some(texels) = &band.texels
            && old
                .and_then(|old| old.texels.as_ref())
                .is_none_or(|old| !Arc::ptr_eq(old, texels))
        {
            let bytes: Vec<_> = texels
                .iter()
                .flatten()
                .flat_map(|word| word.to_le_bytes())
                .collect();
            queue.write_texture(
                self.texture.as_image_copy(),
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8 * band.extent[0]),
                    rows_per_image: Some(band.extent[1]),
                },
                self.texture.size(),
            );
        }
        let mapping = bytemuck::bytes_of(&band.mapping);
        if old.is_none_or(|old| bytemuck::bytes_of(&old.mapping) != mapping) {
            queue.write_buffer(&self.mapping, 0, mapping);
        }
    }

    pub(crate) fn target(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.texture
            .usage()
            .contains(wgpu::TextureUsages::STORAGE_BINDING)
            .then(|| (self.texture.clone(), self.mapping.clone()))
    }

    pub(crate) fn layout(stages: wgpu::ShaderStages) -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            binding::texture(
                TEXTURE_BINDING,
                stages,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D3,
            ),
            binding::uniform(
                MAPPING_BINDING,
                stages,
                size_of::<GroundBandMapping>() as u64,
            ),
        ]
    }

    pub(crate) fn entries(&self) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: TEXTURE_BINDING,
                resource: wgpu::BindingResource::TextureView(&self.view),
            },
            wgpu::BindGroupEntry {
                binding: MAPPING_BINDING,
                resource: self.mapping.as_entire_binding(),
            },
        ]
    }

    pub(crate) fn append_reads(
        &self,
        graph: &mut crate::graph::FrameGraph,
        accesses: &mut Vec<crate::graph::Access>,
        produced: Option<&crate::graph::Access>,
    ) -> Result<(), crate::graph::GraphError> {
        use crate::graph::{Access, AccessKind, ResourceDescriptor, ResourceScope, TextureKind};
        let scope = ResourceScope::Imported { initialized: true };
        let texture = graph.resource(
            ResourceDescriptor::Texture {
                dimension: TextureKind::D3 {
                    depth: self.texture.depth_or_array_layers(),
                },
                width: self.texture.width(),
                height: self.texture.height(),
                layers: 1,
                mips: 1,
                samples: 1,
                format: self.texture.format(),
                usage: self.texture.usage(),
            },
            scope,
        )?;
        let bytes = self.mapping.size();
        let mapping = graph.resource(
            ResourceDescriptor::Buffer {
                bytes,
                usage: self.mapping.usage(),
            },
            scope,
        )?;
        accesses.extend([
            produced
                .cloned()
                .unwrap_or_else(|| Access::new(texture, 0..1, AccessKind::SampledRead)),
            Access::new(mapping, 0..bytes, AccessKind::UniformRead),
        ]);
        Ok(())
    }
}
