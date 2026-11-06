//! Binary layouts shared by the cooker (writer) and this runtime (reader).
//!
//! A Thrum container is a 64-byte [`ContainerHeader`], a table of [`Section`] records, then
//! section payloads, each starting on a [`SECTION_ALIGN`]-byte boundary. Payload element types are
//! plain `repr(C)` values that cast directly from mapped bytes and upload to the GPU unchanged.

use bytemuck::{Pod, Zeroable};

/// Alignment of every container section, matching a cache line and any GPU copy alignment.
pub const SECTION_ALIGN: usize = 64;

/// Container magic for a meshlet mesh product.
pub const MESH_MAGIC: [u8; 8] = *b"THRMMESH";
/// Container magic for a manifest product.
pub const MANIFEST_MAGIC: [u8; 8] = *b"THRMMANI";
/// Current container format version for meshes and manifests.
pub const CONTAINER_FORMAT: u32 = 1;

/// Magic of the 64-byte `HEAD` record that selects the current manifest.
pub const HEAD_MAGIC: [u8; 8] = *b"THRMHEAD";
/// Current `HEAD` record version.
pub const HEAD_FORMAT: u32 = 1;
/// Exact length of a `HEAD` record.
pub const HEAD_LEN: usize = 64;

/// Fixed container header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ContainerHeader {
    /// Kind-specific magic.
    pub magic: [u8; 8],
    /// Format version.
    pub format: u32,
    /// Number of section records following this header.
    pub section_count: u32,
    /// Exact container length in bytes.
    pub total_len: u64,
    /// Zero.
    pub reserved: [u64; 5],
}

/// One section record in the table after the header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Section {
    /// Four-character section tag.
    pub tag: [u8; 4],
    /// Size of one element in bytes.
    pub element_size: u32,
    /// Byte offset from container start; a multiple of [`SECTION_ALIGN`].
    pub offset: u64,
    /// Number of elements.
    pub count: u64,
    /// Zero.
    pub reserved: u64,
}

/// Mesh section tags, in canonical order.
pub mod mesh_tag {
    /// [`super::MeshMeta`], exactly one element.
    pub const META: [u8; 4] = *b"META";
    /// Vertex positions, `[f32; 3]` in metres.
    pub const POSITION: [u8; 4] = *b"POS0";
    /// [`super::VertexAttributes`], one per vertex.
    pub const ATTRIBUTES: [u8; 4] = *b"ATR0";
    /// `u32` triangle list in meshlet order for the portable indexed draw.
    pub const INDICES: [u8; 4] = *b"IDX0";
    /// [`super::Meshlet`] records.
    pub const MESHLETS: [u8; 4] = *b"MLET";
    /// `u32` mesh vertex indices referenced by meshlets.
    pub const MESHLET_VERTICES: [u8; 4] = *b"MLVX";
    /// `u8` meshlet-local triangle corners, three per triangle.
    pub const MESHLET_TRIANGLES: [u8; 4] = *b"MLTR";
    /// [`super::MeshletBounds`], one per meshlet.
    pub const MESHLET_BOUNDS: [u8; 4] = *b"MLBD";
    /// [`super::Submesh`] ranges, one per source primitive.
    pub const SUBMESHES: [u8; 4] = *b"SUBM";
}

/// Mesh-wide facts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MeshMeta {
    /// Object-space bounds minimum.
    pub bounds_min: [f32; 3],
    /// Number of vertices.
    pub vertex_count: u32,
    /// Object-space bounds maximum.
    pub bounds_max: [f32; 3],
    /// Number of `u32` indices.
    pub index_count: u32,
    /// Number of meshlets.
    pub meshlet_count: u32,
    /// Largest meshlet vertex count the cooker allowed.
    pub max_meshlet_vertices: u32,
    /// Largest meshlet triangle count the cooker allowed.
    pub max_meshlet_triangles: u32,
    /// Number of submeshes.
    pub submesh_count: u32,
}

/// Shading attributes, stored apart from positions so depth and shadow passes fetch 12 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct VertexAttributes {
    /// Unit normal, octahedral-encoded as signed normalized 16-bit.
    pub normal_oct: [i16; 2],
    /// Texture coordinate.
    pub uv: [f32; 2],
}

/// One meshlet: a small vertex set and the local triangles that use it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct Meshlet {
    /// First entry in the meshlet-vertex section.
    pub vertex_offset: u32,
    /// First byte in the meshlet-triangle section.
    pub triangle_offset: u32,
    /// Number of meshlet vertices.
    pub vertex_count: u32,
    /// Number of triangles.
    pub triangle_count: u32,
}

/// Culling bounds for one meshlet: bounding sphere plus normal cone.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MeshletBounds {
    /// Sphere centre.
    pub center: [f32; 3],
    /// Sphere radius.
    pub radius: f32,
    /// Cone apex for perspective back-face cone culling.
    pub cone_apex: [f32; 3],
    /// Cosine-based cutoff; a meshlet is back-facing when `dot(normalize(apex - eye), axis) >= cutoff`.
    pub cone_cutoff: f32,
    /// Cone axis.
    pub cone_axis: [f32; 3],
    /// Zero.
    pub reserved: f32,
}

/// One source primitive: a contiguous index and meshlet range.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct Submesh {
    /// First index.
    pub index_offset: u32,
    /// Number of indices.
    pub index_count: u32,
    /// First meshlet.
    pub meshlet_offset: u32,
    /// Number of meshlets.
    pub meshlet_count: u32,
}

/// Manifest section tags.
pub mod manifest_tag {
    /// [`super::ManifestMeta`], exactly one element.
    pub const META: [u8; 4] = *b"META";
    /// [`super::ManifestRecord`] entries sorted by `(name_key, profile)`.
    pub const ENTRIES: [u8; 4] = *b"ENTR";
    /// UTF-8 logical names referenced by entries.
    pub const STRINGS: [u8; 4] = *b"STRS";
}

/// Manifest-wide facts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ManifestMeta {
    /// Publication generation; strictly increases per store.
    pub generation: u64,
    /// Number of entries.
    pub entry_count: u32,
    /// Zero.
    pub reserved: [u32; 13],
}

/// One `(logical name, profile) -> product` binding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ManifestRecord {
    /// First eight bytes (little-endian) of BLAKE3 of the logical name.
    pub name_key: u64,
    /// Content hash of the product bytes.
    pub product: [u8; 32],
    /// Product length in bytes.
    pub byte_len: u64,
    /// Name offset in the string section.
    pub name_offset: u32,
    /// Name length in bytes.
    pub name_len: u16,
    /// [`crate::ProductKind`] code.
    pub kind: u8,
    /// [`crate::Profile`] code.
    pub profile: u8,
    /// Zero.
    pub reserved: [u8; 8],
}
