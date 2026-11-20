//! Whole host/shader records. Layouts are held against reflected WGSL in `wgsl_records`.
use bytemuck::{Pod, Zeroable};

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

/// Reflection tooling's view of the host record, generated from the actual Rust members.
#[derive(Debug)]
pub struct RecordLayout {
    /// WGSL type name.
    pub name: &'static str,
    /// Rust record size in bytes.
    pub size: usize,
    /// WGSL member names and Rust byte offsets, in declaration order.
    pub members: Vec<(&'static str, usize)>,
}

/// Host layouts for shader reflection tools. New bound WGSL structs must have a row here.
pub fn layouts() -> Vec<RecordLayout> {
    macro_rules! record {
        ($ty:ident, $($member:ident),+ $(,)?) => {
            RecordLayout {
                name: stringify!($ty),
                size: size_of::<$ty>(),
                members: vec![$((stringify!($member), std::mem::offset_of!($ty, $member))),+],
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
