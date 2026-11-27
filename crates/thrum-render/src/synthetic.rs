//! Prepared in-memory instanced diagnostic. It exercises the real material and
//! instance tables, four texture-class bindings, graph declaration and optional
//! timestamps. One directional metallic-roughness closure and opaque depth are
//! real, followed by a fixed authored SDR display transform. Fixed-view
//! selective GPU local-light lists and two synthetic Sun cascades run here;
//! a checked constant-light presentation handoff and the general render scene
//! remain open.

use crate::abi::{self, InstanceRecord, MaterialRecord, TextureClasses};
use crate::atmosphere::{AtmosphereFrame, AtmospherePrepared, CloudPrepared, SunOpticalDepth};
use crate::capabilities::{DeviceCapabilities, RayStructureSelection};
pub use crate::capabilities::{ShadowSelection, SunShadowMethod};
use crate::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SurfaceParts,
};
use crate::graph::{
    CompiledPass, GraphColorAttachment, GraphDepthAttachment, GraphTextures, PipelineKey,
    ResourceId,
};
use crate::graph_record::{ExecutableGraph, RecordFamily};
use crate::light::{DIRECTIONAL_BYTES, DirectionalLight};
use crate::pass_key::PipelineRole;
use crate::timing::GpuTimings;
use crate::visibility::casters::{CasterCulling, CasterInputs};
use crate::visibility::{VisibilityPrepared, VisibilityTarget};
pub use camera_culling::{CountedFrame, CullMode, CullingCounts, CullingReason, CullingSelection};
use glam::Vec3;
use std::sync::Arc;
use wgpu::util::DeviceExt;

mod after_opaque;
mod air_shadow;
mod buckets;
pub(crate) use buckets::MAX_BUCKETS;
pub(crate) mod camera_culling;
mod cluster;
mod cluster_gpu;
#[cfg(feature = "creation-counters")]
mod cluster_probe;
#[cfg(feature = "creation-counters")]
pub use cluster_probe::LightListProbe;
mod contributed;
pub(crate) mod display;
pub(crate) mod exposure;
mod forward;
pub(crate) mod frame_stats;
mod history_gpu;
mod mesh_arena;
mod mesh_route;
pub mod mesh_rows;
pub(crate) mod native_witness;
mod preparation;
mod raytrace;
pub(crate) use raytrace::RetainedRayMeshes;
pub(crate) use raytrace::ambient::{
    Stage as AmbientStage, shader_source as ambient_visibility_shader,
};
pub(crate) use raytrace::soft::{
    COMBINED_TRACE, Stage as SoftSunStage, combined_shader_source as combined_visibility_shader,
    shader_source as soft_sun_shader,
};
#[cfg(all(feature = "creation-counters", target_os = "macos"))]
pub use raytrace::{RayBuildProbe, RayBuildSample, RayStructureInstance};
pub(crate) mod readback;
mod scale;
pub use scale::RenderScale;
#[cfg(feature = "creation-counters")]
mod scale_witness;
mod scene_apply;
mod scene_graph;
pub(crate) mod scene_source;
pub(crate) mod secondary;
mod shadow;
pub(crate) mod stars;
mod taau;
mod target;
mod target_inputs;
pub(crate) use target_inputs::SyntheticTargetInputs;
mod target_storage;
mod temporal;
mod upload;
mod view;
pub(crate) use crate::apple_tile::NativeFrame;
pub use crate::apple_tile::metalfx::MetalFxProbe;
use crate::apple_tile::{NativeBinding, NativeGraph, NativeProvider};
pub use crate::apple_tile::{
    NativeOutcome, NativeProviderReport, NativeSegment, NativeTwinSelection, TwinReason,
};
pub use native_witness::{NativeWitness, NativeWitnessReport};
pub use readback::{CaptureSource, PixelReadback};
pub use temporal::TEMPORAL_WITNESS_JSON;
pub use view::Reconstruction;

pub(crate) use air_shadow::AIR_SHADOW_PASS;

use air_shadow::{AirShadowPrepared, AirShadowTarget};
use cluster::ClusterTable;
pub use cluster::SyntheticPointLight;
use cluster_gpu::ClusterCompute;
pub(crate) use contributed::CONTRIBUTION_COMPUTE;
#[cfg(target_os = "macos")]
pub use display::mac_backing_grid_requested;
pub use display::{DisplayOverlay, SdrToneMap};
pub(crate) use display::{
    PreToneInput, PreToneViews, PreparedPreToneInput, SdrToneStage, ToneOutput,
};
use exposure::ExposureResolve;
pub(crate) use frame_stats::{SceneTimingLayout, validate_pass_timings};
use history_gpu::TransformHistory;
use mesh_arena::{ArenaLimits, MeshArena};
use mesh_route::{BucketRoute, MeshSelection, admit_meshes};
pub use scene_apply::SceneApplyReport;
use shadow::{CasterBox, CasterDraw, CasterDraws, ShadowPrepared, ShadowTarget};
use stars::{StarPrepared, StarTarget};
use upload::{InstanceUpload, PackedInstances};
pub use view::SyntheticView;
pub(crate) use view::ViewDiscontinuity;
use view::{AUX_BYTES, Projection};

use crate::error::RenderError;
pub use crate::render_config::{DEPTH_FORMAT, OUTPUT_FORMAT, SYNTHETIC_REVISION};
use crate::render_config::{PRE_EXPOSURE, SCENE_FORMAT};

const VIEW_BYTES: usize = view::VIEW_BYTES;
const VIEW_SLOTS: u32 = 2;
const MOTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
const CUBE_INDICES: [u16; 36] = [
    0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7, 8, 9, 10, 10, 9, 11, 12, 13, 14, 14, 13, 15, 16, 17, 18,
    18, 17, 19, 20, 21, 22, 22, 21, 23,
];

/// Immutable CPU scene, converted at prepare time rather than each draw.
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent scene preparation requests"
)]
pub struct SyntheticScene {
    /// Ask preparation for a scene-wide ray structure for diagnostics.
    /// Preparation falls back to the ordinary scene if the device or recipe cannot trace.
    pub ray_structure: bool,
    /// Optional Sun-path optical-depth provider. Presence is fixed at prepare
    /// time. Metadata and same-sized contents may change on the live frame.
    pub sun_optical_depth: Option<SunOpticalDepth>,
    /// Optional ground wetness field. Its presence and shape are fixed at preparation.
    pub wetness: Option<crate::wetness::WetnessProvider>,
    /// Allocate a world-space cloud sky overlay for a scene contribution.
    pub sky_overlay: bool,
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
    /// Main view's Sun visibility method, when the Sun casts shadows.
    pub sun_shadow_method: SunShadowMethod,
    /// Prepare soft Sun visibility when the scene resolves to traced Sun shadows.
    pub sun_penumbra: bool,
    /// Ask for hard shadows from the main view's local lights.
    pub lamp_shadows: bool,
    /// Maximum traced ambient occlusion distance in metres.
    pub ambient_occlusion_radius: Option<f32>,
    /// Camera opaque draw: the direct oracle or GPU frustum compaction.
    pub camera_culling: CullMode,
    /// Optional immutable unit equatorial directions with incident lux in w.
    pub star_catalogue: Option<&'static [[f32; 4]]>,
    /// Each instance slot's stable mesh choice. A live selection prepares the
    /// newest committed geometry on reconstruction. `None` draws the unit cube.
    pub meshes: Option<Arc<SyntheticMeshes>>,
    /// Authored row indices in [`SyntheticMeshes::meshes`] that are aloft.
    /// They cast into both Sun cascades and are omitted from the terrain
    /// horizon. The mark is fixed at preparation and survives row replacement.
    /// Preparation refuses indices outside the selected mesh table.
    pub aloft_mesh_rows: Vec<u32>,
}

pub use mesh_arena::MeshVertex as SyntheticVertex;

/// One indexed triangle list in its local space. Triangles wind
/// counter-clockwise seen from outside.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticMesh {
    /// Vertices, at most the device's storage-binding limit over all meshes.
    pub vertices: Vec<SyntheticVertex>,
    /// Triangle-list indices into this mesh's own `vertices`.
    pub indices: Vec<u32>,
}

/// One logical slot's mesh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntheticMeshChoice {
    /// The unit cube every scene without a selection draws.
    Cube,
    /// An index into [`SyntheticMeshes::meshes`].
    Mesh(u32),
}

/// Authored meshes and each logical instance slot's choice among them.
///
/// The choice belongs to the slot, not to an entity: it survives short
/// uploads, inactive slots, physical compaction and class changes, and an
/// entity that reuses a slot draws that slot's mesh. Changing a slot's mesh
/// means preparing again.
///
/// Preparation checks the whole selection before any GPU work and refuses it
/// when:
/// - `slots` does not hold exactly one choice per scene instance;
/// - a choice names an index outside `meshes`;
/// - a mesh has no triangles, an index count that is not a multiple of
///   three, an index at or past its vertex count, or a non-finite value;
/// - a mesh has a referenced coordinate beyond ±1e30 m, so its transformed
///   bounds could not be represented;
/// - the meshes exceed the device's vertex or index storage limits.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticMeshes {
    /// Authored meshes, in addition to the cube.
    pub meshes: Vec<SyntheticMesh>,
    /// One choice per scene instance, in logical-slot order.
    pub slots: Vec<SyntheticMeshChoice>,
    /// Optional publication handshake for bounded live replacement of these rows.
    pub updates: Option<mesh_rows::MeshRows>,
}

/// The largest admitted mesh coordinate magnitude. Upload admits transform
/// components up to 1e6, so a transformed bound (three basis products plus a
/// translation, then a unit light axis) stays below 1e37, inside `f32`'s
/// 3.4e38, through the Sun fit and containment.
const MESH_COORDINATE_LIMIT: f32 = 1e30;

impl SyntheticScene {
    pub(crate) fn ray_admission(&self, capabilities: &DeviceCapabilities) -> RayStructureSelection {
        capabilities.resolve_ray_structure(
            self.ray_structure
                || self.view.traced_geometry()
                || self.sun_shadow_method == SunShadowMethod::Traced
                || self.lamp_shadows
                || self.ambient_occlusion_radius.is_some(),
        )
    }

    /// Empty CPU storage for a host that never enters the 3D renderer.
    pub(crate) fn empty() -> Self {
        Self {
            ray_structure: false,
            sun_shadow_method: SunShadowMethod::Maps,
            sun_penumbra: false,
            lamp_shadows: false,
            ambient_occlusion_radius: None,
            sun_optical_depth: None,
            wetness: None,
            sky_overlay: false,
            materials: Vec::new(),
            instances: Vec::new(),
            local_lights: Vec::new(),
            view: SyntheticView::default(),
            sun_shadows: false,
            camera_culling: CullMode::Direct,
            star_catalogue: None,
            meshes: None,
            aloft_mesh_rows: Vec::new(),
        }
    }

    /// Generate a bounded regular grid. Instance IDs remain stable; one draw
    /// renders the active prefix. The default view uses the historical orthographic
    /// diagnostic coordinates; callers may supply a checked perspective view.
    /// # Errors
    /// Refuses zero instances or more than one million diagnostic instances.
    pub fn grid(count: u32) -> Result<Self, RenderError> {
        if count == 0 || count > 1_000_000 {
            return Err(RenderError::GridInstanceCount);
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
            sun_optical_depth: None,
            wetness: None,
            sky_overlay: false,
            materials,
            instances,
            local_lights: Vec::new(),
            view: SyntheticView::default(),
            sun_shadows: false,
            sun_shadow_method: SunShadowMethod::Maps,
            sun_penumbra: false,
            lamp_shadows: false,
            ambient_occlusion_radius: None,
            camera_culling: CullMode::Direct,
            star_catalogue: None,
            meshes: None,
            aloft_mesh_rows: Vec::new(),
            ray_structure: false,
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
    fn pack(self) -> Result<DirectionalLight, RenderError> {
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
) -> Result<(DirectionalLight, wgpu::Color, usize), RenderError> {
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
            let stored_lux_limit =
                crate::shader_constants::MAX_FINITE_HALF * SOLAR_DISK_SOLID_ANGLE / PRE_EXPOSURE;
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
    fn pack(self) -> Result<(DirectionalLight, wgpu::Color, usize), RenderError> {
        const MAX_PRESENTATION_LIGHT: f64 = 10_000_000.0;
        // Retain F3's exact accepted predicate at this boundary.
        let [r, g, b] = crate::shader_constants::REC709_LUMA;
        let luminance = r * self.sun_colour[0] + g * self.sun_colour[1] + b * self.sun_colour[2];
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
) -> Result<DirectionalLight, RenderError> {
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
    Ok(DirectionalLight {
        direction_to_light: Vec3::from_array(direction.map(|value| value as f32))
            .extend(0.0)
            .to_array(),
        irradiance: Vec3::from_array(irradiance).extend(0.0).to_array(),
        half_vector_fresnel: Vec3::from_array(half.map(|value| value as f32))
            .extend(angular_fresnel)
            .to_array(),
        ambient_irradiance: Vec3::from_array(ambient).extend(0.0).to_array(),
    })
}

/// A slot's caster box: its selected mesh bounds, or the legacy unit cube.
fn caster_box(boxes: Option<&[CasterBox]>, slot: usize) -> CasterBox {
    boxes
        .and_then(|boxes| boxes.get(slot).copied())
        .unwrap_or(CasterBox::UNIT)
}

fn packed_light_direction(data: &DirectionalLight) -> Vec3 {
    glam::Vec4::from_array(data.direction_to_light).truncate()
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
    pub(crate) native: Option<Arc<dyn NativeProvider>>,
    culling_selection: CullingSelection,
    temporal_inputs: bool,
    temporal: temporal::TemporalInputs,
    taau: Option<taau::Taau>,
    cloud: Option<CloudPrepared>,
    wetness: Option<crate::wetness::PreparedWetness>,
    capabilities: DeviceCapabilities,
    classes: TextureClasses,
    owner: Arc<()>,
    key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    pipeline_two_sided: wgpu::RenderPipeline,
    dry_pipelines: Option<[wgpu::RenderPipeline; 2]>,
    /// Occlusion only: the depth-only twins of `pipeline` and
    /// `pipeline_two_sided`, for the depth prepass.
    depth_prepass: Option<[wgpu::RenderPipeline; 2]>,
    ambient_receiver_pipelines: Option<[wgpu::RenderPipeline; 2]>,
    /// Camera and Sun caster passes pull vertices and indices from the arena;
    /// it holds the cube (row 0) and its orthographic winding (row 1) over
    /// shared vertices.
    mesh_arena: MeshArena,
    ray_structure: Option<raytrace::SceneStructure>,
    traced_view: Option<Box<raytrace::TracedView>>,
    ray_structure_selection: RayStructureSelection,
    shadow_selection: ShadowSelection,
    lamp_shadows: Option<raytrace::LampShadows>,
    soft_sun: Option<raytrace::soft::SoftSun>,
    ambient: Option<raytrace::ambient::Ambient>,
    ambient_selection: crate::capabilities::AmbientOcclusionSelection,
    row_updates: Option<mesh_rows::LiveRows>,
    /// Scenes that select meshes draw per bucket; others keep the legacy
    /// single-pipeline route.
    bucket_route: Option<BucketRoute>,
    /// Each slot's selected mesh bounds, for the Sun fit and containment.
    /// `None` keeps the unit cube.
    caster_boxes: Option<Vec<CasterBox>>,
    /// The bucket route's member list for the Sun caster passes.
    shadow_members: Option<wgpu::BindGroup>,
    /// Culled camera modes on the bucket route: each Sun pass's caster list
    /// against its own light volume, and that list bound like the members.
    caster_culling: Option<(CasterCulling, Vec<wgpu::BindGroup>)>,
    /// The arena cube's checked index range and base vertex.
    cube_draw: (std::ops::Range<u32>, i32),
    display: SdrToneStage,
    pub(crate) interface_inputs: Option<crate::sprite::interface::SpriteInputs>,
    view_layout: crate::runtime::PipelineInput<wgpu::BindGroupLayout>,
    view_buffer: wgpu::Buffer,
    view_stride: u32,
    view_sample: SyntheticView,
    jitter_phase: u32,
    pending_view: Option<SyntheticView>,
    frame_revision: u64,
    directional_buffer: wgpu::Buffer,
    environment_light: wgpu::Buffer,
    local_light_buffer: wgpu::Buffer,
    cluster_head_buffer: wgpu::Buffer,
    cluster_index_buffer: wgpu::Buffer,
    clusters: ClusterTable,
    cluster_compute: Option<ClusterCompute>,
    light: [wgpu::BindGroup; 2],
    map_light_layout: crate::runtime::PipelineInput<wgpu::BindGroupLayout>,
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
    effect_graph: crate::contribution::EffectGraph,
    effect_time: crate::contribution::EffectTime,
    /// Camera culling pipelines; `None` is the Direct oracle with no extra resources.
    visibility: Option<VisibilityPrepared>,
    stars: Option<StarPrepared>,
    secondary: Vec<secondary::SecondaryPrepared>,
    secondary_handles: Vec<crate::contribution::SecondaryView>,
    secondary_sources: Vec<SyntheticPointLight>,
}

struct SceneFamily;

impl RecordFamily for SceneFamily {
    type Frame<'a> = SceneContext<'a>;
    type Error = RenderError;
}

type LateSurface<'a> = (
    &'a crate::present::SurfacePresenter,
    &'a crate::present::PresentationBinding,
    &'a wgpu::TextureView,
    [u32; 2],
);

struct SceneContext<'a> {
    interface: Option<&'a crate::sprite::interface::PreparedInterface>,
    native: Option<&'a NativeFrame>,
    segments: Option<&'a mut Vec<wgpu::CommandBuffer>>,
    surface: Option<LateSurface<'a>>,
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
    overlay_written: bool,
}

impl SceneContext<'_> {
    fn atmosphere(
        &mut self,
        stage: crate::pass_key::AtmosphereStage,
        pass: &CompiledPass,
    ) -> Result<(), RenderError> {
        let atmosphere = self
            .prepared
            .atmosphere
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        if stage == crate::pass_key::AtmosphereStage::Aerial
            && let (Some(volume), Some(scene)) =
                (&atmosphere.view_volume, &self.target.volume_group)
        {
            let mut compute = pass
                .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
                .ok_or(RenderError::GraphPassMismatch)?;
            volume.dispatch(&mut compute, scene);
            return Ok(());
        }
        atmosphere.encode(
            stage,
            self.encoder,
            pass,
            self.timings.as_deref_mut(),
            self.overlay_written || self.prepared.effect_graph().sky_overlays.is_empty(),
        )
    }

    /// Write this recording's Sun fit once, before the first pass that reads
    /// the caster views; whether the cooked contribution draws into it.
    fn shadow_fit(&mut self) -> Result<bool, RenderError> {
        let shadow = self
            .prepared
            .shadow
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        Ok(*self.cooked_shadow_fit.get_or_insert_with(|| {
            let bounds = self
                .contribution
                .as_deref()
                .and_then(SceneContribution::shadow_bounds);
            shadow.write_recording(bounds, self.prepared.cascade_casters())
        }))
    }

    /// Every Sun pass's caster list, against the views this recording fits.
    fn caster_cull(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.shadow_fit()?;
        let (culling, _) = self
            .prepared
            .caster_culling
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let result = if self.prepared.cascade_casters() {
            culling.encode(
                self.encoder,
                self.generation,
                pass,
                self.timings.as_deref_mut(),
            )
        } else {
            culling.encode_range(
                self.encoder,
                self.generation,
                pass,
                self.timings.as_deref_mut(),
                shadow::HEIGHT_LAYER..shadow::MAPS,
            )
        };
        result?.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }

    fn shadow(&mut self, index: usize, pass: &CompiledPass) -> Result<(), RenderError> {
        let draw_contribution = self.shadow_fit()?;
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
        let prepared = &self.prepared;
        let arena = &prepared.mesh_arena;
        let buffers = arena.buffers().ok_or(RenderError::GraphPassMismatch)?;
        // Every admitted caster draws, on screen or not: the route's members,
        // or the cube over the physical rows.
        let (indices, base_vertex) = prepared.cube_draw.clone();
        let cube = [CasterDraw {
            bucket: 0,
            member_offset: 0,
            indices,
            base_vertex,
            instances: prepared.instance_upload.active_count(),
        }];
        let casters = CasterDraws {
            indices: &buffers.indices,
            two_sided: prepared.instance_upload.two_sided_count() != 0,
            members: prepared.shadow_members.as_ref(),
            culled: match &prepared.caster_culling {
                Some((culling, groups)) => Some(
                    groups
                        .get(index)
                        .zip(culling.lists().nth(index).map(|(_, args)| args))
                        .ok_or(RenderError::GraphPassMismatch)?,
                ),
                None => None,
            },
            draws: prepared.bucket_route.as_ref().map_or(&cube, |route| {
                if index == shadow::HEIGHT_LAYER {
                    route.height_caster_draws()
                } else {
                    route.caster_draws()
                }
            }),
        };
        shadow.encode_cascade(
            self.encoder,
            shadow_target,
            prepared.history.read_group(),
            &casters,
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
                self.contribution
                    .as_deref()
                    .map(shadow::CasterContribution::Main)
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
            if !self.prepared.secondary.is_empty() {
                let (views, count) = self.prepared.upload_secondary_views();
                if contribution.secondary_views_enabled() {
                    contribution.encode_secondary_upload(
                        self.encoder,
                        &self.contribution_frame,
                        &views[..count],
                    );
                }
            }
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
        if let Some(traced) = &self.prepared.traced_view
            && self
                .prepared
                .pending_view
                .unwrap_or(self.prepared.view_sample)
                .traced_geometry()
        {
            return traced.encode(self, pass);
        }
        self.prepared.display.encode(
            self.encoder,
            self.generation,
            &self.target.display,
            pass,
            ToneOutput {
                resource: &self.target.ids.output,
                view: &self.target.view,
                exposure_side: self.exposure_side,
            },
            self.timings.as_deref_mut(),
        )
    }

    fn interface(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let interface = self.interface.ok_or(RenderError::GraphPassMismatch)?;
        let view = self
            .target
            .interface_view
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        interface.encode_sdr(
            self.encoder,
            pass,
            GraphColorAttachment {
                resource: &self.target.ids.output,
                range: 0..1,
                view,
                clear: wgpu::Color::BLACK,
            },
            self.timings.as_deref_mut(),
        )
    }

    fn copy(&mut self, _pass: &CompiledPass) -> Result<(), RenderError> {
        self.readback
            .as_deref_mut()
            .ok_or(RenderError::GraphPassMismatch)?
            .copy_into(
                self.encoder,
                self.target,
                Some(&self.prepared.exposure.samples[self.exposure_side]),
                self.native,
            )
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

    async fn prepare_with_lighting(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        exposure: f32,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_meshes(capabilities, queue, scene, lighting, exposure, None, None).await
    }

    pub(crate) async fn prepare_host_candidate(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        exposure: f32,
        retained: Option<RetainedRayMeshes>,
        selection: RayStructureSelection,
    ) -> Result<Self, RenderError> {
        capabilities.recipe().map_err(RenderError::Recipe)?;
        Self::prepare_candidate(
            capabilities,
            queue,
            scene,
            lighting,
            exposure,
            None,
            retained,
            selection,
        )
        .await
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
    /// Timestamped scene passes in this scene's compiled graph: the most render
    /// and compute passes any of its day, night, scene or readback variants
    /// holds. The surface consumer is counted by its presenter. Zero only when
    /// the graph does not compile, which timing refuses as an invalid capacity.
    /// This compiles the scene's graphs, so call it to size a batch once; a
    /// frame host's slots read the count from their targets' compiled graphs.
    pub fn scene_pass_count(&self) -> u32 {
        self.target_inputs()
            .compile_graph(1, 1, CaptureSource::Sdr, None)
            .ok()
            .and_then(|compiled| u32::try_from(compiled.pipeline_pass_count()).ok())
            .unwrap_or(0)
    }

    pub(crate) fn wetness_bytes(&self) -> Option<u64> {
        self.wetness
            .as_ref()
            .map(crate::wetness::PreparedWetness::bytes)
    }

    pub(crate) fn view_volume_storage(&self) -> Option<crate::atmosphere::ViewVolumeStorage> {
        self.atmosphere.as_ref()?.volume_storage()
    }

    pub(crate) fn scene_shader_source(&self) -> &str {
        &self.surface.shader_source
    }

    /// Read the latest prepared atmospheric inputs and diagnostic chart textures.
    /// Chart order is transmittance, multiple scattering, sky view, ground irradiance.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn atmosphere_witness(
        &self,
    ) -> Option<(crate::atmosphere::EnvironmentFrame, [&wgpu::Texture; 4])> {
        let atmosphere = self.atmosphere.as_ref()?;
        Some((atmosphere.environment(), atmosphere.chart_witnesses()))
    }

    #[cfg(all(test, feature = "creation-counters"))]
    pub(crate) fn chart_schedule_witness(&self) -> (&wgpu::Texture, &wgpu::Buffer) {
        let atmosphere = self.atmosphere.as_ref().expect("fixture atmosphere");
        (atmosphere.reflection_witness(), &self.environment_light)
    }

    #[cfg(all(test, feature = "creation-counters"))]
    pub(crate) fn overlay_witness(&self) -> wgpu::Texture {
        self.atmosphere
            .as_ref()
            .and_then(AtmospherePrepared::overlay_target)
            .expect("fixture sky overlay")
    }

    #[cfg(all(test, feature = "creation-counters"))]
    pub(crate) fn overlay_classification_witness(&self) -> &wgpu::Buffer {
        self.atmosphere
            .as_ref()
            .and_then(AtmospherePrepared::overlay_classification_witness)
            .expect("fixture sky overlay classification")
    }

    /// Inspect the two cumulative view-volume textures and the submitted history phase.
    /// The scatter texture's last layer carries the terminal pair for sky rays.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn view_volume_witness(&self) -> Option<([&wgpu::Texture; 2], u16)> {
        let atmosphere = self.atmosphere.as_ref()?;
        Some((
            atmosphere.aerial_textures(),
            atmosphere.view_volume.as_ref()?.submitted_phase(),
        ))
    }
    /// Borrow the coefficient target for a GPU test provider.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn view_volume_medium_witness(&self) -> Option<crate::atmosphere::ViewVolumeMediumTarget> {
        self.atmosphere.as_ref()?.medium_target()
    }

    /// The shared forward lighting ABI, for receiver probes.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn forward_light_witness(&self) -> (&str, &wgpu::BindGroupLayout, &wgpu::BindGroup) {
        (
            &self.surface.shader_source,
            &self.surface.light_layout,
            &self.light[0],
        )
    }

    /// The incident meter output for the frame about to be submitted.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn incident_meter_witness(&self) -> &wgpu::Buffer {
        self.pending_exposure_sample()
    }

    /// The atmosphere operation lent to forward effects, for transport probes.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn environment_products_witness(
        &self,
        queue: &wgpu::Queue,
    ) -> Option<crate::atmosphere::EnvironmentProducts> {
        self.scene_surface(queue).environment_products()
    }

    /// Camera opaque draw mode; Frustum adds one timed compute pass and
    /// Occlusion four timed passes.
    pub fn camera_culling(&self) -> CullMode {
        self.visibility
            .as_ref()
            .map_or(CullMode::Direct, VisibilityPrepared::mode)
    }
    /// Requested and resolved camera modes and the preparation-time reason.
    pub fn culling_selection(&self) -> CullingSelection {
        self.culling_selection
    }
    /// Whether the requested structure was prepared, or why the scene draws without it.
    pub fn ray_structure_selection(&self) -> RayStructureSelection {
        self.ray_structure_selection
    }

    /// Requested and resolved hard-shadow methods for this preparation.
    #[must_use]
    pub fn shadow_selection(&self) -> ShadowSelection {
        self.shadow_selection
    }

    fn surface_uses_cascades(&self) -> bool {
        self.shadow_selection.sun == SunShadowMethod::Maps
    }

    fn cascade_casters(&self) -> bool {
        self.surface_uses_cascades()
            || self
                .atmosphere
                .as_ref()
                .is_some_and(|air| air.view_volume.is_some())
    }

    pub(crate) fn lamp_quality(&self) -> &[crate::contribution::QualityKnob] {
        self.lamp_shadows
            .as_ref()
            .map_or(&[], |lamps| lamps.settings.knobs())
    }

    pub(crate) fn set_lamp_quality(
        &mut self,
        name: &str,
        value: u32,
    ) -> Option<Result<(), crate::contribution::QualityError>> {
        self.lamp_shadows
            .as_mut()
            .filter(|_| name == raytrace::LampShadows::KNOB)
            .map(|lamps| lamps.settings.set(name, value))
    }

    pub(crate) fn commit_lamp_quality(&mut self, queue: &wgpu::Queue) -> bool {
        self.lamp_shadows.as_mut().is_some_and(|lamps| {
            lamps.commit(queue) == crate::contribution::QualityInvalidation::ViewHistory
        })
    }

    /// The counter written by the last encoded forward pass, or absent when
    /// lamp shadows are off. A diagnostic consumer may copy it after encode.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn unshadowed_lamp_counter(&self) -> Option<&wgpu::Buffer> {
        self.lamp_shadows
            .as_ref()
            .map(raytrace::LampShadows::unshadowed)
    }

    /// Runtime controls for the prepared Sun penumbra.
    #[must_use]
    pub fn sun_penumbra_quality(&self) -> &[crate::contribution::QualityKnob] {
        self.soft_sun
            .as_ref()
            .map_or(&[], |soft| soft.settings.knobs())
    }

    pub(crate) fn set_sun_penumbra_quality(
        &mut self,
        name: &str,
        value: u32,
    ) -> Option<Result<(), crate::contribution::QualityError>> {
        self.soft_sun
            .as_mut()
            .filter(|soft| soft.settings.knobs().iter().any(|knob| knob.name == name))
            .map(|soft| soft.settings.set(name, value))
    }

    pub(crate) fn commit_sun_penumbra_quality(&mut self) {
        if let Some(soft) = self.soft_sun.as_mut() {
            soft.commit_settings();
        }
    }

    /// Update the runtime penumbra control outside a frame host.
    ///
    /// # Errors
    /// Returns an error if soft Sun visibility was not prepared.
    #[cfg(feature = "creation-counters")]
    pub fn set_sun_penumbra_for_probe(
        &mut self,
        enabled: bool,
    ) -> Result<(), crate::contribution::QualityError> {
        self.set_sun_penumbra_quality(raytrace::soft::SoftSun::PENUMBRA, u32::from(enabled))
            .ok_or(crate::contribution::QualityError::NoContribution)??;
        self.commit_sun_penumbra_quality();
        Ok(())
    }

    pub(crate) fn atmosphere_quality(&self) -> &[crate::contribution::QualityKnob] {
        self.atmosphere
            .as_ref()
            .map_or(&[], AtmospherePrepared::quality)
    }

    pub(crate) fn set_atmosphere_quality(
        &mut self,
        name: &str,
        value: u32,
    ) -> Option<Result<(), crate::contribution::QualityError>> {
        self.atmosphere.as_mut()?.set_quality(name, value)
    }

    pub(crate) fn commit_atmosphere_quality(&mut self) {
        if let Some(atmosphere) = &mut self.atmosphere {
            atmosphere.commit_quality();
        }
    }

    pub(crate) fn pending_chart_update(&self) -> Option<&'static str> {
        self.atmosphere.as_ref()?.pending_chart_update()
    }

    pub(crate) fn pending_chart_position(&self) -> Option<(u32, u32)> {
        self.atmosphere.as_ref()?.pending_chart_position()
    }

    pub(crate) fn pending_chart_change(&self) -> Option<f32> {
        self.atmosphere.as_ref()?.pending_chart_change()
    }

    pub(crate) fn aerial_age(&self) -> Option<u32> {
        self.atmosphere
            .as_ref()
            .map(AtmospherePrepared::pending_aerial_age)
    }

    /// Set and commit the production lamp quality knob for an offscreen probe.
    /// # Errors
    /// The scene must trace lamps and the limit must fit the cluster capacity.
    #[cfg(feature = "creation-counters")]
    pub fn set_lamp_ray_limit_for_probe(
        &mut self,
        queue: &wgpu::Queue,
        limit: u32,
    ) -> Result<(), crate::contribution::QualityError> {
        self.set_lamp_quality(raytrace::LampShadows::KNOB, limit)
            .ok_or(crate::contribution::QualityError::NoContribution)??;
        self.commit_lamp_quality(queue);
        Ok(())
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
    pub(crate) fn pending_view(&self) -> Option<&SyntheticView> {
        self.pending_view.as_ref()
    }

    pub(crate) fn supports_render_scale(&self) -> bool {
        self.reconstruction() == Reconstruction::Taau
            && (self.native.is_none() || self.view_sample.requests_metalfx())
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
        let output = target.extent();
        let drawn = scene.view.render_scale().extent(output);
        if drawn != output && !self.supports_render_scale() {
            return Err(RenderError::UnsupportedRenderScale);
        }
        let view = if scene.view.jitter_enabled() || scene.view.reconstruction() {
            scene.view.at_jitter_phase(self.jitter_phase, drawn)
        } else {
            scene.view
        };
        if self.atmosphere.is_some() != matches!(lighting, FrameLighting::Atmosphere(_))
            || scene.sky_overlay
                != self
                    .atmosphere
                    .as_ref()
                    .is_some_and(AtmospherePrepared::has_overlay)
        {
            return Err(RenderError::InvalidLighting);
        }
        if let (Some(atmosphere), FrameLighting::Atmosphere(frame)) = (&self.atmosphere, lighting)
            && !atmosphere.accepts(frame)
        {
            return Err(RenderError::InvalidLighting);
        }
        match (&self.wetness, &scene.wetness) {
            (None, None) => {}
            (Some(prepared), Some(provider)) if prepared.accepts(provider) => {}
            _ => return Err(RenderError::WetnessFieldLayout),
        }
        match (&self.cloud, &scene.sun_optical_depth) {
            (None, None) => {}
            (Some(cloud), Some(field)) if cloud.accepts(field) => {}
            _ => return Err(RenderError::CloudFieldLayout),
        }
        if scene.instances.len()
            != usize::try_from(self.instance_count)
                .map_err(|_| RenderError::InstanceCountMismatch)?
        {
            return Err(RenderError::InstanceCountMismatch);
        }
        let (light_data, background, clamps) = pack_frame_lighting(lighting)?;
        if self.cluster_compute.is_some() {
            self.clusters.repack_gpu(&scene.local_lights, &view)?;
        } else {
            self.clusters.rebuild(&scene.local_lights, &view)?;
        }
        for (len, capacity, error) in [
            (
                self.clusters.lights.len(),
                self.local_light_bytes,
                RenderError::LocalLightBufferCapacity,
            ),
            (
                self.clusters.heads.len(),
                self.cluster_head_bytes,
                RenderError::ClusterHeadBufferCapacity,
            ),
            (
                self.clusters.indices.len(),
                self.cluster_index_bytes,
                RenderError::ClusterIndexBufferCapacity,
            ),
        ] {
            if u64::try_from(len).map_or(true, |len| len > capacity) {
                return Err(error);
            }
        }
        let light_data = light_data.with_local_lights(self.clusters.active);
        if let Some(shadow) = self.shadow.as_mut() {
            shadow.update(
                &view,
                (&scene.instances, self.caster_boxes.as_deref()),
                packed_light_direction(&light_data),
            )?;
        }
        self.history.synchronize_key(crate::history::HistoryKey {
            view: 1,
            provider: 0,
            extent: view.output_extent(target.extent()),
            device_generation: self.key.device_generation,
            content_epoch: 0,
            profile_epoch: 0,
            origin_epoch: 0,
            correspondence_epoch: view.history_epoch(),
        });
        self.history
            .prepare_sample(queue, presentation_alpha, view.history_reset());
        if let Some(taau) = &target.taau {
            taau.update(
                queue,
                &view,
                &self.view_sample,
                self.history.candidate_identity(view.history_reset())[1] != 0,
                drawn,
                output,
            );
        }
        if let Some((soft, target)) = self.soft_sun.as_mut().zip(target.soft_sun.as_ref()) {
            soft.update(
                queue,
                target,
                (&view, &self.view_sample),
                drawn,
                self.history.candidate_identity(view.history_reset())[1] != 0,
                packed_light_direction(&light_data),
            );
        }
        if let Some((ambient, target)) = self.ambient.as_mut().zip(target.ambient.as_ref()) {
            ambient.update(
                queue,
                target,
                (&view, &self.view_sample),
                drawn,
                self.history.candidate_identity(view.history_reset())[1] != 0,
            );
        }
        let next_revision = self.frame_revision.wrapping_add(1);
        let view_offset = self.view_stride * u32::from(next_revision & 1 != 0);
        let mut record = view.view_record(&self.view_sample, f32::from_bits(self.exposure_bits));
        if drawn != output {
            record.mip_bias = (drawn[0] as f32 / output[0] as f32).log2() - 1.0;
        }
        queue.write_buffer(
            &self.view_buffer,
            u64::from(view_offset),
            bytemuck::bytes_of(&record),
        );
        queue.write_buffer(&target.aux_buffer, 0, &view.pack_aux(drawn[0], drawn[1]));
        target.draw_stars =
            self.stars
                .as_mut()
                .zip(target.stars.as_ref())
                .is_some_and(|(stars, binding)| {
                    stars.update_target(
                        queue,
                        binding,
                        &view,
                        &self.view_sample,
                        drawn,
                        &light_data,
                    )
                });
        queue.write_buffer(&self.directional_buffer, 0, bytemuck::bytes_of(&light_data));
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
        if let Some(wetness) = &mut self.wetness {
            wetness.upload(queue);
        }
        if let (Some(cloud), Some(field)) = (&mut self.cloud, &scene.sun_optical_depth) {
            cloud.update(
                queue,
                field,
                match lighting {
                    FrameLighting::Atmosphere(frame) => frame.simulation_time_seconds(),
                    _ => 0.0,
                },
            );
        }
        if let (Some(atmosphere), FrameLighting::Atmosphere(frame)) =
            (&mut self.atmosphere, lighting)
        {
            atmosphere.update(
                queue,
                frame,
                &view,
                drawn,
                scene
                    .sun_optical_depth
                    .as_ref()
                    .map(SunOpticalDepth::source_epoch),
            );
        }
        self.exposure.update(
            queue,
            view.exposure_policy(
                f32::from_bits(self.exposure_bits),
                self.atmosphere.is_some(),
            ),
        );
        if !self.secondary.is_empty() {
            self.secondary_sources.clone_from(&scene.local_lights);
        }
        self.pending_view = Some(view);
        self.frame_revision = next_revision;
        target.frame_revision = self.frame_revision;
        target.drawn = drawn;
        Ok(())
    }

    /// Carry submitted CPU view metadata across GPU-resource replacement. The
    /// new GPU history stays invalid, but a prepared view never becomes the prior
    /// submitted camera. Publication copies the live cursor, not a worker snapshot.
    pub(crate) fn inherit_submitted_view(&mut self, previous: &Self) {
        self.jitter_phase = previous.jitter_phase;
        self.view_sample = previous.view_sample;
    }

    pub(crate) fn submitted_view_projection(&self) -> [[f32; 4]; 4] {
        self.view_sample.view_proj_columns()
    }

    /// History validity key used by the GPU transform samples. Its extent is
    /// the output grid even when the render grid changes.
    pub fn temporal_history_key(&self) -> Option<crate::history::HistoryKey> {
        self.history.key()
    }

    /// Candidate and predecessor serials packed for this frame; zero predecessor is invalid.
    pub fn history_identity(&self) -> [u64; 2] {
        self.history.candidate_identity(
            self.pending_view
                .as_ref()
                .is_some_and(SyntheticView::history_reset),
        )
    }

    pub(crate) fn pending_jitter_phase(&self) -> Option<u32> {
        self.pending_view
            .filter(SyntheticView::jitter_enabled)
            .map(|view| view.raster_phase())
    }

    /// Advance retained camera, transform and exposure sides after successful submission.
    pub fn commit_frame(&mut self) {
        if let Some(atmosphere) = self.atmosphere.as_mut() {
            atmosphere.commit();
        }
        if let Some(view) = self.pending_view.take() {
            self.view_sample = view;
            self.jitter_phase = (self.jitter_phase + 1) % 32;
            self.history.submitted();
            if let Some(soft) = self.soft_sun.as_mut() {
                soft.submitted();
            }
            if let Some(ambient) = &mut self.ambient {
                ambient.submitted();
            }
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
        let boxes = self.caster_boxes.as_deref();
        let report =
            self.instance_upload
                .replace(queue, instances, changed_slots, |slot, instance| {
                    instance.is_active()
                        && shadow.is_some_and(|shadow| {
                            !shadow.contains_caster(instance, caster_box(boxes, slot))
                        })
                })?;
        if let Some(visibility) = &mut self.visibility {
            visibility.set_active(queue, self.instance_upload.active_count());
        }
        if let Some(route) = &mut self.bucket_route {
            route.update(queue, &self.instance_upload, instances, changed_slots)?;
            if let Some(visibility) = &mut self.visibility {
                visibility.set_member_groups(route.groups());
            }
            if let Some((culling, _)) = &mut self.caster_culling {
                culling.set_member_groups(route.groups());
            }
            for slot in &mut self.secondary {
                slot.set_caster_member_groups(route.groups());
            }
        }
        Ok(report)
    }
    /// Prepared generation-local finite draw tuple.
    pub fn pipeline_key(&self) -> PipelineKey {
        self.key
    }
}

impl SyntheticTargetInputs {
    fn exposure_key(&self) -> PipelineKey {
        PipelineRole::Exposure.key(self.key)
    }

    fn display_key(&self) -> PipelineKey {
        PipelineRole::Display.key(self.key)
    }
    /// The near and far Sun cascades, then the horizon height map.
    fn shadow_key(&self, index: u32) -> PipelineKey {
        match index {
            0 => PipelineRole::ShadowNear,
            1 => PipelineRole::ShadowFar,
            _ => PipelineRole::HorizonHeights,
        }
        .key(self.key)
    }
    /// The Sun caster cull, on the bucket route in culled camera modes.
    fn caster_cull_key(&self) -> PipelineKey {
        PipelineRole::CasterCull.key(self.key)
    }
    fn air_shadow_key(&self) -> PipelineKey {
        PipelineRole::AirShadow.key(self.key)
    }
    fn cluster_key(&self) -> PipelineKey {
        PipelineRole::Cluster.key(self.key)
    }
    fn history_key(&self) -> PipelineKey {
        PipelineRole::History.key(self.key)
    }
}

impl PreparedSynthetic {
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
        self.encode_inner(
            encoder, generation, target, timings, None, None, None, None, None,
        )
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
        native: Option<&NativeFrame>,
        segments: &mut Vec<wgpu::CommandBuffer>,
        interface: Option<&crate::sprite::interface::PreparedInterface>,
    ) -> Result<(), RenderError> {
        if let Some(readback) = readback.as_deref() {
            readback.check_target(target)?;
        }
        self.encode_inner(
            encoder,
            generation,
            target,
            timings,
            readback,
            contribution,
            native,
            Some(segments),
            interface,
        )
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
        self.encode_inner(
            encoder,
            generation,
            target,
            timings,
            Some(readback),
            None,
            None,
            None,
            None,
        )
    }

    pub(crate) fn pending_exposure_sample(&self) -> &wgpu::Buffer {
        &self.exposure.samples[self.history.write_side()]
    }

    pub(crate) fn write_display_headroom(&self, queue: &wgpu::Queue, headroom: f32) {
        if let Some(stage) = &self.display.edr {
            stage.write_headroom(queue, headroom);
        }
    }

    pub(crate) fn encode_surface(
        &self,
        target: &OffscreenTarget,
        encoder: &mut wgpu::CommandEncoder,
        surface: LateSurface<'_>,
        timings: Option<&mut GpuTimings>,
        readback: bool,
        interface: Option<&crate::sprite::interface::PreparedInterface>,
    ) -> Result<(), RenderError> {
        self.check_target(target)?;
        target
            .graph(readback)
            .execute_after_acquire(&mut SceneContext {
                native: None,
                segments: None,
                interface,
                surface: Some(surface),
                prepared: self,
                target,
                encoder,
                timings,
                generation: self.key.device_generation,
                readback: None,
                contribution: None,
                contribution_frame: self.contribution_frame(target.drawn),
                cooked_shadow_fit: None,
                overlay_written: false,
                exposure_side: self.history.write_side(),
            })
    }

    fn encode_inner<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        generation: u64,
        target: &'a OffscreenTarget,
        mut timings: Option<&'a mut GpuTimings>,
        readback: Option<&'a mut PixelReadback>,
        contribution: Option<&'a mut (dyn SceneContribution + 'static)>,
        native: Option<&'a NativeFrame>,
        segments: Option<&'a mut Vec<wgpu::CommandBuffer>>,
        interface: Option<&'a crate::sprite::interface::PreparedInterface>,
    ) -> Result<(), RenderError> {
        self.check_target(target)?;
        if generation != self.key.device_generation {
            return Err(RenderError::StaleTarget);
        }
        for view in &self.secondary_handles {
            view.begin(
                self.pending_view
                    .as_ref()
                    .unwrap_or(&self.view_sample)
                    .camera_position(),
            );
        }
        let graph = target.graph(readback.is_some());
        if graph.plan().device_generation != generation {
            return Err(RenderError::StaleTarget);
        }
        if let Some(timings) = timings.as_deref_mut() {
            let count = u32::try_from(graph.plan().pipeline_pass_count_before_acquire())
                .map_err(|_| RenderError::GraphPassMismatch)?;
            timings.check_passes(generation, count)?;
        }
        graph.execute_before_acquire(&mut SceneContext {
            native,
            segments,
            interface,
            surface: None,
            prepared: self,
            target,
            encoder,
            timings,
            generation,
            readback,
            contribution,
            contribution_frame: self.contribution_frame(target.drawn),
            cooked_shadow_fit: None,
            overlay_written: false,
            exposure_side: self.history.write_side(),
        })
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
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | draw_witness_usage(),
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
}

const REACTIVE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

const fn draw_witness_usage() -> wgpu::BufferUsages {
    if cfg!(feature = "creation-counters") {
        wgpu::BufferUsages::COPY_SRC
    } else {
        wgpu::BufferUsages::empty()
    }
}

#[cfg(feature = "creation-counters")]
impl PreparedSynthetic {
    pub(crate) fn shadow_view_witness(&self) -> Option<Vec<&wgpu::Buffer>> {
        Some(self.shadow.as_ref()?.caster_views())
    }

    pub(crate) fn main_draw_witness<'a>(
        &'a self,
        target: &'a OffscreenTarget,
    ) -> Vec<(&'a wgpu::Buffer, u64, u64)> {
        let view_offset = u64::from(self.view_stride) * (target.frame_revision & 1);
        let mut buffers = vec![(&self.view_buffer, view_offset, VIEW_BYTES as u64)];
        let mut add = |buffer: &'a wgpu::Buffer| buffers.push((buffer, 0, buffer.size()));
        add(self.instance_upload.buffer());
        if let Some(visibility) = &target.visibility {
            add(visibility.visible());
            add(visibility.args());
        }
        if let Some(shadow) = &self.shadow {
            add(&shadow.sample_buffer);
            for view in shadow.caster_views() {
                add(view);
            }
        }
        if let Some((culling, _)) = &self.caster_culling {
            for (visible, args) in culling.lists() {
                add(visible);
                add(args);
            }
        }
        buffers
    }
}

struct TargetResources {
    secondary: Vec<secondary::Resources>,
    native: Option<NativeGraph>,
    refraction: Option<ResourceId>,
    after_opaque_images: Vec<Vec<ResourceId>>,
    soft_sun: Option<raytrace::soft::SoftResources>,
    pre_tone: PreToneInput,
    temporal: temporal::TemporalResources,
    taau: Option<taau::TaauResources>,
    depth: ResourceId,
    scene: ResourceId,
    motion: ResourceId,
    reactive: ResourceId,
    ambient_receiver: Option<ResourceId>,
    output: ResourceId,
    shadow: Option<ResourceId>,
    air_shadow: Option<ResourceId>,
    /// Occlusion only: the private depth the prepass draws.
    prepass_depth: Option<ResourceId>,
}

struct SceneGraphs<G> {
    scene: G,
    readback: G,
}

/// Generation-bound offscreen target. Retain while encoded/submitted work uses
/// it; actual old/new allocation retirement belongs to the executor's fences.
pub struct OffscreenTarget {
    secondary: Vec<wgpu::BindGroup>,
    pub(crate) native: Option<Arc<NativeBinding>>,
    effects: after_opaque::TargetEffects,
    soft_sun: Option<raytrace::soft::SoftTarget>,
    ambient: Option<raytrace::ambient::Target>,
    capture_source: CaptureSource,
    device: wgpu::Device,
    prepared_owner: Arc<()>,
    day_graphs: SceneGraphs<ExecutableGraph<SceneFamily>>,
    night_graphs: Option<SceneGraphs<ExecutableGraph<SceneFamily>>>,
    draw_stars: bool,
    textures: GraphTextures,
    ids: TargetResources,
    view: wgpu::TextureView,
    interface_view: Option<wgpu::TextureView>,
    scene_view: wgpu::TextureView,
    motion_view: wgpu::TextureView,
    reactive_view: wgpu::TextureView,
    ambient_receiver_view: Option<wgpu::TextureView>,
    temporal: temporal::TemporalTarget,
    taau: Option<taau::TaauTarget>,
    display: PreparedPreToneInput,
    view_bind_group: wgpu::BindGroup,
    aux_buffer: wgpu::Buffer,
    shadow: Option<ShadowTarget>,
    air_shadow: Option<AirShadowTarget>,
    volume_group: Option<wgpu::BindGroup>,
    visibility: Option<VisibilityTarget>,
    occlusion: Option<camera_culling::PrepassCulling>,
    culling_stats: Option<camera_culling::CullingStats>,
    stars: Option<StarTarget>,
    depth: wgpu::TextureView,
    day_depth: Option<(wgpu::Texture, wgpu::TextureView)>,
    drawn: [u32; 2],
    #[cfg(feature = "creation-counters")]
    border_witness: Option<wgpu::RenderPipeline>,
    width: u32,
    height: u32,
    generation: u64,
    capture_layout: readback::CaptureLayout,
    frame_revision: u64,
}

#[cfg(not(test))]
mod target_allocation {
    pub(super) fn started() {}
}

#[cfg(test)]
pub(crate) mod target_allocation {
    use std::cell::Cell;

    thread_local! {
        static STARTS: Cell<u32> = const { Cell::new(0) };
    }

    pub(super) fn started() {
        STARTS.set(STARTS.get() + 1);
    }

    pub(crate) fn starts() -> u32 {
        STARTS.get()
    }
}

impl OffscreenTarget {
    /// Air-shadow height field, when CPU readback was requested at preparation.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    #[must_use]
    pub fn air_shadow_witness(&self) -> Option<&wgpu::Texture> {
        let texture = self.textures.texture(self.ids.air_shadow.as_ref()?)?;
        texture
            .usage()
            .contains(wgpu::TextureUsages::COPY_SRC)
            .then_some(texture)
    }

    #[cfg(feature = "creation-counters")]
    pub(crate) fn shadow_witness(&self) -> Option<&wgpu::Texture> {
        let texture = self.textures.texture(self.ids.shadow.as_ref()?)?;
        texture
            .usage()
            .contains(wgpu::TextureUsages::COPY_SRC)
            .then_some(texture)
    }

    pub(crate) fn reserve_native(
        &self,
        view: &SyntheticView,
    ) -> Result<Option<NativeFrame>, RenderError> {
        self.native
            .as_ref()
            .map(|native| {
                let values = view.native_content([self.width, self.height])?;
                native.reserve(view.history_reset(), values)
            })
            .transpose()
    }

    /// Diagnostic provider outcomes for this extent/generation, when enabled.
    #[must_use]
    pub fn native_witness_report(&self) -> Option<NativeWitnessReport> {
        self.native.as_deref().and_then(native_witness::report)
    }
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
        Self::with_capture_source(device, prepared, width, height, CaptureSource::Sdr)
    }

    /// Prepare a target whose capture graph reads the declared engine stage.
    /// Normal targets retain the SDR graph and texture usage.
    /// # Errors
    /// Refuses invalid extents or a foreign device, as [`Self::new`].
    pub fn with_capture_source(
        device: &wgpu::Device,
        prepared: &PreparedSynthetic,
        width: u32,
        height: u32,
        capture_source: CaptureSource,
    ) -> Result<Self, RenderError> {
        let inputs = prepared.target_inputs();
        if !inputs.capabilities.matches(device) {
            return Err(RenderError::StaleTarget);
        }
        let compiled = inputs.compile_graph(width, height, capture_source, None)?;
        Self::from_compiled_graph(device, &inputs, [width, height], capture_source, compiled)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "prepares the scene attachments in dependency order"
    )]
    pub(crate) fn from_compiled_graph(
        device: &wgpu::Device,
        prepared: &SyntheticTargetInputs,
        [width, height]: [u32; 2],
        capture_source: CaptureSource,
        compiled: scene_graph::CompiledScene,
    ) -> Result<Self, RenderError> {
        if !prepared.capabilities.matches(device) {
            return Err(RenderError::StaleTarget);
        }
        let recipe = prepared
            .capabilities
            .recipe()
            .map_err(RenderError::Recipe)?;
        target_allocation::started();
        let day_depth = compiled.prepare_day_depth(device, recipe, width, height);
        let scene_graph::CompiledScene {
            day,
            night,
            ids,
            capture_layout,
        } = compiled;
        // The night capture extends scene and output lifetimes.
        let textures = GraphTextures::with_srgb_view(
            device,
            &night.as_ref().unwrap_or(&day).readback,
            recipe,
            prepared.interface_inputs.as_ref().map(|_| &ids.output),
        );
        let texture_view = |id: &ResourceId| {
            textures
                .texture(id)
                .ok_or(RenderError::GraphPassMismatch)
                .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
        };
        let view = texture_view(&ids.output)?;
        let interface_view = prepared
            .interface_inputs
            .as_ref()
            .map(|_| crate::sprite::interface::sdr_view(view.texture()));
        let depth = texture_view(&ids.depth)?;
        let scene_view = texture_view(&ids.scene)?;
        let motion_view = texture_view(&ids.motion)?;
        let reactive_view = texture_view(&ids.reactive)?;
        let effects = prepared.late_bindings(device, &textures, &ids, &depth)?;
        let soft_sun = prepared.prepare_soft_target(
            device,
            &textures,
            ids.soft_sun.as_ref(),
            &depth,
            &motion_view,
            [width, height],
        )?;
        let ambient_receiver_view = ids
            .ambient_receiver
            .as_ref()
            .map(texture_view)
            .transpose()?;
        let ambient = prepared.prepare_ambient_target(
            device,
            &depth,
            &motion_view,
            ambient_receiver_view.as_ref(),
            [width, height],
        );
        let temporal = prepared.temporal.target(
            device,
            &textures,
            &ids.temporal,
            [&depth, &motion_view, &reactive_view],
            &prepared.exposure_samples,
        )?;
        let (stars, draw_stars) = prepared.prepare_star_target(device, &depth, [width, height]);
        let native = prepared
            .native
            .as_ref()
            .zip(ids.native.as_ref())
            .map(|(provider, declaration)| provider.bind(&textures, declaration))
            .transpose()?;
        let taau = prepared.prepare_taau_target(
            device,
            &textures,
            &ids,
            [&scene_view, &depth, &motion_view, &reactive_view],
            native.as_deref(),
        )?;
        let display = prepared.prepare_target_display(
            device,
            &ids,
            &scene_view,
            &motion_view,
            &temporal,
            native.as_deref(),
            taau.as_ref(),
        )?;
        let aux = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic target view aux"),
            contents: &prepared
                .pending_view
                .unwrap_or(prepared.view_sample)
                .pack_aux(width, height),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let (shadow, air_shadow) = prepared.prepare_target_shadows(device, &textures, &ids)?;
        let prepass_depth = ids
            .prepass_depth
            .as_ref()
            .map(|id| {
                textures
                    .texture(id)
                    .map(|texture| {
                        (
                            id.clone(),
                            texture.create_view(&wgpu::TextureViewDescriptor::default()),
                        )
                    })
                    .ok_or(RenderError::GraphPassMismatch)
            })
            .transpose()?;
        let (visibility, occlusion) = prepared.camera_visibility_target(
            device,
            prepass_depth.as_ref().map(|(_, view)| view),
            [width, height],
        );
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
        match (&prepared.bucket_route, &visibility, &prepared.visibility) {
            (Some(_), Some(target), Some(culling)) => view_entries.push(wgpu::BindGroupEntry {
                binding: camera_culling::VISIBLE_BINDING,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: target.visible(),
                    offset: 0,
                    size: culling.segment_bytes().and_then(wgpu::BufferSize::new),
                }),
            }),
            (Some(route), _, _) => {
                view_entries.push(route.members_entry(camera_culling::VISIBLE_BINDING));
            }
            (None, ..) => view_entries.extend(camera_culling::visible_entry(visibility.as_ref())),
        }
        if let (Some(prepared_shadow), Some(target_shadow)) = (&prepared.shadow, &shadow) {
            view_entries.extend([
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: prepared_shadow.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&target_shadow.array_view),
                },
            ]);
        }
        view_entries.extend(air_shadow.as_ref().map(AirShadowTarget::view_entry));
        let secondary = prepared
            .secondary
            .iter()
            .map(|slot| slot.target(device, &view_entries, prepared.bucket_route.as_ref()))
            .collect();
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic view and target aux"),
            layout: &prepared.view_layout,
            entries: &view_entries,
        });
        let occlusion = camera_culling::prepass_view_group(
            device,
            &prepared.view_layout,
            &view_entries,
            occlusion,
            prepass_depth,
        );
        let day_graphs = day.bind(prepared)?;
        let night_graphs = night.map(|graphs| graphs.bind(prepared)).transpose()?;
        Ok(Self {
            secondary,
            native,
            capture_source,
            effects,
            soft_sun,
            ambient,
            device: device.clone(),
            prepared_owner: Arc::clone(&prepared.owner),
            day_graphs,
            night_graphs,
            draw_stars,
            textures,
            ids,
            view,
            interface_view,
            scene_view,
            motion_view,
            reactive_view,
            ambient_receiver_view,
            temporal,
            taau,
            display,
            view_bind_group,
            aux_buffer: aux,
            volume_group: prepared.prepare_target_volume(device, shadow.as_ref()),
            shadow,
            air_shadow,
            visibility,
            occlusion,
            culling_stats: camera_culling::CullingStats::new(
                device,
                prepared.camera_culling(),
                prepared.visibility.as_ref().map_or(
                    crate::visibility::ARGS_BYTES,
                    VisibilityPrepared::args_bytes,
                ),
            ),
            stars,
            depth,
            day_depth,
            drawn: [width, height],
            #[cfg(feature = "creation-counters")]
            border_witness: None,
            width,
            height,
            generation: prepared.key.device_generation,
            capture_layout,
            frame_revision: prepared.frame_revision,
        })
    }
    /// Timed passes in this target's largest compiled graph: the timestamp
    /// pairs one frame on it can write. Read from the graphs compiled with the
    /// target, so it builds nothing.
    pub(crate) fn timed_pass_count(&self) -> u32 {
        [Some(&self.day_graphs), self.night_graphs.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|graphs| [&graphs.scene, &graphs.readback])
            .map(|graph| graph.plan().pipeline_pass_count())
            .max()
            .map_or(0, crate::pass_key::pass_index)
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

    pub(crate) fn capture_source(&self) -> CaptureSource {
        self.capture_source
    }

    pub(crate) fn capture_extent(&self) -> [u32; 2] {
        self.capture_layout.extent
    }

    fn set_drawn_viewport(&self, pass: &mut wgpu::RenderPass<'_>) {
        let [_, _, width, height] = self.forward_rect();
        if self.drawn != self.extent() {
            pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, width, height);
        }
    }

    pub(crate) fn forward_rect(&self) -> [u32; 4] {
        [0, 0, self.drawn[0], self.drawn[1]]
    }

    pub(crate) fn reconstruction_extent(&self) -> Option<[u32; 2]> {
        let output = if let Some(native) = &self.native {
            native.output_view()
        } else {
            &self.taau.as_ref()?.output
        };
        let texture = output.texture();
        Some([texture.width(), texture.height()])
    }

    /// Target extent in pixels.
    pub fn extent(&self) -> [u32; 2] {
        [self.width, self.height]
    }
    pub(crate) fn texture_allocation(&self) -> (u32, u64) {
        let (count, bytes) = self.textures.allocation();
        let extra = u64::from(self.day_depth.is_some());
        let soft_textures =
            4 * (u32::from(self.soft_sun.is_some()) + u32::from(self.ambient.is_some()));
        (
            count + u32::from(self.day_depth.is_some()) + soft_textures,
            bytes
                + extra * u64::from(self.width) * u64::from(self.height) * 4
                + self.sun_penumbra_history_bytes()
                + if self.ambient.is_some() {
                    u64::from(self.width) * u64::from(self.height) * 48
                } else {
                    0
                },
        )
    }

    /// Actual depth, scene, motion and output texture dimensions for diagnostics.
    pub(crate) fn attachment_extents(&self) -> [[u32; 2]; 4] {
        [
            &self.ids.depth,
            &self.ids.scene,
            &self.ids.motion,
            &self.ids.output,
        ]
        .map(|id| {
            let texture = self
                .textures
                .texture(id)
                .expect("compiled target owns each attachment");
            [texture.width(), texture.height()]
        })
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
    /// Allocate one bounded CPU readback slot for this exact generation/grid.
    pub fn readback(&self, device: &wgpu::Device) -> PixelReadback {
        PixelReadback::new(
            device,
            self.generation,
            self.capture_layout.extent,
            self.capture_source,
            self.capture_layout.row_bytes,
            "synthetic pixel readback",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod cloud;
    mod motion;

    mod compute_material;
    pub(super) mod support;

    impl PreparedSynthetic {
        /// Prepare with each slot's mesh row selected: the bucket route.
        pub(in crate::synthetic) async fn prepare_selected(
            capabilities: &DeviceCapabilities,
            queue: &wgpu::Queue,
            scene: &SyntheticScene,
            slots: Vec<mesh_arena::MeshId>,
        ) -> Result<Self, RenderError> {
            let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
            let selection = MeshSelection {
                slots,
                aloft: Vec::new(),
            };
            Self::prepare_with_meshes(
                capabilities,
                queue,
                scene,
                lighting,
                1.0,
                Some(selection),
                None,
            )
            .await
        }
    }

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
            assert_eq!(bytes.direction_to_light, [1.0, 0.0, 0.0, 0.0]);
            assert_eq!(bytes.irradiance[3], 0.0);
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
        let (light, background, clamps) = source.pack().unwrap();
        assert_eq!(clamps, 0);
        assert_eq!(light.irradiance[0], 100_000.0);
        assert_eq!(light.ambient_irradiance[0], 1000.0);
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
