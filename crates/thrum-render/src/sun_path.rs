//! An optional illumination provider for the scene's direct Sun.

use crate::{abi::AbiError, binding};

const FIELD: u32 = 27;
const COLUMNS: u32 = 28;
const SAMPLER: u32 = 29;
const MAPPING: u32 = 30;
const SHADOW_SAMPLER: u32 = 31;
const MAPPING_BYTES: u64 = 48;

pub(crate) const FIELD_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(crate) const COLUMN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// Description of a GPU-written Sun-path field. Its shader defines
/// `sun_path(point, toward_sun)` returning the visibility point, direct
/// factor and transition weight in a `SunPathSample`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SunPathProvider {
    pub(crate) source: &'static str,
    side: u32,
    rungs: u32,
}

impl SunPathProvider {
    /// Describe a square depth ladder and its column map. Preparation checks
    /// these dimensions against the selected device.
    #[must_use]
    pub const fn new(source: &'static str, side: u32, rungs: u32) -> Self {
        Self {
            source,
            side,
            rungs,
        }
    }
}

/// The scene-owned textures and uniform filled by the Sun-path contribution.
#[derive(Clone, Debug)]
pub struct SunPathTarget {
    provider: SunPathProvider,
    /// Irradiance rungs with the full mip chain.
    pub field: wgpu::Texture,
    /// Level and attenuation coefficients at each position in the window.
    pub columns: wgpu::Texture,
    /// The window, Sun and current quality values.
    pub mapping: wgpu::Buffer,
    field_view: wgpu::TextureView,
    column_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,
}

impl SunPathTarget {
    pub(crate) fn new(device: &wgpu::Device, provider: SunPathProvider) -> Result<Self, AbiError> {
        let limits = device.limits();
        if provider.side == 0
            || provider.rungs == 0
            || provider.side > limits.max_texture_dimension_2d
            || provider.rungs > limits.max_texture_array_layers
            || limits.max_bindings_per_bind_group <= SHADOW_SAMPLER
        {
            return Err(AbiError::DeviceLimits);
        }
        let texture = |label, layers, mip_level_count, format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: provider.side,
                    height: provider.side,
                    depth_or_array_layers: layers,
                },
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | usage,
                view_formats: &[],
            })
        };
        let field = texture(
            "Sun-path irradiance field",
            provider.rungs,
            provider.side.ilog2() + 1,
            FIELD_FORMAT,
            wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
        );
        let columns = texture(
            "Sun-path water columns",
            1,
            1,
            COLUMN_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let field_view = field.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let column_view = columns.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sun-path field sampling"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sun-path shadow comparison"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::GreaterEqual),
            ..Default::default()
        });
        let mapping = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sun-path window mapping"),
            size: MAPPING_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            provider,
            field,
            columns,
            mapping,
            field_view,
            column_view,
            sampler,
            shadow_sampler,
        })
    }

    pub(crate) fn accepts(&self, provider: SunPathProvider) -> bool {
        self.provider == provider
    }

    pub(crate) fn source(&self) -> &'static str {
        self.provider.source
    }

    /// Shared reverse-depth comparison for the refracted Sun's visibility.
    #[must_use]
    pub fn shadow_sampler(&self) -> &wgpu::Sampler {
        &self.shadow_sampler
    }

    pub(crate) fn layouts() -> [wgpu::BindGroupLayoutEntry; 5] {
        let stage = wgpu::ShaderStages::FRAGMENT;
        [
            binding::texture(
                FIELD,
                stage,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2Array,
            ),
            binding::texture(
                COLUMNS,
                stage,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
            binding::sampler(SAMPLER, stage, wgpu::SamplerBindingType::Filtering),
            binding::uniform(MAPPING, stage, MAPPING_BYTES),
            binding::sampler(SHADOW_SAMPLER, stage, wgpu::SamplerBindingType::Comparison),
        ]
    }

    pub(crate) fn entries(&self) -> [wgpu::BindGroupEntry<'_>; 5] {
        [
            wgpu::BindGroupEntry {
                binding: FIELD,
                resource: wgpu::BindingResource::TextureView(&self.field_view),
            },
            wgpu::BindGroupEntry {
                binding: COLUMNS,
                resource: wgpu::BindingResource::TextureView(&self.column_view),
            },
            wgpu::BindGroupEntry {
                binding: SAMPLER,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
            wgpu::BindGroupEntry {
                binding: MAPPING,
                resource: self.mapping.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: SHADOW_SAMPLER,
                resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
            },
        ]
    }
}

pub(crate) fn compose(mut scene: String, provider: Option<&str>, shadowed: bool) -> String {
    use std::fmt::Write;

    let Some(source) = provider else {
        return scene;
    };
    if shadowed {
        scene.push_str(crate::shaders::wgsl!("synthetic/forward/point_sun.wgsl"));
    }
    write!(
        scene,
        "\n@group(1) @binding({FIELD}) var sun_path_field: texture_2d_array<f32>;
@group(1) @binding({COLUMNS}) var sun_path_columns: texture_2d<f32>;
@group(1) @binding({SAMPLER}) var sun_path_sampler: sampler;
@group(1) @binding({SHADOW_SAMPLER}) var sun_path_shadow_sampler: sampler_comparison;
struct SunPathSample {{ exit: vec3<f32>, factor: f32, weight: f32 }}
struct SunPathMapping {{ window: vec4<f32>, sun: vec4<f32>, quality: vec4<u32> }}
@group(1) @binding({MAPPING}) var<uniform> sun_path_mapping: SunPathMapping;
{source}",
    )
    .expect("writing to a String");
    scene
}
