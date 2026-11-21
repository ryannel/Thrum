//! Whole host/shader records. Layouts are held against reflected WGSL in `wgsl_records`.
use bytemuck::{Pod, Zeroable};
use thrum_render::{RecordLayout, RecordMember, RecordScalar};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Params {
    pub time: [u32; 4],
    pub physical: [f32; 4],
    pub lengths: [f32; 4],
    pub levels: [[f32; 4]; crate::LEVELS as usize],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct Bin {
    pub physical: [f32; 4],
    pub clock: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Long {
    pub physical: [f32; 4],
    pub rate: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Query {
    pub point: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Field {
    pub displacement: [f32; 4],
    pub slope: [f32; 4],
    pub velocity: [f32; 4],
}

/// The scalar kind a host record member stores, for [`layouts`].
trait Kind {
    const KIND: RecordScalar;
}
impl Kind for f32 {
    const KIND: RecordScalar = RecordScalar::Float;
}
impl Kind for u32 {
    const KIND: RecordScalar = RecordScalar::Uint;
}
impl<T: Kind, const N: usize> Kind for [T; N] {
    const KIND: RecordScalar = T::KIND;
}

fn member<T, F: Kind>(name: &'static str, offset: usize, _: fn(&T) -> &F) -> RecordMember {
    RecordMember {
        name,
        offset,
        size: size_of::<F>(),
        kind: Some(F::KIND),
    }
}

/// Host layouts for shader reflection tools, in the renderer's record form, generated from the actual Rust
/// members. New bound WGSL structs must have a row here.
pub fn layouts() -> Vec<RecordLayout> {
    macro_rules! record {
        ($ty:ident, $($member:ident),+ $(,)?) => {
            RecordLayout {
                name: stringify!($ty),
                size: size_of::<$ty>(),
                members: vec![$(member(
                    stringify!($member),
                    std::mem::offset_of!($ty, $member),
                    |record: &$ty| &record.$member,
                )),+],
            }
        };
    }
    vec![
        record!(Params, time, physical, lengths, levels),
        record!(Bin, physical, clock),
        record!(Long, physical, rate),
        record!(Query, point),
        record!(Field, displacement, slope, velocity),
    ]
}
