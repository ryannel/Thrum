//! Prepared in-memory instanced diagnostic. It exercises the real material and
//! instance tables, four texture-class bindings, graph declaration and optional
//! timestamps. One directional metallic-roughness closure and opaque depth are
//! real, followed by a fixed authored SDR display transform. Fixed-view
//! selective GPU local-light lists and two synthetic Sun cascades run here;
//! a checked constant-light presentation handoff and the general render scene
//! remain open.

use crate::abi::{self, AbiError, InstanceRecord, MaterialRecord, TextureClasses};
use crate::atmosphere::{AtmosphereFrame, AtmospherePrepared};
use crate::capabilities::DeviceCapabilities;
use crate::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SurfaceParts,
};
use crate::graph::{
    CompiledGraph, CompiledPass, GraphColorAttachment, GraphDepthAttachment, GraphError,
    GraphTextures, PipelineKey, ResourceId,
};
use crate::graph_record::{ExecutableGraph, RecordFamily};
use crate::timing::{GpuTimings, TimingError};
use crate::visibility::{VisibilityPrepared, VisibilityTarget};
pub use camera_culling::CullMode;
use glam::Vec3;
use std::sync::Arc;
use wgpu::util::DeviceExt;

mod air_shadow;
mod camera_culling;
mod cluster;
mod cluster_gpu;
mod contributed;
mod display;
mod exposure;
mod frame_stats;
mod history_gpu;
mod readback;
mod scene_apply;
mod scene_graph;
mod shadow;
mod stars;
mod upload;
mod view;
pub use readback::PixelReadback;

pub(crate) use air_shadow::AIR_SHADOW_PASS;

use air_shadow::{AirShadowPrepared, AirShadowTarget};
use cluster::ClusterTable;
pub use cluster::SyntheticPointLight;
use cluster_gpu::ClusterCompute;
pub(crate) use contributed::CONTRIBUTION_COMPUTE;
use contributed::CONTRIBUTION_UPLOAD;
pub use display::{DisplayOverlay, SdrToneMap};
use display::{DisplayPrepared, DisplayTarget};
use exposure::ExposureResolve;
pub(crate) use frame_stats::{SceneTimingLayout, pass_durations};
use history_gpu::TransformHistory;
pub use scene_apply::SceneApplyReport;
use shadow::{ShadowPrepared, ShadowTarget};
use stars::{StarPrepared, StarTarget};
use upload::{InstanceUpload, PackedInstances};
pub use view::SyntheticView;
use view::{AUX_BYTES, Projection};

/// Synthetic source revision; not a substitute for an F4 product identity.
pub const SYNTHETIC_REVISION: u64 = 16;
/// Portable reversed-Z opaque depth, sampled by the optional star composite.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const VIEW_BYTES: usize = 160;
const VIEW_SLOTS: u32 = 2;
pub(crate) const LIGHT_BYTES: usize = 208;
pub(crate) const PRE_EXPOSURE: f32 = 1.0 / 1024.0;
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const MOTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
const CUBE_INDICES: [u16; 36] = [
    0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7, 8, 9, 10, 10, 9, 11, 12, 13, 14, 14, 13, 15, 16, 17, 18,
    18, 17, 19, 20, 21, 22, 22, 21, 23,
];
const CUBE_INDEX_BYTES: u64 = 72;
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
    /// Camera opaque draw: the direct oracle or GPU frustum compaction.
    pub camera_culling: CullMode,
    /// Optional immutable unit equatorial directions with incident lux in w.
    pub star_catalogue: Option<&'static [[f32; 4]]>,
}
impl SyntheticScene {
    /// Generate a bounded regular grid. Instance IDs remain stable; one draw
    /// renders the active prefix. The default view uses the historical orthographic
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
            camera_culling: CullMode::Direct,
            star_catalogue: None,
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

/// One frame's directional and background lighting in a declared unit system.
#[derive(Clone, Copy, Debug)]
pub enum FrameLighting {
    /// Finite diagnostic illumination for the synthetic game scene.
    Diagnostic(SyntheticLight),
    /// Photometric environment converted at the presentation boundary.
    Photometric(PhotometricEnvironment),
    /// Checked physical Earth atmosphere and its single directional Sun.
    Atmosphere(AtmosphereFrame),
}

fn pack_frame_lighting(
    lighting: FrameLighting,
) -> Result<([u8; LIGHT_BYTES], wgpu::Color, usize), RenderError> {
    match lighting {
        FrameLighting::Diagnostic(light) => Ok((
            light.pack()?,
            wgpu::Color {
                r: 0.02 * f64::from(PRE_EXPOSURE),
                g: 0.03 * f64::from(PRE_EXPOSURE),
                b: 0.04 * f64::from(PRE_EXPOSURE),
                a: 1.0,
            },
            0,
        )),
        FrameLighting::Photometric(environment) => environment.pack(),
        FrameLighting::Atmosphere(frame) => {
            const SOLAR_DISK_SOLID_ANGLE: f32 = 6.805_165e-5;
            let (direction, illuminance) = frame.directional();
            // The disk shader converts incident lux to radiance over the
            // apparent solar solid angle. Count channels whose direct disk
            // term alone exceeds the current fp16 scene envelope. G3t's
            // coordinated pre-exposure/EDR path must remove this repair.
            let stored_lux_limit = 65_504.0 * SOLAR_DISK_SOLID_ANGLE / PRE_EXPOSURE;
            let disk_clamps = illuminance
                .iter()
                .filter(|&&lux| lux > stored_lux_limit)
                .count();
            Ok((
                pack_directional(direction.map(f64::from), illuminance, [0.0; 3])?,
                wgpu::Color::BLACK,
                disk_clamps,
            ))
        }
    }
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
    /// A record's logical GPU slot does not match its scene row.
    InvalidInstanceIdentity,
    /// Unsupported semantic flags, skinning or mapped normals in this diagnostic.
    UnsupportedSyntheticMaterial,
    /// Invalid directional source or degenerate/unsupported diagnostic transform.
    InvalidLighting,
    /// Nonfinite or degenerate camera projection.
    InvalidView,
    /// Exposure gain is unrepresentable or incident EV settings are unsupported.
    InvalidExposure,
    /// Target/readback belongs to another generation, grid or prepared exposure.
    StaleTarget,
    /// Target dimensions exceed device limits or overflow readback arithmetic.
    TargetExtent,
    /// A compiled pass has no recorder for this prepared scene.
    GraphPassMismatch,
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
    capabilities: DeviceCapabilities,
    classes: TextureClasses,
    owner: Arc<()>,
    key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    pipeline_two_sided: wgpu::RenderPipeline,
    cube_indices: wgpu::Buffer,
    cube_reversed_indices: wgpu::Buffer,
    display: DisplayPrepared,
    view_layout: wgpu::BindGroupLayout,
    view_buffer: wgpu::Buffer,
    view_stride: u32,
    view_sample: SyntheticView,
    pending_view: Option<SyntheticView>,
    frame_revision: u64,
    directional_buffer: wgpu::Buffer,
    local_light_buffer: wgpu::Buffer,
    cluster_head_buffer: wgpu::Buffer,
    cluster_index_buffer: wgpu::Buffer,
    clusters: ClusterTable,
    cluster_compute: Option<ClusterCompute>,
    light: [wgpu::BindGroup; 2],
    materials: wgpu::BindGroup,
    history: TransformHistory,
    exposure: ExposureResolve,
    history_source: Option<(u128, u64)>,
    instance_upload: InstanceUpload,
    material_bytes: u64,
    instance_bytes: u64,
    instance_count: u32,
    exposure_bits: u32,
    local_light_bytes: u64,
    cluster_head_bytes: u64,
    cluster_index_bytes: u64,
    dropped_light_references: usize,
    presentation_light_clamps: usize,
    background: wgpu::Color,
    shadow: Option<ShadowPrepared>,
    /// Sun shadow heights for the air, with shadows under atmosphere lighting.
    air_shadow: Option<AirShadowPrepared>,
    atmosphere: Option<AtmospherePrepared>,
    surface: SurfaceParts,
    contribution_passes: ContributionPasses,
    /// Camera culling pipelines; `None` is the Direct oracle with no extra resources.
    visibility: Option<VisibilityPrepared>,
    stars: Option<StarPrepared>,
}

struct SceneFamily;

impl RecordFamily for SceneFamily {
    type Frame<'a> = SceneContext<'a>;
    type Error = RenderError;
}

struct SceneContext<'a> {
    prepared: &'a PreparedSynthetic,
    target: &'a OffscreenTarget,
    encoder: &'a mut wgpu::CommandEncoder,
    timings: Option<&'a mut GpuTimings>,
    generation: u64,
    readback: Option<&'a mut PixelReadback>,
    contribution: Option<&'a mut (dyn SceneContribution + 'static)>,
    contribution_frame: ContributionFrame,
    cooked_shadow_fit: Option<bool>,
    exposure_side: usize,
}

impl SceneContext<'_> {
    fn atmosphere(&mut self, index: usize, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared
            .atmosphere
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .encode(index, self.encoder, pass, self.timings.as_deref_mut())
    }

    fn shadow(&mut self, index: usize, pass: &CompiledPass) -> Result<(), RenderError> {
        let shadow = self
            .prepared
            .shadow
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let shadow_target = self
            .target
            .shadow
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let draw_contribution = *self.cooked_shadow_fit.get_or_insert_with(|| {
            let bounds = self
                .contribution
                .as_deref()
                .and_then(SceneContribution::shadow_bounds);
            shadow.write_recording(bounds)
        });
        shadow.encode_cascade(
            self.encoder,
            shadow_target,
            self.prepared.history.read_group(),
            &self.prepared.cube_indices,
            self.prepared.instance_upload.active_count(),
            self.prepared.instance_upload.two_sided_count() != 0,
            self.generation,
            index,
            pass,
            self.target
                .ids
                .shadow
                .as_ref()
                .ok_or(RenderError::GraphPassMismatch)?,
            self.timings.as_deref_mut(),
            if draw_contribution {
                self.contribution.as_deref()
            } else {
                None
            },
        )
    }

    fn opaque(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared.encode_opaque(
            self.encoder,
            self.target,
            pass,
            self.timings.as_deref_mut(),
            self.contribution.as_deref(),
            self.exposure_side,
        )
    }

    fn contribution_upload(&mut self) {
        if let Some(contribution) = self.contribution.as_deref_mut() {
            contribution.encode_upload(self.encoder, &self.contribution_frame);
        }
    }

    fn contribution_compute(&mut self, compiled: &CompiledPass) -> Result<(), RenderError> {
        if let Some(contribution) = self.contribution.as_deref_mut()
            && contribution.computes(&self.contribution_frame)
        {
            let mut pass = compiled
                .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
                .ok_or(RenderError::GraphPassMismatch)?;
            contribution.encode_compute(&mut pass, &self.contribution_frame);
        }
        if let Some(contribution) = self.contribution.as_deref_mut() {
            contribution.encode_after_compute(self.encoder);
        }
        Ok(())
    }

    fn publish_history(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared.history.encode(
            self.encoder,
            self.generation,
            pass,
            self.prepared.instance_upload.active_count(),
            self.timings.as_deref_mut(),
        )
    }

    fn air_shadow(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let (air, target) = (self.prepared.air_shadow.as_ref())
            .zip(self.target.air_shadow.as_ref())
            .ok_or(RenderError::GraphPassMismatch)?;
        let resource =
            (self.target.ids.air_shadow.as_ref()).ok_or(RenderError::GraphPassMismatch)?;
        let timings = self.timings.as_deref_mut();
        air.encode(
            self.encoder,
            target,
            self.generation,
            pass,
            resource,
            timings,
        )
    }

    fn cluster(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared
            .cluster_compute
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .encode(
                self.encoder,
                self.generation,
                pass,
                self.timings.as_deref_mut(),
            )
    }

    fn exposure(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared.exposure.encode(
            self.encoder,
            self.generation,
            pass,
            self.exposure_side,
            self.timings.as_deref_mut(),
        )
    }

    fn tone(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared.display.encode(
            self.encoder,
            self.generation,
            self.target,
            pass,
            self.exposure_side,
            self.timings.as_deref_mut(),
        )
    }

    fn copy(&mut self, _pass: &CompiledPass) -> Result<(), RenderError> {
        self.readback
            .as_deref_mut()
            .ok_or(RenderError::GraphPassMismatch)?
            .copy_into(self.encoder, self.target)
    }
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
    /// Prepare with a checked fixed camera fallback. Live views may select a
    /// fixed override or physical incident metering without changing storage p.
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
        let capabilities = DeviceCapabilities::capture_device(device, generation);
        Self::prepare_with_lighting(
            &capabilities,
            queue,
            scene,
            FrameLighting::Diagnostic(light),
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
        let capabilities = DeviceCapabilities::capture_device(device, generation);
        Self::prepare_with_capabilities(
            &capabilities,
            queue,
            scene,
            FrameLighting::Photometric(environment),
            exposure,
        )
        .await
    }

    /// Prepare the live scene using one adapter-generation record captured by
    /// the platform before pipeline closure.
    ///
    /// # Errors
    ///
    /// Refuses invalid authored quantities, view or GPU preparation.
    pub async fn prepare_with_capabilities(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        exposure: f32,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_lighting(capabilities, queue, scene, lighting, exposure).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one finite pipeline preparation transaction keeps resources, layouts and its validation scope together"
    )]
    async fn prepare_with_lighting(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        exposure: f32,
    ) -> Result<Self, RenderError> {
        let device = capabilities.device();
        let generation = capabilities.generation();
        checked_display_gain(exposure)?;
        let (mut light_data, background, presentation_light_clamps) =
            pack_frame_lighting(lighting)?;
        // Prepare the declared shadow recipe even while the Sun is dark, so a
        // later live light change can use the same pipelines and target maps.
        let shadow_enabled = scene.sun_shadows;
        let gpu_clusters = (1..=cluster::GPU_MAX_SOURCES).contains(&scene.local_lights.len());
        let clusters = if gpu_clusters {
            ClusterTable::build_gpu(&scene.local_lights, &scene.view)?
        } else {
            ClusterTable::build(&scene.local_lights, &scene.view)?
        };
        let has_local_lights: f32 = if clusters.active { 1.0 } else { 0.0 };
        light_data[12..16].copy_from_slice(&has_local_lights.to_le_bytes());
        let [local_light_bytes, cluster_head_bytes, cluster_index_bytes] = clusters.capacities();
        let dropped_light_references = clusters.dropped_references;
        let limits = capabilities.limits();
        if matches!(lighting, FrameLighting::Atmosphere(_))
            && (limits.max_sampled_textures_per_shader_stage < 9 + 2 * u32::from(shadow_enabled)
                || limits.max_samplers_per_shader_stage < 3 + u32::from(shadow_enabled)
                || limits.max_bindings_per_bind_group < 12)
        {
            return Err(AbiError::DeviceLimits.into());
        }
        if [local_light_bytes, cluster_head_bytes, cluster_index_bytes]
            .into_iter()
            .any(|bytes| bytes > limits.max_storage_buffer_binding_size)
        {
            return Err(RenderError::SceneCapacity);
        }
        let classes = TextureClasses::defaults(limits)?;
        let (material_bytes, instance_bytes) =
            abi::table_bytes(scene.materials.len(), scene.instances.len(), limits)?;
        let material_count =
            u32::try_from(scene.materials.len()).map_err(|_| RenderError::SceneCapacity)?;
        let instance_count =
            u32::try_from(scene.instances.len()).map_err(|_| RenderError::SceneCapacity)?;
        if u64::from(instance_count) * history_gpu::ENTRY_BYTES
            > limits.max_storage_buffer_binding_size
        {
            return Err(RenderError::SceneCapacity);
        }
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
        let packed_instances = PackedInstances::new(&scene.instances, material_count)?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let material_buffer = storage_buffer(
            device,
            queue,
            "synthetic materials",
            &packed_materials,
            material_bytes,
        );
        let instance_buffer = storage_buffer(
            device,
            queue,
            "synthetic forward opaque",
            &packed_instances.bytes,
            instance_bytes,
        );
        let mut instance_upload = InstanceUpload::new(
            instance_buffer,
            packed_instances,
            &scene.instances,
            material_count,
            instance_count,
        );
        let view_alignment = limits.min_uniform_buffer_offset_alignment.max(1);
        let view_stride = u32::try_from(VIEW_BYTES)
            .expect("view record fits u32")
            .checked_add(view_alignment - 1)
            .map(|bytes| bytes / view_alignment * view_alignment)
            .filter(|stride| *stride <= u32::MAX / VIEW_SLOTS)
            .ok_or(RenderError::SceneCapacity)?;
        let view_capacity = u64::from(view_stride) * u64::from(VIEW_SLOTS);
        if view_capacity > limits.max_buffer_size {
            return Err(RenderError::SceneCapacity);
        }
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic ABI2 view"),
            size: view_capacity,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &view_buffer,
            0,
            &scene.view.pack_view(&scene.view, exposure),
        );
        let mut view_entries = vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(VIEW_BYTES as u64),
                },
                count: None,
            },
            uniform_layout(1, AUX_BYTES as u64, wgpu::ShaderStages::FRAGMENT),
        ];
        view_entries.extend(camera_culling::view_layout_entry(scene.camera_culling));
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
            ]);
            if matches!(lighting, FrameLighting::Atmosphere(_)) {
                view_entries.push(air_shadow::VIEW_ENTRY);
            }
        }
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic view and target aux"),
            entries: &view_entries,
        });
        let mut light_entries = vec![
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
        ];
        if matches!(lighting, FrameLighting::Atmosphere(_)) {
            for (binding, dimension) in [
                (4, wgpu::TextureViewDimension::D2Array),
                (5, wgpu::TextureViewDimension::D2Array),
                (8, wgpu::TextureViewDimension::Cube),
                (9, wgpu::TextureViewDimension::D2),
                (11, wgpu::TextureViewDimension::D2),
            ] {
                light_entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: dimension,
                        multisampled: false,
                    },
                    count: None,
                });
            }
            for binding in [6, 10] {
                light_entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                });
            }
            light_entries.push(uniform_layout(7, 128, wgpu::ShaderStages::FRAGMENT));
        }
        light_entries.push(uniform_layout(12, 16, wgpu::ShaderStages::FRAGMENT));
        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic directional and clustered sources"),
            entries: &light_entries,
        });
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic directional source"),
            size: LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        queue.write_buffer(&directional, 0, &light_data);
        let exposure_samples = ExposureResolve::allocate_samples(device);
        let atmosphere = match lighting {
            FrameLighting::Atmosphere(frame) => Some(AtmospherePrepared::new(
                device,
                queue,
                generation,
                frame,
                &scene.view,
                &directional,
                &exposure_samples,
            )),
            _ => None,
        };
        let local_lights = storage_buffer(
            device,
            queue,
            "synthetic local lights",
            &clusters.lights,
            local_light_bytes,
        );
        let cluster_heads = storage_buffer(
            device,
            queue,
            "synthetic cluster heads",
            &clusters.heads,
            cluster_head_bytes,
        );
        let cluster_indices = storage_buffer(
            device,
            queue,
            "synthetic cluster indices",
            &clusters.indices,
            cluster_index_bytes,
        );
        let cluster_compute = gpu_clusters.then(|| {
            let bounds = storage_buffer(
                device,
                queue,
                "synthetic cluster source bounds",
                &clusters.bounds,
                u64::try_from(clusters.bounds.len()).expect("32 sources fit u64"),
            );
            ClusterCompute::new(device, bounds, &cluster_heads, &cluster_indices)
        });
        let mut light_bindings = vec![
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
        ];
        if let Some(atmosphere) = &atmosphere {
            let (views, sampler, uniform) = atmosphere.aerial_sampled();
            let (reflection, dfg, reflection_sampler) = atmosphere.reflection_sampled();
            light_bindings.extend([
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(reflection),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(dfg),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(reflection_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: wgpu::BindingResource::TextureView(atmosphere.transmittance_view()),
                },
            ]);
        }
        let light = std::array::from_fn(|side| {
            light_bindings.push(wgpu::BindGroupEntry {
                binding: 12,
                resource: exposure_samples[side].as_entire_binding(),
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("synthetic clustered light lists"),
                layout: &light_layout,
                entries: &light_bindings,
            });
            light_bindings.pop();
            group
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
            entries: &[
                storage_layout(0, wgpu::ShaderStages::VERTEX, abi::INSTANCE_BYTES as u64),
                storage_layout(1, wgpu::ShaderStages::VERTEX, history_gpu::ENTRY_BYTES),
                uniform_layout(2, history_gpu::SERIAL_BYTES, wgpu::ShaderStages::VERTEX),
            ],
        });
        let history = TransformHistory::new(
            device,
            queue,
            instance_upload.buffer(),
            &instance_layout,
            instance_count,
        );
        let visibility = camera_culling::prepare(
            device,
            queue,
            generation,
            scene.camera_culling,
            instance_count,
            instance_upload.active_count(),
        );
        let shadow = if shadow_enabled {
            let direction = packed_light_direction(&light_data);
            Some(ShadowPrepared::new(
                device,
                queue,
                &scene.view,
                &scene.instances,
                direction,
                &instance_layout,
            )?)
        } else {
            None
        };
        let air_shadow = (shadow.is_some() && atmosphere.is_some()).then(|| {
            AirShadowPrepared::new(device, shadow::SAMPLE_BYTES as u64, LIGHT_BYTES as u64)
        });
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
        let shader_source = format!(
            "{}\n{}\n{}",
            concat!(
                include_str!("atmosphere/storage.wgsl"),
                "\n",
                include_str!("atmosphere/brdf.wgsl"),
                "\n",
                include_str!("atmosphere/irradiance.wgsl"),
                "\n",
                include_str!("synthetic.wgsl")
            ),
            air_shadow::aerial_source(atmosphere.is_some(), air_shadow.is_some()),
            if atmosphere.is_some() {
                concat!(
                    include_str!("atmosphere/geometry.wgsl"),
                    "\n",
                    include_str!("synthetic_sun.wgsl"),
                    "\n",
                    include_str!("synthetic_environment.wgsl")
                )
            } else {
                ""
            }
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("prepared synthetic shader ABI2"),
            source: wgpu::ShaderSource::Wgsl(shader_source.as_str().into()),
        });
        let fragment_entry = if shadow.is_some() {
            "shadowed_fragment"
        } else {
            "fragment"
        };
        let vertex_entry = camera_culling::vertex_entry(scene.camera_culling);
        let create_pipeline = |cull_mode| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("prepared opaque synthetic role"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode,
                    ..Default::default()
                },
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
                    entry_point: Some(fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: SCENE_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: MOTION_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = create_pipeline(Some(wgpu::Face::Back));
        let pipeline_two_sided = create_pipeline(None);
        let index_bytes: [u8; 72] =
            std::array::from_fn(|byte| CUBE_INDICES[byte / 2].to_le_bytes()[byte % 2]);
        let cube_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic shared cube indices"),
            contents: &index_bytes,
            usage: wgpu::BufferUsages::INDEX,
        });
        // The old orthographic diagnostic keeps world XY as clip XY while its
        // camera looks from -Z. Reverse only its index winding; perspective
        // cameras use the normal outward-facing cube triangles.
        let reversed_bytes: [u8; 72] = std::array::from_fn(|byte| {
            let index = byte / 2;
            let triangle = index / 3;
            let reversed = [0, 2, 1][index % 3];
            CUBE_INDICES[triangle * 3 + reversed].to_le_bytes()[byte % 2]
        });
        let cube_reversed_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic orthographic cube winding"),
            contents: &reversed_bytes,
            usage: wgpu::BufferUsages::INDEX,
        });
        let exposure_resolve = ExposureResolve::new(
            device,
            &directional,
            history.serial_buffer(),
            exposure_samples,
        );
        exposure_resolve.update(
            queue,
            scene.view.exposure_policy(exposure, atmosphere.is_some()),
        );
        let display = DisplayPrepared::new(device);
        let stars = if atmosphere.is_some() {
            scene
                .star_catalogue
                .map(|records| {
                    StarPrepared::new(
                        device,
                        records,
                        limits.max_storage_buffer_binding_size,
                        &light_data,
                    )
                })
                .transpose()?
        } else {
            None
        };
        if let Some(error) = scope.pop().await {
            return Err(RenderError::Validation(error.to_string()));
        }
        Ok(Self {
            capabilities: capabilities.clone(),
            classes,
            owner: Arc::new(()),
            key: PipelineKey {
                device_generation: generation,
                revision: SYNTHETIC_REVISION,
                role: 0,
            },
            pipeline,
            pipeline_two_sided,
            cube_indices,
            cube_reversed_indices,
            display,
            view_layout,
            view_buffer,
            view_stride,
            view_sample: scene.view,
            pending_view: None,
            frame_revision: 0,
            directional_buffer: directional,
            local_light_buffer: local_lights,
            cluster_head_buffer: cluster_heads,
            cluster_index_buffer: cluster_indices,
            clusters,
            cluster_compute,
            light,
            materials,
            history,
            visibility,
            stars,
            exposure: exposure_resolve,
            history_source: None,
            instance_upload,
            material_bytes,
            instance_bytes,
            instance_count,
            exposure_bits: exposure.to_bits(),
            local_light_bytes,
            cluster_head_bytes,
            cluster_index_bytes,
            dropped_light_references,
            presentation_light_clamps,
            background,
            shadow,
            air_shadow,
            atmosphere,
            surface: SurfaceParts {
                shader_source,
                pipeline_layout: layout,
                light_layout,
                material_layout,
                fragment_entry,
            },
            contribution_passes: ContributionPasses::default(),
        })
    }
    /// Immutable facts shared by every preparation on this adapter generation.
    pub fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
    pub(crate) fn fallback_exposure(&self) -> f32 {
        f32::from_bits(self.exposure_bits)
    }
    /// Local-light references dropped when a cluster exceeds its declared
    /// capacity. This is presentation telemetry, not a gameplay light count.
    pub fn dropped_light_references(&self) -> usize {
        self.dropped_light_references
    }
    /// Live sources beyond the prepared local-light capacity, omitted from
    /// presentation until a larger scene is prepared.
    pub fn dropped_light_sources(&self) -> usize {
        self.clusters.dropped_sources
    }
    /// Authored light/background channels clamped to the finite presentation
    /// envelope while preparing this generation.
    pub fn presentation_light_clamps(&self) -> usize {
        self.presentation_light_clamps
    }
    /// Number of prepared GPU scene passes for this scene's shadow recipe.
    pub fn scene_pass_count(&self) -> u32 {
        4 + u32::from(self.shadow.is_some()) * shadow::LAYERS
            + u32::from(self.visibility.is_some())
            + u32::from(self.stars.is_some())
            + u32::from(self.air_shadow.is_some())
            + u32::from(self.cluster_compute.is_some())
            + u32::from(self.contribution_passes.compute)
            + self
                .atmosphere
                .as_ref()
                .map_or(0, AtmospherePrepared::pass_count)
    }
    /// Camera opaque draw mode; Frustum adds one timed compute pass.
    pub fn camera_culling(&self) -> CullMode {
        if self.visibility.is_some() {
            CullMode::Frustum
        } else {
            CullMode::Direct
        }
    }
    /// Whether this prepared scene builds selective local-light lists on GPU.
    pub fn gpu_clustered(&self) -> bool {
        self.cluster_compute.is_some()
    }
    /// Number of active instance rows submitted by each opaque and shadow draw.
    pub fn active_instance_count(&self) -> u32 {
        self.instance_upload.active_count()
    }
    /// Active rows that require the prepared two-sided fallback for mixed winding.
    pub fn two_sided_instance_count(&self) -> usize {
        self.instance_upload.two_sided_count()
    }
    /// Whether the prepared frame records two Sun cascade passes and a height pass.
    pub fn shadowed(&self) -> bool {
        self.shadow.is_some()
    }
    /// Update one live view and its directional, local-light and cascade data.
    /// Pipelines, bind groups and target textures stay prepared. Call after the
    /// previous frame was submitted, then commit the view only after this frame
    /// is submitted. A dropped frame leaves temporal history unchanged.
    /// # Errors
    /// Refuses a foreign target, changed scene capacity or invalid frame data
    /// before modifying the GPU buffers.
    pub fn update_frame(
        &mut self,
        queue: &wgpu::Queue,
        target: &mut OffscreenTarget,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        presentation_alpha: f32,
    ) -> Result<(), RenderError> {
        self.check_owner(target)?;
        if self.atmosphere.is_some() != matches!(lighting, FrameLighting::Atmosphere(_)) {
            return Err(RenderError::InvalidLighting);
        }
        if scene.instances.len()
            != usize::try_from(self.instance_count).map_err(|_| RenderError::SceneCapacity)?
        {
            return Err(RenderError::SceneCapacity);
        }
        let (mut light_data, background, clamps) = pack_frame_lighting(lighting)?;
        if self.cluster_compute.is_some() {
            self.clusters.repack_gpu(&scene.local_lights, &scene.view)?;
        } else {
            self.clusters.rebuild(&scene.local_lights, &scene.view)?;
        }
        if [
            (self.clusters.lights.len(), self.local_light_bytes),
            (self.clusters.heads.len(), self.cluster_head_bytes),
            (self.clusters.indices.len(), self.cluster_index_bytes),
        ]
        .into_iter()
        .any(|(len, capacity)| u64::try_from(len).map_or(true, |len| len > capacity))
        {
            return Err(RenderError::SceneCapacity);
        }
        let has_local_lights: f32 = if self.clusters.active { 1.0 } else { 0.0 };
        light_data[12..16].copy_from_slice(&has_local_lights.to_le_bytes());
        if let Some(shadow) = self.shadow.as_mut() {
            shadow.update(
                &scene.view,
                &scene.instances,
                packed_light_direction(&light_data),
            )?;
        }
        self.history.set_extent(queue, target.extent());
        if scene.view.history_reset() {
            self.history.reset(queue);
        }
        self.history
            .set_presentation_alpha(queue, presentation_alpha);
        let next_revision = self.frame_revision.wrapping_add(1);
        let view_offset = self.view_stride * u32::from(next_revision & 1 != 0);
        queue.write_buffer(
            &self.view_buffer,
            u64::from(view_offset),
            &scene
                .view
                .pack_view(&self.view_sample, f32::from_bits(self.exposure_bits)),
        );
        queue.write_buffer(
            &target.aux_buffer,
            0,
            &scene.view.pack_aux(target.width, target.height),
        );
        target.draw_stars =
            self.stars
                .as_mut()
                .zip(target.stars.as_ref())
                .is_some_and(|(stars, binding)| {
                    stars.update_target(queue, binding, &scene.view, target.extent(), &light_data)
                });
        // Atmosphere owns the ambient tail on GPU; preserve its cached value.
        let cpu_light_bytes = if self.atmosphere.is_some() {
            48
        } else {
            LIGHT_BYTES
        };
        queue.write_buffer(&self.directional_buffer, 0, &light_data[..cpu_light_bytes]);
        queue.write_buffer(&self.local_light_buffer, 0, &self.clusters.lights);
        if let Some(compute) = self.cluster_compute.as_ref() {
            queue.write_buffer(&compute.bounds, 0, &self.clusters.bounds);
        } else {
            queue.write_buffer(&self.cluster_head_buffer, 0, &self.clusters.heads);
            queue.write_buffer(&self.cluster_index_buffer, 0, &self.clusters.indices);
        }
        self.dropped_light_references = self.clusters.dropped_references;
        self.presentation_light_clamps = clamps;
        self.background = background;
        if let (Some(atmosphere), FrameLighting::Atmosphere(frame)) =
            (&mut self.atmosphere, lighting)
        {
            atmosphere.update(queue, frame, &scene.view, target.extent());
        }
        self.exposure.update(
            queue,
            scene.view.exposure_policy(
                f32::from_bits(self.exposure_bits),
                self.atmosphere.is_some(),
            ),
        );
        self.pending_view = Some(scene.view);
        self.frame_revision = next_revision;
        target.frame_revision = self.frame_revision;
        Ok(())
    }

    /// Advance previous-view history after the host submits the encoded frame.
    pub fn commit_frame(&mut self) {
        if let Some(atmosphere) = self.atmosphere.as_mut() {
            atmosphere.commit();
        }
        if let Some(view) = self.pending_view.take() {
            self.view_sample = view;
            self.history.submitted();
        }
    }
    /// Reset temporal correspondence when the authoritative world or replay
    /// timeline changes. A later submitted view establishes fresh history.
    pub fn synchronize_history(&mut self, queue: &wgpu::Queue, epoch: u128, timeline: u64) {
        let source = (epoch, timeline);
        if self.history_source != Some(source) {
            self.history.reset(queue);
            self.history_source = Some(source);
        }
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
        let report = self
            .instance_upload
            .replace(queue, instances, changed_slots, |instance| {
                instance.ids[3] != 0
                    && shadow.is_some_and(|shadow| !shadow.contains_caster(instance))
            })?;
        if let Some(visibility) = &mut self.visibility {
            visibility.set_active(queue, self.instance_upload.active_count());
        }
        Ok(report)
    }
    /// Prepared generation-local finite draw tuple.
    pub fn pipeline_key(&self) -> PipelineKey {
        self.key
    }
    fn exposure_key(&self) -> PipelineKey {
        PipelineKey {
            role: 6,
            ..self.key
        }
    }

    fn display_key(&self) -> PipelineKey {
        PipelineKey {
            role: 1,
            ..self.key
        }
    }
    /// Roles 2 and 3 are the Sun cascades; the height map takes role 7.
    fn shadow_key(&self, index: u32) -> PipelineKey {
        PipelineKey {
            role: if index < 2 { index + 2 } else { 7 },
            ..self.key
        }
    }
    fn air_shadow_key(&self) -> PipelineKey {
        PipelineKey {
            role: 8,
            ..self.key
        }
    }
    fn cluster_key(&self) -> PipelineKey {
        PipelineKey {
            role: 4,
            ..self.key
        }
    }
    fn history_key(&self) -> PipelineKey {
        PipelineKey {
            role: 5,
            ..self.key
        }
    }
    fn check_owner(&self, target: &OffscreenTarget) -> Result<(), RenderError> {
        if !Arc::ptr_eq(&self.owner, &target.prepared_owner)
            || target.generation != self.key.device_generation
        {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }
    fn check_target(&self, target: &OffscreenTarget) -> Result<(), RenderError> {
        self.check_owner(target)?;
        if target.frame_revision != self.frame_revision {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }
    /// Inspect the prepared scene or its scene-plus-readback graph.
    /// # Errors
    /// Refuses a stale target generation or invalid typed resource/pass declarations.
    pub fn graph(
        &self,
        target: &OffscreenTarget,
        readback: bool,
    ) -> Result<CompiledGraph, RenderError> {
        self.check_target(target)?;
        Ok(target.graph(readback).plan().clone())
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
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        self.encode_inner(encoder, generation, target, timings, None, None)
    }

    /// Encode the scene with an engine contribution drawn at the end of its forward pass, and
    /// optionally its dependent readback.
    /// # Errors
    /// As [`Self::encode`] and [`Self::encode_with_readback`].
    pub(crate) fn encode_contributed(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        target: &OffscreenTarget,
        timings: Option<&mut GpuTimings>,
        readback: Option<&mut PixelReadback>,
        contribution: Option<&mut (dyn SceneContribution + 'static)>,
    ) -> Result<(), RenderError> {
        if let Some(readback) = readback.as_deref() {
            readback.check_target(target)?;
        }
        self.encode_inner(encoder, generation, target, timings, readback, contribution)
    }

    /// Encode the same scene with its dependent Copy pass in one checked graph.
    /// The readback buffer maps only when the caller submits the encoder.
    /// # Errors
    /// Refuses a pending or foreign readback before recording any GPU pass.
    pub fn encode_with_readback(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        target: &OffscreenTarget,
        timings: Option<&mut GpuTimings>,
        readback: &mut PixelReadback,
    ) -> Result<(), RenderError> {
        readback.check_target(target)?;
        self.encode_inner(encoder, generation, target, timings, Some(readback), None)
    }

    fn encode_inner<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        generation: u64,
        target: &'a OffscreenTarget,
        mut timings: Option<&'a mut GpuTimings>,
        readback: Option<&'a mut PixelReadback>,
        contribution: Option<&'a mut (dyn SceneContribution + 'static)>,
    ) -> Result<(), RenderError> {
        self.check_target(target)?;
        if generation != self.key.device_generation {
            return Err(RenderError::StaleTarget);
        }
        let graph = target.graph(readback.is_some());
        if graph.plan().device_generation != generation {
            return Err(RenderError::StaleTarget);
        }
        if let Some(timings) = timings.as_deref_mut() {
            let count = u32::try_from(graph.plan().pipeline_pass_count())
                .map_err(|_| RenderError::GraphPassMismatch)?;
            timings.check_passes(generation, count)?;
        }
        graph.execute(&mut SceneContext {
            prepared: self,
            target,
            encoder,
            timings,
            generation,
            readback,
            contribution,
            contribution_frame: self.contribution_frame(),
            cooked_shadow_fit: None,
            exposure_side: self.history.write_side(),
        })
    }

    fn encode_opaque(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
        contribution: Option<&dyn SceneContribution>,
        exposure_side: usize,
    ) -> Result<(), RenderError> {
        let mut pass = compiled
            .begin_render(
                encoder,
                [
                    Some(GraphColorAttachment {
                        resource: &target.ids.scene,
                        range: 0..1,
                        view: &target.scene_view,
                        clear: self.background,
                    }),
                    Some(GraphColorAttachment {
                        resource: &target.ids.motion,
                        range: 0..1,
                        view: &target.motion_view,
                        clear: wgpu::Color::TRANSPARENT,
                    }),
                ],
                Some(GraphDepthAttachment {
                    resource: &target.ids.depth,
                    range: 0..1,
                    view: &target.depth,
                    clear: 0.0,
                }),
                self.key.device_generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        if let Some(atmosphere) = self.atmosphere.as_ref() {
            atmosphere.draw(&mut pass, exposure_side);
        }
        pass.set_pipeline(if self.instance_upload.two_sided_count() == 0 {
            &self.pipeline
        } else {
            &self.pipeline_two_sided
        });
        let view_offset = self.view_stride * u32::from(target.frame_revision & 1 != 0);
        pass.set_bind_group(0, &target.view_bind_group, &[view_offset]);
        pass.set_bind_group(1, &self.light[exposure_side], &[]);
        pass.set_bind_group(2, &self.materials, &[]);
        pass.set_bind_group(3, self.history.read_group(), &[]);
        let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
        let indices = if matches!(view.projection(), Projection::OrthographicFixture) {
            &self.cube_reversed_indices
        } else {
            &self.cube_indices
        };
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
        self.draw_camera(&mut pass, target);
        if let Some(contribution) = contribution {
            contribution.draw(&mut pass);
        }
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
    capacity: u64,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: capacity,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
}

struct TargetResources {
    depth: ResourceId,
    scene: ResourceId,
    motion: ResourceId,
    output: ResourceId,
    shadow: Option<ResourceId>,
    air_shadow: Option<ResourceId>,
}

struct SceneGraphs<G> {
    scene: G,
    readback: G,
}

/// Generation-bound offscreen target. Retain while encoded/submitted work uses
/// it; actual old/new allocation retirement belongs to the executor's fences.
pub struct OffscreenTarget {
    device: wgpu::Device,
    prepared_owner: Arc<()>,
    day_graphs: SceneGraphs<ExecutableGraph<SceneFamily>>,
    night_graphs: Option<SceneGraphs<ExecutableGraph<SceneFamily>>>,
    draw_stars: bool,
    textures: GraphTextures,
    ids: TargetResources,
    view: wgpu::TextureView,
    scene_view: wgpu::TextureView,
    motion_view: wgpu::TextureView,
    display: DisplayTarget,
    view_bind_group: wgpu::BindGroup,
    aux_buffer: wgpu::Buffer,
    shadow: Option<ShadowTarget>,
    air_shadow: Option<AirShadowTarget>,
    visibility: Option<VisibilityTarget>,
    stars: Option<StarTarget>,
    depth: wgpu::TextureView,
    width: u32,
    height: u32,
    generation: u64,
    row_bytes: u32,
    frame_revision: u64,
}
impl OffscreenTarget {
    /// Allocate a nonempty bounded target and display bind group from
    /// one prepared view's generation, on the executor's device. The
    /// prepared view retains both pipelines across resize and additional targets.
    /// # Errors
    /// Refuses empty/overflowing targets or extents beyond the device limits.
    pub fn new(
        device: &wgpu::Device,
        prepared: &PreparedSynthetic,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        if !prepared.capabilities.matches(device) {
            return Err(RenderError::StaleTarget);
        }
        let limits = prepared.capabilities.limits();
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
        let scene_graph::CompiledScene { day, night, ids } =
            prepared.compile_graph(width, height, row_bytes)?;
        // The night capture adds buffer inputs and extends depth, scene and output lifetimes.
        let textures = GraphTextures::new(device, &night.as_ref().unwrap_or(&day).readback);
        let view = textures
            .texture(&ids.output)
            .ok_or(RenderError::GraphPassMismatch)?
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth = textures
            .texture(&ids.depth)
            .ok_or(RenderError::GraphPassMismatch)?
            .create_view(&wgpu::TextureViewDescriptor::default());
        let scene_view = textures
            .texture(&ids.scene)
            .ok_or(RenderError::GraphPassMismatch)?
            .create_view(&wgpu::TextureViewDescriptor::default());
        let motion_view = textures
            .texture(&ids.motion)
            .ok_or(RenderError::GraphPassMismatch)?
            .create_view(&wgpu::TextureViewDescriptor::default());
        let initial_view = prepared
            .pending_view
            .as_ref()
            .unwrap_or(&prepared.view_sample);
        let (stars, draw_stars) = prepared
            .stars
            .as_ref()
            .zip(prepared.atmosphere.as_ref())
            .map_or((None, false), |(stars, atmosphere)| {
                let (binding, draw) = stars.target(
                    device,
                    &depth,
                    atmosphere,
                    &prepared.exposure.samples,
                    (initial_view, [width, height]),
                );
                (Some(binding), draw)
            });
        let display = prepared
            .display
            .target(device, &scene_view, &prepared.exposure.samples);
        let aux = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic target view aux"),
            contents: &prepared
                .pending_view
                .unwrap_or(prepared.view_sample)
                .pack_aux(width, height),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shadow = match (&prepared.shadow, &ids.shadow) {
            (Some(_prepared_shadow), Some(id)) => Some(ShadowTarget::new(
                textures.texture(id).ok_or(RenderError::GraphPassMismatch)?,
            )),
            (None, None) => None,
            _ => return Err(RenderError::GraphPassMismatch),
        };
        let air_shadow = match (&prepared.air_shadow, &ids.air_shadow) {
            (Some(air), Some(id)) => Some(air.target(
                device,
                textures.texture(id).ok_or(RenderError::GraphPassMismatch)?,
                prepared.shadow.as_ref().zip(shadow.as_ref()),
                &prepared.directional_buffer,
            )?),
            (None, None) => None,
            _ => return Err(RenderError::GraphPassMismatch),
        };
        let visibility = prepared.camera_visibility_target(device);
        let mut view_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &prepared.view_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(VIEW_BYTES as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: aux.as_entire_binding(),
            },
        ];
        view_entries.extend(camera_culling::visible_entry(visibility.as_ref()));
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
            ]);
        }
        view_entries.extend(air_shadow.as_ref().map(AirShadowTarget::view_entry));
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic view and target aux"),
            layout: &prepared.view_layout,
            entries: &view_entries,
        });
        let day_graphs = day.bind(prepared)?;
        let night_graphs = night.map(|graphs| graphs.bind(prepared)).transpose()?;
        Ok(Self {
            device: device.clone(),
            prepared_owner: Arc::clone(&prepared.owner),
            day_graphs,
            night_graphs,
            draw_stars,
            textures,
            ids,
            view,
            scene_view,
            motion_view,
            display,
            view_bind_group,
            aux_buffer: aux,
            shadow,
            air_shadow,
            visibility,
            stars,
            depth,
            width,
            height,
            generation: prepared.key.device_generation,
            row_bytes,
            frame_revision: prepared.frame_revision,
        })
    }
    fn graph(&self, readback: bool) -> &ExecutableGraph<SceneFamily> {
        let graphs = self
            .night_graphs
            .as_ref()
            .filter(|_| self.draw_stars)
            .unwrap_or(&self.day_graphs);
        if readback {
            &graphs.readback
        } else {
            &graphs.scene
        }
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
    pub(crate) fn belongs_to_device(&self, device: &wgpu::Device) -> bool {
        self.device == *device
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
