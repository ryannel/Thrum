//! Prepared in-memory instanced diagnostic. It exercises the real material and
//! instance tables, four texture-class bindings, graph declaration and optional
//! timestamps. One directional metallic-roughness closure and opaque depth are
//! real, followed by a fixed authored SDR display transform. Fixed-view
//! clustered local lights and two synthetic Sun cascades are prepared here;
//! a checked constant-light presentation handoff and the general render scene
//! remain open.

use crate::abi::{self, AbiError, InstanceRecord, MaterialRecord, TextureClasses};
use crate::graph::{
    Access, AccessKind, CompiledGraph, FrameGraph, GraphError, Pass, PassScope, PipelineKey,
    ResourceDescriptor, ResourceScope,
};
use crate::timing::{GpuTimings, TimingError};
use glam::Vec3;
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use wgpu::util::DeviceExt;

mod cluster;
mod shadow;
mod upload;
mod view;
use cluster::ClusterTable;
pub use cluster::SyntheticPointLight;
use shadow::{ShadowPrepared, ShadowTarget};
use upload::{InstanceUpload, pack_synthetic_instance};
use view::AUX_BYTES;
pub use view::SyntheticView;

/// Synthetic source revision; not a substitute for an F4 product identity.
pub const SYNTHETIC_REVISION: u64 = 6;
/// Portable opaque depth; no sampling or separate prepass is needed.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const VIEW_BYTES: usize = 160;
const LIGHT_BYTES: usize = 64;
const PRE_EXPOSURE: f32 = 1.0 / 1024.0;
const DISPLAY_BYTES: u64 = 16;
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Explicit shader sRGB encoding after exposure, tone and gamut mapping.
pub const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Immutable CPU scene, converted at prepare time rather than each draw.
#[derive(Clone, Debug)]
pub struct SyntheticScene {
    /// Nonempty storage-buffer material table.
    pub materials: Vec<MaterialRecord>,
    /// Nonempty indexed draw instances.
    pub instances: Vec<InstanceRecord>,
    /// Fixed-view local sources for the synthetic diagnostic. Game light
    /// ownership and live extraction belong to the later V3 scene bridge.
    pub local_lights: Vec<SyntheticPointLight>,
    /// Unjittered camera used for this prepared view.
    pub view: SyntheticView,
    /// Prepare two finite perspective Sun cascades; off avoids their maps and passes.
    pub sun_shadows: bool,
}
impl SyntheticScene {
    /// Generate a bounded regular grid. Instance IDs remain stable; one draw
    /// renders all cubes. The default view uses the historical orthographic
    /// diagnostic coordinates; callers may supply a checked perspective view.
    /// # Errors
    /// Refuses zero instances or more than one million diagnostic instances.
    pub fn grid(count: u32) -> Result<Self, RenderError> {
        if count == 0 || count > 1_000_000 {
            return Err(RenderError::SceneCapacity);
        }
        let side = (count - 1).isqrt() + 1;
        let cell = 1.8 / side as f32;
        let materials = vec![
            MaterialRecord {
                base_rgba: [0.8, 0.2, 0.1, 1.0],
                ..Default::default()
            },
            MaterialRecord {
                base_rgba: [0.1, 0.5, 0.9, 1.0],
                surface: [1.0, 0.3, 0.2, 1.0],
                ..Default::default()
            },
        ];
        let instances = (0..count)
            .map(|index| {
                let mut instance = InstanceRecord::identity(index, index % 2);
                // A fixed orthonormal rotation exposes three differently lit faces.
                let (sx, cx) = 0.35_f32.sin_cos();
                let (sy, cy) = 0.5_f32.sin_cos();
                let columns = [
                    [cy, 0.0, -sy],
                    [sy * sx, cx, cy * sx],
                    [sy * cx, -sx, cy * cx],
                ];
                let scale = (cell * 0.3).min(0.3);
                for (axis, column) in columns.into_iter().enumerate() {
                    for (row, value) in column.into_iter().enumerate() {
                        instance.current[axis][row] = value * scale;
                        instance.normal[axis][row] = value / scale;
                    }
                }
                instance.current[3][2] = 0.5;
                instance.current[3][0] = -0.9 + cell * (index % side) as f32 + cell * 0.5;
                instance.current[3][1] = -0.9 + cell * (index / side) as f32 + cell * 0.5;
                instance.previous = instance.current;
                instance
            })
            .collect();
        Ok(Self {
            materials,
            instances,
            local_lights: Vec::new(),
            view: SyntheticView::default(),
            sun_shadows: false,
        })
    }
}

/// Bounded in-memory directional illumination, in linear working RGB. Direction
/// points from the receiver toward the source; irradiance is for a perpendicular
/// receiver, before cosine. No physical-world light registry is created here.
#[derive(Clone, Copy, Debug)]
pub struct SyntheticLight {
    /// Nonzero finite direction; normalized once during preparation.
    pub direction_to_light: [f32; 3],
    /// Finite nonnegative RGB illuminance, each at most 10,000 diagnostic units.
    pub irradiance: [f32; 3],
}
impl Default for SyntheticLight {
    fn default() -> Self {
        Self {
            direction_to_light: [0.4, -0.6, -1.0],
            irradiance: [4.0; 3],
        }
    }
}
impl SyntheticLight {
    fn pack(self) -> Result<[u8; LIGHT_BYTES], RenderError> {
        if !self
            .irradiance
            .iter()
            .all(|v| v.is_finite() && (0.0..=10_000.0).contains(v))
        {
            return Err(RenderError::InvalidLighting);
        }
        pack_directional(
            self.direction_to_light.map(f64::from),
            self.irradiance,
            [0.0; 3],
        )
    }
}

/// Photometric constant-light quantities from a checked cooked product.
/// The fields retain source units until one finite GPU preparation.
#[derive(Clone, Copy, Debug)]
pub struct PhotometricEnvironment {
    /// Geometric receiver-to-Sun world direction.
    pub toward_sun: [f64; 3],
    /// Perpendicular Sun illuminance in lux.
    pub sun_lux: f64,
    /// Nonnegative linear Rec.709 colour with photometric luminance one.
    pub sun_colour: [f64; 3],
    /// Isotropic hemispherical incident irradiance in lux.
    pub ambient_lux: [f64; 3],
    /// Independent terminal background radiance in cd/m².
    pub background_radiance: [f64; 3],
}

enum Lighting {
    Diagnostic(SyntheticLight),
    Photometric(PhotometricEnvironment),
}

impl PhotometricEnvironment {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "validated and presentation-clamped f64 authored quantities round once to the f32 GPU ABI"
    )]
    fn pack(self) -> Result<([u8; LIGHT_BYTES], wgpu::Color, usize), RenderError> {
        const MAX_PRESENTATION_LIGHT: f64 = 10_000_000.0;
        // Retain F3's exact accepted predicate at this boundary.
        let luminance =
            0.2126 * self.sun_colour[0] + 0.7152 * self.sun_colour[1] + 0.0722 * self.sun_colour[2];
        if !self.sun_lux.is_finite()
            || self.sun_lux < 0.0
            || !self.sun_colour.iter().all(|x| x.is_finite() && *x >= 0.0)
            || (luminance - 1.0).abs() > 1e-9
            || !self.ambient_lux.iter().all(|x| x.is_finite() && *x >= 0.0)
            || !self
                .background_radiance
                .iter()
                .all(|x| x.is_finite() && *x >= 0.0)
        {
            return Err(RenderError::InvalidLighting);
        }
        let mut clamps = 0;
        let mut limit = |value: f64| {
            if value > MAX_PRESENTATION_LIGHT {
                clamps += 1;
                MAX_PRESENTATION_LIGHT as f32
            } else {
                value as f32
            }
        };
        let sun = self.sun_colour.map(|channel| limit(self.sun_lux * channel));
        let ambient = self.ambient_lux.map(&mut limit);
        let background = self.background_radiance.map(limit);
        let bytes = pack_directional(self.toward_sun, sun, ambient)?;
        Ok((
            bytes,
            wgpu::Color {
                r: f64::from(background[0]) * f64::from(PRE_EXPOSURE),
                g: f64::from(background[1]) * f64::from(PRE_EXPOSURE),
                b: f64::from(background[2]) * f64::from(PRE_EXPOSURE),
                a: 1.0,
            },
            clamps,
        ))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "validated finite direction rounds once to the f32 shader ABI"
)]
fn pack_directional(
    toward_source: [f64; 3],
    irradiance: [f32; 3],
    ambient: [f32; 3],
) -> Result<[u8; LIGHT_BYTES], RenderError> {
    let length = toward_source.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length == 0.0 {
        return Err(RenderError::InvalidLighting);
    }
    let direction = toward_source.map(|value| value / length);
    // Orthographic receiver-to-camera direction is fixed at -Z, so this
    // precomputed half-vector is used only by the orthographic diagnostic.
    // f64 preparation avoids f32 squared-length underflow near opposition.
    let half = [direction[0], direction[1], direction[2] - 1.0];
    let half_length = half.iter().map(|v| v * v).sum::<f64>().sqrt();
    let half = if half_length > 0.0 {
        half.map(|v| v / half_length)
    } else {
        [0.0, 0.0, -1.0]
    };
    let angular_fresnel = (1.0 + half[2]).clamp(0.0, 1.0).powi(5) as f32;
    let mut bytes = [0; LIGHT_BYTES];
    for i in 0..3 {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&(direction[i] as f32).to_le_bytes());
        bytes[32 + i * 4..36 + i * 4].copy_from_slice(&(half[i] as f32).to_le_bytes());
    }
    bytes[44..48].copy_from_slice(&angular_fresnel.to_le_bytes());
    for (i, value) in irradiance.into_iter().enumerate() {
        bytes[16 + i * 4..20 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    for (i, value) in ambient.into_iter().enumerate() {
        bytes[48 + i * 4..52 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}

fn packed_light_direction(data: &[u8; LIGHT_BYTES]) -> Vec3 {
    let words = data[..12].as_chunks::<4>().0;
    Vec3::new(
        f32::from_le_bytes(words[0]),
        f32::from_le_bytes(words[1]),
        f32::from_le_bytes(words[2]),
    )
}

/// Expected failures before a view can be encoded or published ready.
#[derive(Debug)]
pub enum RenderError {
    /// ABI validation failed.
    Abi(AbiError),
    /// Typed graph failed validation.
    Graph(GraphError),
    /// Optional timestamp lifecycle failure.
    Timing(TimingError),
    /// Pipeline/resource creation was rejected by the device.
    Validation(String),
    /// Empty/unreasonably large synthetic fixture.
    SceneCapacity,
    /// Changed instance slots were out of bounds, duplicated or unordered.
    InvalidInstanceUpdate,
    /// Unsupported semantic flags, skinning or mapped normals in this diagnostic.
    UnsupportedSyntheticMaterial,
    /// Invalid directional source or degenerate/unsupported diagnostic transform.
    InvalidLighting,
    /// Nonfinite or degenerate camera projection.
    InvalidView,
    /// Authored exposure or its required reciprocal/storage gain is not positive finite.
    InvalidExposure,
    /// Target/readback belongs to another generation, grid or prepared exposure.
    StaleTarget,
    /// Target dimensions exceed device limits or overflow readback arithmetic.
    TargetExtent,
    /// Readback is already pending or failed.
    Readback,
}
impl From<AbiError> for RenderError {
    fn from(error: AbiError) -> Self {
        Self::Abi(error)
    }
}
impl From<GraphError> for RenderError {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}
impl From<TimingError> for RenderError {
    fn from(error: TimingError) -> Self {
        Self::Timing(error)
    }
}

/// Work enqueued for one instance-table update. Bytes count payload sent to
/// the queue, not driver staging overhead or GPU transfer time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstanceUploadReport {
    /// Rows whose packed ABI bytes changed since the preceding submitted view.
    pub changed_rows: usize,
    /// Malformed presentation rows kept at their previously valid GPU value.
    pub dropped_invalid: usize,
    /// Active casters flattened to a prepared cascade depth plane.
    pub clamped_shadow_casters: usize,
    /// Queue write calls made for this update.
    pub writes: usize,
    /// Payload bytes enqueued, including unchanged rows in a full fallback.
    pub bytes: usize,
}

/// Fully prepared opaque/static forward and SDR display roles. Both pipelines
/// finish creation before this object is returned; encode never builds them.
pub struct PreparedSynthetic {
    key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    display_pipeline: wgpu::RenderPipeline,
    display_layout: wgpu::BindGroupLayout,
    view_layout: wgpu::BindGroupLayout,
    view_buffer: wgpu::Buffer,
    view_sample: SyntheticView,
    light: wgpu::BindGroup,
    materials: wgpu::BindGroup,
    instances: wgpu::BindGroup,
    instance_upload: InstanceUpload,
    material_bytes: u64,
    instance_bytes: u64,
    instance_count: u32,
    display_gain: f32,
    exposure_bits: u32,
    local_light_bytes: u64,
    cluster_head_bytes: u64,
    cluster_index_bytes: u64,
    dropped_light_references: usize,
    presentation_light_clamps: usize,
    background: wgpu::Color,
    shadow: Option<ShadowPrepared>,
}
impl PreparedSynthetic {
    /// Prepare on the V1 executor's existing device and upload queue. No private
    /// adapter, device, submission or cook authority is created here.
    /// # Errors
    /// Refuses invalid/unsupported scene data, device capacity or GPU validation failure.
    pub async fn prepare(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_light(device, queue, generation, scene, SyntheticLight::default()).await
    }
    /// Prepare the same finite draw role with an explicit directional source.
    /// Zero irradiance disables direct lighting; emission remains independent.
    /// # Errors
    /// Refuses invalid lights/transforms/materials, device capacity or GPU validation failure.
    pub async fn prepare_with_light(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
        light: SyntheticLight,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_light_and_exposure(device, queue, generation, scene, light, 1.0).await
    }
    /// Prepare with one authored positive exposure shared by the ABI2 view and
    /// target display transform. Exposure is immutable after preparation.
    /// # Errors
    /// Refuses invalid exposure or its derived reciprocals, unsupported scene
    /// data, device capacity or GPU validation failure.
    pub async fn prepare_with_light_and_exposure(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
        light: SyntheticLight,
        exposure: f32,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_lighting(
            device,
            queue,
            generation,
            scene,
            Lighting::Diagnostic(light),
            exposure,
        )
        .await
    }

    /// Prepare a cooked photometric constant-light generation on this device.
    ///
    /// # Errors
    ///
    /// Refuses invalid authored quantities, view or GPU preparation.
    pub async fn prepare_with_photometric_environment(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
        environment: PhotometricEnvironment,
        exposure: f32,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_lighting(
            device,
            queue,
            generation,
            scene,
            Lighting::Photometric(environment),
            exposure,
        )
        .await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one finite pipeline preparation transaction keeps resources, layouts and its validation scope together"
    )]
    async fn prepare_with_lighting(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
        lighting: Lighting,
        exposure: f32,
    ) -> Result<Self, RenderError> {
        let display_gain = checked_display_gain(exposure)?;
        let (mut light_data, background, presentation_light_clamps) = match lighting {
            Lighting::Diagnostic(light) => (
                light.pack()?,
                wgpu::Color {
                    r: 0.02 * f64::from(PRE_EXPOSURE),
                    g: 0.03 * f64::from(PRE_EXPOSURE),
                    b: 0.04 * f64::from(PRE_EXPOSURE),
                    a: 1.0,
                },
                0,
            ),
            Lighting::Photometric(environment) => environment.pack()?,
        };
        let shadow_enabled = scene.sun_shadows
            && light_data[16..28]
                .as_chunks::<4>()
                .0
                .iter()
                .any(|word| f32::from_le_bytes(*word) > 0.0);
        let clusters = ClusterTable::build(&scene.local_lights, &scene.view)?;
        let has_local_lights: f32 = if clusters.active { 1.0 } else { 0.0 };
        light_data[12..16].copy_from_slice(&has_local_lights.to_le_bytes());
        let local_light_bytes = clusters.lights.len() as u64;
        let cluster_head_bytes = clusters.heads.len() as u64;
        let cluster_index_bytes = clusters.indices.len() as u64;
        let dropped_light_references = clusters.dropped_references;
        let limits = device.limits();
        if [local_light_bytes, cluster_head_bytes, cluster_index_bytes]
            .into_iter()
            .any(|bytes| bytes > limits.max_storage_buffer_binding_size)
        {
            return Err(RenderError::SceneCapacity);
        }
        let classes = TextureClasses::defaults(&limits)?;
        let (material_bytes, instance_bytes) =
            abi::table_bytes(scene.materials.len(), scene.instances.len(), &limits)?;
        let material_count =
            u32::try_from(scene.materials.len()).map_err(|_| RenderError::SceneCapacity)?;
        let instance_count =
            u32::try_from(scene.instances.len()).map_err(|_| RenderError::SceneCapacity)?;
        let mut packed_materials = Vec::with_capacity(
            usize::try_from(material_bytes).map_err(|_| RenderError::SceneCapacity)?,
        );
        for material in &scene.materials {
            // This is a single opaque static closure, not admission for all PBR flags.
            if material.flags != [0; 4] || material.texture_layers != [0; 4] {
                return Err(RenderError::UnsupportedSyntheticMaterial);
            }
            packed_materials.extend_from_slice(&material.pack(&classes)?);
        }
        let mut packed_instances = Vec::with_capacity(
            usize::try_from(instance_bytes).map_err(|_| RenderError::SceneCapacity)?,
        );
        for instance in &scene.instances {
            packed_instances.extend_from_slice(&pack_synthetic_instance(instance, material_count)?);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let material_buffer =
            storage_buffer(device, queue, "synthetic materials", &packed_materials);
        let instance_buffer =
            storage_buffer(device, queue, "synthetic forward opaque", &packed_instances);
        let mut instance_upload = InstanceUpload::new(
            instance_buffer,
            packed_instances,
            material_count,
            instance_count,
        );
        let view_data = scene.view.pack_view(exposure);
        let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic ABI2 view"),
            contents: &view_data,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let mut view_entries = vec![
            uniform_layout(
                0,
                VIEW_BYTES as u64,
                wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ),
            uniform_layout(1, AUX_BYTES as u64, wgpu::ShaderStages::FRAGMENT),
        ];
        if shadow_enabled {
            view_entries.extend([
                uniform_layout(2, shadow::SAMPLE_BYTES as u64, wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ]);
        }
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic view and target aux"),
            entries: &view_entries,
        });
        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic directional and clustered sources"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(LIGHT_BYTES as u64),
                    },
                    count: None,
                },
                storage_layout(1, wgpu::ShaderStages::FRAGMENT, 32),
                storage_layout(2, wgpu::ShaderStages::FRAGMENT, 8),
                storage_layout(3, wgpu::ShaderStages::FRAGMENT, 4),
            ],
        });
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic directional source"),
            size: LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&directional, 0, &light_data);
        let local_lights =
            storage_buffer(device, queue, "synthetic local lights", &clusters.lights);
        let cluster_heads =
            storage_buffer(device, queue, "synthetic cluster heads", &clusters.heads);
        let cluster_indices = storage_buffer(
            device,
            queue,
            "synthetic cluster indices",
            &clusters.indices,
        );
        let light = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic clustered light lists"),
            layout: &light_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: directional.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: local_lights.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: cluster_heads.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: cluster_indices.as_entire_binding(),
                },
            ],
        });
        let mut entries = vec![storage_layout(
            0,
            wgpu::ShaderStages::FRAGMENT,
            abi::MATERIAL_BYTES as u64,
        )];
        for binding in 1..=4 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ABI2 material table and classes"),
            entries: &entries,
        });
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ABI2 instance table"),
            entries: &[storage_layout(
                0,
                wgpu::ShaderStages::VERTEX,
                abi::INSTANCE_BYTES as u64,
            )],
        });
        let shadow = if shadow_enabled {
            let direction = packed_light_direction(&light_data);
            Some(ShadowPrepared::new(
                device,
                &scene.view,
                &scene.instances,
                direction,
                &instance_layout,
            )?)
        } else {
            None
        };
        if let Some(shadow) = shadow.as_ref() {
            instance_upload.seed_shadow_clamps(&scene.instances, |instance| {
                instance.ids[3] != 0 && !shadow.contains_caster(instance)
            });
        }
        let views: Vec<wgpu::TextureView> = classes
            .arrays()
            .iter()
            .map(|array| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("typed default texture class"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: array.class.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &array.class.default_texel(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4),
                        rows_per_image: Some(1),
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let materials = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic material classes"),
            layout: &material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&views[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let instances = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic forward opaque"),
            layout: &instance_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_upload.buffer().as_entire_binding(),
            }],
        });
        let bind_group_layouts = [
            Some(&view_layout),
            Some(&light_layout),
            Some(&material_layout),
            Some(&instance_layout),
        ];
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic ABI2"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("prepared synthetic shader ABI2"),
            source: wgpu::ShaderSource::Wgsl(include_str!("synthetic.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("prepared opaque synthetic role"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(if shadow.is_some() {
                    "shadowed_fragment"
                } else {
                    "fragment"
                }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let (display_pipeline, display_layout) = prepare_display_pipeline(device);
        if let Some(error) = scope.pop().await {
            return Err(RenderError::Validation(error.to_string()));
        }
        Ok(Self {
            key: PipelineKey {
                device_generation: generation,
                revision: SYNTHETIC_REVISION,
                role: 0,
            },
            pipeline,
            display_pipeline,
            display_layout,
            view_layout,
            view_buffer,
            view_sample: scene.view,
            light,
            materials,
            instances,
            instance_upload,
            material_bytes,
            instance_bytes,
            instance_count,
            display_gain,
            exposure_bits: exposure.to_bits(),
            local_light_bytes,
            cluster_head_bytes,
            cluster_index_bytes,
            dropped_light_references,
            presentation_light_clamps,
            background,
            shadow,
        })
    }
    /// Local-light references dropped when a cluster exceeds its declared
    /// capacity. This is presentation telemetry, not a gameplay light count.
    pub fn dropped_light_references(&self) -> usize {
        self.dropped_light_references
    }
    /// Authored light/background channels clamped to the finite presentation
    /// envelope while preparing this generation.
    pub fn presentation_light_clamps(&self) -> usize {
        self.presentation_light_clamps
    }
    /// Number of prepared GPU scene passes for this light state. A zero Sun
    /// omits both cascades and their resources.
    pub fn scene_pass_count(&self) -> u32 {
        if self.shadow.is_some() { 4 } else { 2 }
    }
    /// Update sorted, distinct changed slots before the next queue submission.
    /// The caller submits each frame before preparing another one; queue writes
    /// are ordered after prior submitted frames. Malformed presentation rows
    /// retain their prior valid value and are counted. Omitted slots also retain
    /// their prior data.
    /// # Errors
    /// Refuses an invalid slot list or changed table size.
    pub fn upload_instances(
        &mut self,
        queue: &wgpu::Queue,
        instances: &[InstanceRecord],
        changed_slots: &[u32],
    ) -> Result<InstanceUploadReport, RenderError> {
        let shadow = self.shadow.as_ref();
        self.instance_upload
            .replace(queue, instances, changed_slots, |instance| {
                instance.ids[3] != 0
                    && shadow.is_some_and(|shadow| !shadow.contains_caster(instance))
            })
    }
    /// Prepared generation-local finite draw tuple.
    pub fn pipeline_key(&self) -> PipelineKey {
        self.key
    }
    fn display_key(&self) -> PipelineKey {
        PipelineKey {
            role: 1,
            ..self.key
        }
    }
    fn shadow_key(&self, index: u32) -> PipelineKey {
        PipelineKey {
            role: index + 2,
            ..self.key
        }
    }
    fn check_target(&self, target: &OffscreenTarget) -> Result<(), RenderError> {
        if target.generation != self.key.device_generation
            || target.display_gain.to_bits() != self.display_gain.to_bits()
            || target.exposure_bits != self.exposure_bits
            || target.view_sample != self.view_sample
            || target.shadow.as_ref().map(|shadow| &shadow.sample_data)
                != self.shadow.as_ref().map(|shadow| &shadow.sample_data)
        {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }
    /// Compile the actual draw, display + optional readback declaration. Allocation and
    /// graph analysis happen at preparation/target changes, not per instance.
    /// # Errors
    /// Refuses a stale target generation or invalid typed resource/pass declarations.
    pub fn graph(
        &self,
        target: &OffscreenTarget,
        readback: bool,
    ) -> Result<CompiledGraph, RenderError> {
        self.check_target(target)?;
        let mut graph = FrameGraph::default();
        let material = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.material_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let instance = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.instance_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let mut accesses = vec![
            Access {
                resource: material,
                range: 0..self.material_bytes,
                kind: AccessKind::StorageRead,
                store: true,
            },
            Access {
                resource: instance.clone(),
                range: 0..self.instance_bytes,
                kind: AccessKind::StorageRead,
                store: true,
            },
        ];
        let mut prepared_keys = BTreeSet::from([self.key, self.display_key()]);
        self.append_shadow_passes(&mut graph, &instance, &mut accesses, &mut prepared_keys)?;
        for bytes in [VIEW_BYTES as u64, AUX_BYTES as u64, LIGHT_BYTES as u64] {
            let uniform = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                resource: uniform,
                range: 0..bytes,
                kind: AccessKind::UniformRead,
                store: true,
            });
        }
        self.append_cluster_reads(&mut graph, &mut accesses)?;
        let depth = graph.resource(
            ResourceDescriptor::Texture {
                width: target.width,
                height: target.height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        accesses.push(Access {
            resource: depth,
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            store: false,
        });
        for class in TextureClasses::defaults(&wgpu::Limits::default())?.arrays() {
            let texture = graph.resource(
                ResourceDescriptor::Texture {
                    width: 1,
                    height: 1,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: class.class.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                resource: texture,
                range: 0..1,
                kind: AccessKind::SampledRead,
                store: true,
            });
        }
        let scene = graph.resource(
            ResourceDescriptor::Texture {
                width: target.width,
                height: target.height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: SCENE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        accesses.push(Access {
            resource: scene.clone(),
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            store: true,
        });
        graph.pass(Pass {
            name: "synthetic forward opaque".into(),
            scope: PassScope::View(1),
            pipeline: Some(self.key),
            accesses,
            external_side_effect: false,
        });
        let output = graph.resource(
            ResourceDescriptor::Texture {
                width: target.width,
                height: target.height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: OUTPUT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        let display = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: DISPLAY_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        graph.pass(Pass {
            name: "synthetic tone encode".into(),
            scope: PassScope::View(1),
            pipeline: Some(self.display_key()),
            accesses: vec![
                Access {
                    resource: scene,
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    resource: display,
                    range: 0..DISPLAY_BYTES,
                    kind: AccessKind::UniformRead,
                    store: true,
                },
                Access {
                    resource: output.clone(),
                    range: 0..1,
                    kind: AccessKind::AttachmentClear,
                    store: true,
                },
            ],
            external_side_effect: !readback,
        });
        if readback {
            let bytes = target.readback_bytes();
            let destination = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Imported { initialized: false },
            )?;
            graph.pass(Pass {
                name: "output readback".into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![
                    Access {
                        resource: output,
                        range: 0..1,
                        kind: AccessKind::CopyRead,
                        store: true,
                    },
                    Access {
                        resource: destination,
                        range: 0..bytes,
                        kind: AccessKind::CopyWrite,
                        store: true,
                    },
                ],
                external_side_effect: true,
            });
        }
        Ok(graph.compile(self.key.device_generation, &prepared_keys)?)
    }
    fn append_shadow_passes(
        &self,
        graph: &mut FrameGraph,
        instance: &crate::graph::ResourceId,
        accesses: &mut Vec<Access>,
        prepared_keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<(), RenderError> {
        if self.shadow.is_none() {
            return Ok(());
        }

        let maps = graph.resource(
            ResourceDescriptor::Texture {
                width: shadow::SIZE,
                height: shadow::SIZE,
                layers: 2,
                mips: 1,
                samples: 1,
                format: shadow::FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        for index in 0..2_u32 {
            let matrix = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: 64,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            let key = self.shadow_key(index);
            prepared_keys.insert(key);
            graph.pass(Pass {
                name: if index == 0 {
                    "synthetic Sun shadow near"
                } else {
                    "synthetic Sun shadow far"
                }
                .into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses: vec![
                    Access {
                        resource: instance.clone(),
                        range: 0..self.instance_bytes,
                        kind: AccessKind::StorageRead,
                        store: true,
                    },
                    Access {
                        resource: matrix,
                        range: 0..64,
                        kind: AccessKind::UniformRead,
                        store: true,
                    },
                    Access {
                        resource: maps.clone(),
                        range: u64::from(index)..u64::from(index + 1),
                        kind: AccessKind::AttachmentClear,
                        store: true,
                    },
                ],
                external_side_effect: false,
            });
        }
        let sample = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: shadow::SAMPLE_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access {
            resource: sample,
            range: 0..shadow::SAMPLE_BYTES as u64,
            kind: AccessKind::UniformRead,
            store: true,
        });
        accesses.push(Access {
            resource: maps,
            range: 0..2,
            kind: AccessKind::SampledRead,
            store: true,
        });
        Ok(())
    }
    fn append_cluster_reads(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
    ) -> Result<(), RenderError> {
        for bytes in [
            self.local_light_bytes,
            self.cluster_head_bytes,
            self.cluster_index_bytes,
        ] {
            let storage = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                resource: storage,
                range: 0..bytes,
                kind: AccessKind::StorageRead,
                store: true,
            });
        }
        Ok(())
    }
    /// Encode an already prepared draw. The caller owns submission and device
    /// polling. Skipping or failing this method cannot advance view histories.
    /// # Errors
    /// Refuses a stale target/device generation or unavailable timestamp batch capacity.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        target: &OffscreenTarget,
        mut timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        self.check_target(target)?;
        if generation != self.key.device_generation {
            return Err(RenderError::StaleTarget);
        }
        if let Some(timings) = timings.as_deref_mut() {
            timings.check_passes(generation, if self.shadow.is_some() { 4 } else { 2 })?;
        }
        if let (Some(shadow), Some(shadow_target)) = (&self.shadow, &target.shadow) {
            shadow.encode(
                encoder,
                shadow_target,
                &self.instances,
                self.instance_count,
                generation,
                timings.as_deref_mut(),
            )?;
        }
        let timestamps = match timings.as_deref_mut() {
            Some(timings) => timings.pass_writes(generation, "synthetic forward opaque")?,
            None => None,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic forward opaque"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.scene_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.background),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: timestamps,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &target.view_bind_group, &[]);
        pass.set_bind_group(1, &self.light, &[]);
        pass.set_bind_group(2, &self.materials, &[]);
        pass.set_bind_group(3, &self.instances, &[]);
        pass.draw(0..36, 0..self.instance_count);
        drop(pass);
        let timestamps = match timings {
            Some(timings) => timings.pass_writes(generation, "synthetic tone encode")?,
            None => None,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic tone encode"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            timestamp_writes: timestamps,
            ..Default::default()
        });
        pass.set_pipeline(&self.display_pipeline);
        pass.set_bind_group(0, &target.display, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
fn checked_display_gain(exposure: f32) -> Result<f32, RenderError> {
    let inverse = 1.0 / exposure;
    let gain = exposure / PRE_EXPOSURE;
    if !exposure.is_finite()
        || exposure <= 0.0
        || !inverse.is_finite()
        || inverse <= 0.0
        || !gain.is_finite()
        || gain <= 0.0
    {
        return Err(RenderError::InvalidExposure);
    }
    Ok(gain)
}

fn prepare_display_pipeline(
    device: &wgpu::Device,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("synthetic display layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(DISPLAY_BYTES),
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("synthetic display"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("synthetic authored SDR display"),
        source: wgpu::ShaderSource::Wgsl(include_str!("synthetic_display.wgsl").into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("prepared synthetic tone encode role"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: OUTPUT_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    (pipeline, layout)
}

fn prepare_display_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    scene: &wgpu::TextureView,
    gain: f32,
) -> wgpu::BindGroup {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&gain.to_le_bytes());
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("synthetic checked display e/p"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic display inputs"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(scene),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

fn uniform_layout(
    binding: u32,
    size: u64,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size),
        },
        count: None,
    }
}
fn storage_layout(
    binding: u32,
    visibility: wgpu::ShaderStages,
    stride: u64,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(stride),
        },
        count: None,
    }
}
fn storage_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    data: &[u8],
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: data.len() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
}

/// Generation-bound offscreen target. Retain while encoded/submitted work uses
/// it; actual old/new allocation retirement belongs to the executor's fences.
pub struct OffscreenTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    scene_view: wgpu::TextureView,
    display: wgpu::BindGroup,
    view_bind_group: wgpu::BindGroup,
    display_gain: f32,
    exposure_bits: u32,
    view_sample: SyntheticView,
    shadow: Option<ShadowTarget>,
    depth: wgpu::TextureView,
    width: u32,
    height: u32,
    generation: u64,
    row_bytes: u32,
}
impl OffscreenTarget {
    /// Allocate a nonempty bounded target and immutable display bind group from
    /// one prepared view's generation/exposure, on the executor's device. The
    /// prepared view retains both pipelines across resize and additional targets.
    /// # Errors
    /// Refuses empty/overflowing targets or extents beyond the device limits.
    pub fn new(
        device: &wgpu::Device,
        prepared: &PreparedSynthetic,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let limits = device.limits();
        if width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || (prepared.shadow.is_some() && shadow::SIZE > limits.max_texture_dimension_2d)
        {
            return Err(RenderError::TargetExtent);
        }
        let row_bytes = width
            .checked_mul(4)
            .and_then(|value| value.checked_add(255))
            .map(|value| value / 256 * 256)
            .ok_or(RenderError::TargetExtent)?;
        if u64::from(row_bytes) * u64::from(height) > limits.max_buffer_size {
            return Err(RenderError::TargetExtent);
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("synthetic offscreen target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("synthetic opaque depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let scene_view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("synthetic pre-exposed scene HDR"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SCENE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let display = prepare_display_group(
            device,
            &prepared.display_layout,
            &scene_view,
            prepared.display_gain,
        );
        let aux = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic target view aux"),
            contents: &prepared.view_sample.pack_aux(width, height),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let shadow = prepared
            .shadow
            .as_ref()
            .map(|shadow| ShadowTarget::new(device, shadow));
        let mut view_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: prepared.view_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: aux.as_entire_binding(),
            },
        ];
        if let (Some(prepared_shadow), Some(target_shadow)) = (&prepared.shadow, &shadow) {
            view_entries.extend([
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: prepared_shadow.sample_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&target_shadow.array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&prepared_shadow.sampler),
                },
            ]);
        }
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic view and target aux"),
            layout: &prepared.view_layout,
            entries: &view_entries,
        });
        Ok(Self {
            texture,
            view,
            scene_view,
            display,
            view_bind_group,
            display_gain: prepared.display_gain,
            exposure_bits: prepared.exposure_bits,
            view_sample: prepared.view_sample,
            shadow,
            depth,
            width,
            height,
            generation: prepared.key.device_generation,
            row_bytes,
        })
    }
    /// Target extent in pixels.
    pub fn extent(&self) -> [u32; 2] {
        [self.width, self.height]
    }
    pub(crate) fn output_view(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub(crate) fn device_generation(&self) -> u64 {
        self.generation
    }
    fn readback_bytes(&self) -> u64 {
        u64::from(self.row_bytes) * u64::from(self.height)
    }
    /// Allocate one bounded CPU readback slot for this exact generation/grid.
    pub fn readback(&self, device: &wgpu::Device) -> PixelReadback {
        PixelReadback {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic pixel readback"),
                size: self.readback_bytes(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            generation: self.generation,
            extent: self.extent(),
            row_bytes: self.row_bytes,
            pending: None,
        }
    }
}

/// One bounded asynchronous sRGB-encoded SDR RGBA8 readback slot, reusable
/// after collection. Each pixel retains four bytes including opaque alpha.
pub struct PixelReadback {
    buffer: wgpu::Buffer,
    generation: u64,
    extent: [u32; 2],
    row_bytes: u32,
    pending: Option<Receiver<Result<(), wgpu::BufferAsyncError>>>,
}
impl PixelReadback {
    /// Encode a copy and map on submission; no CPU wait or private submission.
    /// # Errors
    /// Refuses a pending readback or mismatched target generation/extent.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
    ) -> Result<(), RenderError> {
        if self.pending.is_some() {
            return Err(RenderError::Readback);
        }
        if self.generation != target.generation || self.extent != target.extent() {
            return Err(RenderError::StaleTarget);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.row_bytes),
                    rows_per_image: Some(self.extent[1]),
                },
            },
            wgpu::Extent3d {
                width: self.extent[0],
                height: self.extent[1],
                depth_or_array_layers: 1,
            },
        );
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&self.buffer, wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        self.pending = Some(receive);
        Ok(())
    }
    /// Nonblocking collection, removing row padding. The caller polls the device.
    /// # Errors
    /// Refuses stale generations, failed mapping or a disconnected completion callback.
    pub fn try_collect(&mut self, generation: u64) -> Result<Option<Vec<u8>>, RenderError> {
        if self.generation != generation {
            return Err(RenderError::StaleTarget);
        }
        let Some(pending) = &self.pending else {
            return Ok(None);
        };
        match pending.try_recv() {
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) | Ok(Err(_)) => {
                self.pending = None;
                return Err(RenderError::Readback);
            }
            Ok(Ok(())) => {}
        }
        self.pending = None;
        let mapped = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| RenderError::Readback)?;
        let mut pixels = Vec::with_capacity(self.extent[0] as usize * self.extent[1] as usize * 4);
        for row in mapped.chunks_exact(self.row_bytes as usize) {
            pixels.extend_from_slice(&row[..self.extent[0] as usize * 4]);
        }
        drop(mapped);
        self.buffer.unmap();
        Ok(Some(pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_refuses_invalid_input_and_derived_gain() {
        for exposure in [
            0.0,
            -1.0,
            f32::NAN,
            f32::INFINITY,
            f32::MAX,
            f32::from_bits(1),
        ] {
            assert!(matches!(
                checked_display_gain(exposure),
                Err(RenderError::InvalidExposure)
            ));
        }
        for exposure in [0.25, 1.0] {
            assert_eq!(
                checked_display_gain(exposure).unwrap().to_bits(),
                (exposure * 1024.0).to_bits()
            );
        }
    }

    #[test]
    fn light_preparation_normalizes_extremes_and_refuses_bad_sources() {
        for direction in [[f32::MAX, 0.0, 0.0], [f32::MIN_POSITIVE, 0.0, 0.0]] {
            let bytes = SyntheticLight {
                direction_to_light: direction,
                ..Default::default()
            }
            .pack()
            .unwrap();
            assert_eq!(f32::from_le_bytes(bytes[..4].try_into().unwrap()), 1.0);
            assert_eq!(&bytes[12..16], &[0; 4]);
            assert_eq!(&bytes[28..32], &[0; 4]);
        }
        for direction in [[0.0; 3], [f32::NAN, 0.0, 1.0], [f32::INFINITY, 0.0, 1.0]] {
            assert!(matches!(
                SyntheticLight {
                    direction_to_light: direction,
                    ..Default::default()
                }
                .pack(),
                Err(RenderError::InvalidLighting)
            ));
        }
        for value in [-1.0, f32::NAN, f32::INFINITY, 10_001.0] {
            assert!(matches!(
                SyntheticLight {
                    irradiance: [value; 3],
                    ..Default::default()
                }
                .pack(),
                Err(RenderError::InvalidLighting)
            ));
        }
        assert!(
            SyntheticLight {
                irradiance: [0.0; 3],
                ..Default::default()
            }
            .pack()
            .is_ok()
        );
    }

    #[test]
    fn photometric_pack_preserves_units_and_counts_presentation_clamps() {
        let source = PhotometricEnvironment {
            toward_sun: [0.0, 1.0, 0.0],
            sun_lux: 100_000.0,
            sun_colour: [1.0; 3],
            ambient_lux: [1000.0; 3],
            background_radiance: [2.0, 3.0, 4.0],
        };
        let (bytes, background, clamps) = source.pack().unwrap();
        assert_eq!(clamps, 0);
        assert_eq!(
            f32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            100_000.0
        );
        assert_eq!(
            f32::from_le_bytes(bytes[48..52].try_into().unwrap()),
            1000.0
        );
        assert_eq!(background.r, 2.0 / 1024.0);
        let (_, _, clamps) = PhotometricEnvironment {
            sun_lux: 20_000_000.0,
            ..source
        }
        .pack()
        .unwrap();
        assert_eq!(clamps, 3);
        assert!(matches!(
            PhotometricEnvironment {
                ambient_lux: [f64::NAN, 0.0, 0.0],
                ..source
            }
            .pack(),
            Err(RenderError::InvalidLighting)
        ));
        assert!(
            PhotometricEnvironment {
                sun_colour: [
                    0.088_462_886_047_803_61,
                    0.060_368_322_438_510_39,
                    12.991_930_293_881_085,
                ],
                ..source
            }
            .pack()
            .is_ok()
        );
    }
}
