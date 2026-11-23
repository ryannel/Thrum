//! Host records for the renderer's WGSL buffers.

use bytemuck::{Pod, Zeroable};

type Matrix = [[f32; 4]; 4];

macro_rules! records {
    ($($record:ident { $($field:ident: $ty:ty),+ $(,)? })+) => {
        $(
            #[repr(C)]
            #[derive(Clone, Copy, Debug, Pod, Zeroable)]
            pub(crate) struct $record {
                $(pub(crate) $field: $ty,)+
            }
        )+

        fn internal_layouts() -> Vec<RecordLayout> {
            vec![$(layout!($record, stringify!($record), $($field),+)),+]
        }
    };
}

/// A renderer buffer record as reflected from the Rust type.
#[derive(Debug)]
pub struct RecordLayout {
    /// WGSL type name.
    pub name: &'static str,
    /// Total size in bytes.
    pub size: usize,
    /// WGSL members in declaration order.
    pub members: Vec<RecordMember>,
}

/// A Rust member's location and scalar representation.
#[derive(Debug)]
pub struct RecordMember {
    /// WGSL member name.
    pub name: &'static str,
    /// Byte offset from the start of the record.
    pub offset: usize,
    /// Size in bytes.
    pub size: usize,
    /// Scalar kind, or `None` for members reflecting a nested record.
    pub kind: Option<RecordScalar>,
}

/// A primitive scalar in a host buffer record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordScalar {
    /// Unsigned integer.
    Uint,
    /// Signed integer.
    Sint,
    /// Floating point.
    Float,
}

trait RecordField {
    const KIND: Option<RecordScalar>;
}

macro_rules! scalar {
    ($type:ty, $kind:ident) => {
        impl RecordField for $type {
            const KIND: Option<RecordScalar> = Some(RecordScalar::$kind);
        }
    };
}
scalar!(u32, Uint);
scalar!(i32, Sint);
scalar!(f32, Float);

impl RecordField for Segment {
    const KIND: Option<RecordScalar> = None;
}
impl<T: RecordField, const N: usize> RecordField for [T; N] {
    const KIND: Option<RecordScalar> = T::KIND;
}

fn member<T, F: RecordField>(name: &'static str, offset: usize, _: fn(&T) -> &F) -> RecordMember {
    RecordMember {
        name,
        offset,
        size: size_of::<F>(),
        kind: F::KIND,
    }
}

fn pod_layout<T: Pod>(name: &'static str, members: Vec<RecordMember>) -> RecordLayout {
    RecordLayout {
        name,
        size: size_of::<T>(),
        members,
    }
}

macro_rules! layout {
    ($ty:ty, $name:expr, $($field:ident $(as $alias:literal)?),+ $(,)?) => {
        pod_layout::<$ty>($name, vec![$(
            member(layout!(@name $field $($alias)?), std::mem::offset_of!($ty, $field), |r: &$ty| &r.$field)
        ),+])
    };
    (@name $field:ident $alias:literal) => { $alias };
    (@name $field:ident) => { stringify!($field) };
}

records! {
    ViewRecord {
        view_proj: Matrix,
        previous_view_proj: Matrix,
        camera_xyz_time: [f32; 4],
        pre_exposure_inv_exposure_jitter: [f32; 4],
        unjittered_view_proj: Matrix,
        mip_bias: f32,
        jitter_phase: u32,
        padding: [u32; 2],
    }
    ViewAuxRecord {
        inverse_view_proj: Matrix,
        viewport_inv_viewport: [f32; 4],
        near_far: [f32; 4],
        time: [f32; 4],
        flags: [u32; 4],
        subrectangle: [f32; 4],
    }
    LocalSource {
        position_radius: [f32; 4],
        candela: [f32; 4],
    }
    ClusterHead { offset: u32, count: u32 }
    Bounds { lo: [u32; 4], hi: [u32; 4] }
    Serials {
        previous_lo: u32,
        previous_hi: u32,
        current_lo: u32,
        current_hi: u32,
        presentation_alpha: f32,
        padding0: u32,
        padding1: u32,
        padding2: u32,
    }
    HistoryRecord { current: Matrix, identity: [u32; 4] }
    MeshVertex { a: [f32; 4], b: [f32; 4], c: [f32; 4] }
    ShadowCascades {
        light_view_proj: [Matrix; 2],
        split_overlap: [f32; 4],
        bias: [f32; 4],
        penumbra: [f32; 4],
        height_support: [f32; 4],
        height_view_proj: Matrix,
        horizon: [f32; 4],
        near_partition: [f32; 4],
    }
    ShadowView { light_view_proj: Matrix, clip_upstream: [u32; 4] }
    MeshRecord {
        first_index: u32,
        index_count: u32,
        base_vertex: i32,
        padding: u32,
        bounds_min: [f32; 4],
        bounds_max: [f32; 4],
    }
    VisibilityParams { active_count: u32, groups: u32, mesh: u32, phase: u32 }
    Segment { first_group: u32, groups: u32, mesh: u32, padding: u32 }
    Buckets {
        count: u32,
        mask_base: u32,
        padding0: u32,
        padding1: u32,
        segments: [Segment; crate::synthetic::MAX_BUCKETS],
    }
    Atmosphere {
        inverse_view_projection: Matrix,
        camera_altitude: [f32; 4],
        sun_lux: [f32; 4],
        colour_pre_exposure: [f32; 4],
        sky_mapping: [f32; 4],
    }
    AerosolUniform {
        extinction: [[f32; 4]; 2],
        scattering: [[f32; 4]; 2],
        surface_extinction: [f32; 4],
    }
    VolumeHistory {
        previous_view_projection: Matrix,
        previous_eye_valid: [f32; 4],
        current_view_forward_phase: [f32; 4],
    }
    SkyMotion { camera_delta: Matrix, current: Matrix, extent: [f32; 4], solar_delta: Matrix }
    StarView {
        clip_from_catalogue: Matrix,
        extent_and_tan_half_fov: [f32; 4],
        catalogue_zenith_and_strength: [f32; 4],
        delta_clip_from_catalogue: Matrix,
        jitter: [f32; 4],
    }
    CloudField { region: [f32; 4], plane_drift: [f32; 4], validity: [f32; 4] }
    Policy {
        inverse_compensation_min_max: [f32; 4],
        mode: u32,
        pre_exposure: f32,
        atmosphere: u32,
        display_selectors: u32,
    }
    ExposureSample { values: [f32; 4], metadata: [u32; 4] }
    ReconstructionParams { inverse: Matrix, previous_view: Matrix, view: Matrix, state: [f32; 4], jitter: [f32; 4], previous_extent: [f32; 4] }
    LampShadowQuality { ray_limit: u32 }
    SpriteView {
        clip_per_world: [f32; 2],
        centre_high: [f32; 2],
        centre_low: [f32; 2],
        alpha: f32,
        pixels_per_unit: f32,
    }
    SpriteRow {
        current: [f32; 4],
        previous: [f32; 4],
        scales: [f32; 4],
        rotation: f32,
        rotation_delta: f32,
        frame: u32,
        sheet: u32,
        tint: [f32; 4],
        flash: [f32; 4],
        additive: f32,
        padding0: u32,
        padding1: u32,
        padding2: u32,
    }
    SpriteFrameRecord {
        origin: [u32; 2], size: [u32; 2], page: u32, kind: u32, tile: u32, padding: u32,
    }
}

/// Host layouts for shader reflection tooling.
pub fn layouts() -> Vec<RecordLayout> {
    let mut layouts = internal_layouts();
    layouts.extend([
        layout!(crate::abi::MaterialRecord, "MaterialRecord", base_rgba as "base", emissive_cutoff, surface, flags, texture_layers),
        layout!(crate::abi::InstanceRecord, "InstanceRecord", current, previous, normal, ids),
        layout!(crate::light::DirectionalLight, "DirectionalLight", direction_to_light, irradiance, half_vector_fresnel, ambient_irradiance),
        layout!(crate::light::EnvironmentLight, "EnvironmentLight", horizontal_irradiance, sky_diffuse),
        layout!(crate::contribution::SecondaryViewRecord, "SecondaryViewRecord", view_proj, inverse_view_proj, unjittered_view_proj, inverse_unjittered_view_proj, camera, extent, clip_plane, options),
        layout!(crate::atmosphere::GpuSunMapping, "GpuSunMapping", region, plane_drift, path_border, far, near, middle),
    ]);
    layouts
}
