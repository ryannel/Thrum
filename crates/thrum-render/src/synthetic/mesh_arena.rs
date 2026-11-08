//! Synthetic mesh arena (G3c M3b, private until root reviews the wiring).
//!
//! One vertex array and one index array hold every synthetic mesh. A
//! [`MeshId`] names a row giving the mesh's index range, base vertex and local
//! bounds. Identity rules:
//! - a `MeshId` never changes once allocated;
//! - growth copies both arrays into larger buffers at the same offsets, so no
//!   row's offsets change;
//! - replacing a mesh keeps its `MeshId`: it is rewritten in place when it
//!   fits its ranges, and otherwise moves to fresh space at the end, leaving
//!   the old ranges unused (counted in [`MeshArena::unused`]);
//! - culling never touches the arena. Mesh rows are keyed by logical slot;
//!   [`physical_meshes`] lays them out in the upload's physical order, which
//!   is the order the camera-visible list indexes.
//!
//! Every mesh is checked before anything is written: a nonzero index count
//! that is a multiple of three, every index below the mesh's vertex count,
//! finite vertex data, and the arena's vertex and index limits. Local bounds
//! are computed from the vertices.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "M3b arena; wiring waits for root's review")
)]

use std::ops::Range;

/// One packed vertex: what `synthetic.wgsl` consumes for a cube corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MeshVertex {
    pub(super) position: [f32; 3],
    pub(super) normal: [f32; 3],
    pub(super) uv: [f32; 2],
    /// Tangent and bitangent sign.
    pub(super) tangent: [f32; 4],
}

/// Bytes per packed vertex in the storage buffer.
pub(super) const VERTEX_BYTES: u64 = 48;
const VERTEX_STRIDE: usize = 48;
const INDEX_BYTES: u64 = 4;

impl MeshVertex {
    fn finite(&self) -> bool {
        self.position
            .iter()
            .chain(&self.normal)
            .chain(&self.uv)
            .chain(&self.tangent)
            .all(|value| value.is_finite())
    }

    fn pack(&self, bytes: &mut Vec<u8>) {
        for value in self
            .position
            .iter()
            .chain(&self.normal)
            .chain(&self.uv)
            .chain(&self.tangent)
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}

/// A stable mesh identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct MeshId(u32);

impl MeshId {
    /// The outward-wound cube, always row 0.
    pub(super) const CUBE: Self = Self(0);
    /// The cube with each triangle's winding reversed, for the orthographic
    /// diagnostic. It shares the cube's vertices (base vertex 0), so a draw
    /// selects it by index-buffer offset alone, as the fixture always has.
    pub(super) const CUBE_REVERSED: Self = Self(1);

    pub(super) fn row(self) -> u32 {
        self.0
    }

    #[cfg(test)]
    pub(super) const fn for_test(row: u32) -> Self {
        Self(row)
    }
}

/// Where a mesh lives in the arena, in draw-argument terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MeshRow {
    pub(super) first_index: u32,
    pub(super) index_count: u32,
    pub(super) base_vertex: u32,
    pub(super) vertex_count: u32,
    /// Local-space minimum and maximum corners.
    pub(super) bounds: [[f32; 3]; 2],
}

impl MeshRow {
    fn vertices(&self) -> Range<usize> {
        self.base_vertex as usize..(self.base_vertex + self.vertex_count) as usize
    }

    fn indices(&self) -> Range<usize> {
        self.first_index as usize..(self.first_index + self.index_count) as usize
    }

    /// The draw's base vertex. Rows are admitted within `i32`, so this only
    /// fails on an arena built with limits past it.
    pub(super) fn draw_base_vertex(&self) -> Result<i32, ArenaError> {
        i32::try_from(self.base_vertex).map_err(|_| ArenaError::Capacity)
    }

    /// This row's byte range in the index buffer.
    pub(super) fn index_bytes(&self) -> Range<u64> {
        let range = self.indices();
        range.start as u64 * INDEX_BYTES..range.end as u64 * INDEX_BYTES
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ArenaError {
    /// No triangles, or an index count that is not a multiple of three.
    NotTriangles,
    /// An index at or past the mesh's own vertex count.
    IndexOutOfRange { index: u32, vertex_count: u32 },
    /// A non-finite vertex component.
    NonFinite,
    /// The arena's vertex or index limit would be exceeded.
    Capacity,
    /// A `MeshId` this arena never allocated.
    UnknownMesh,
    /// The cube rows are fixed: the reversed row shares the cube's vertices.
    Fixed,
}

/// Vertex and index limits, in elements.
#[derive(Clone, Copy, Debug)]
pub(super) struct ArenaLimits {
    pub(super) vertices: u32,
    pub(super) indices: u32,
}

impl ArenaLimits {
    /// The largest arrays one storage binding of `max_binding` bytes holds.
    /// Vertices also stay within a draw's `i32` base vertex.
    pub(super) fn for_binding(max_binding: u64) -> Self {
        let clamp = |bytes: u64| u32::try_from(max_binding / bytes).unwrap_or(u32::MAX);
        Self {
            vertices: clamp(VERTEX_BYTES).min(i32::MAX.unsigned_abs()),
            indices: clamp(INDEX_BYTES),
        }
    }
}

/// GPU copies of the arena. Replaced whole when the arena outgrows them.
pub(super) struct ArenaBuffers {
    pub(super) vertices: wgpu::Buffer,
    pub(super) indices: wgpu::Buffer,
}

/// What an upload did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ArenaUpload {
    /// New buffers were created; bind groups that name them must be rebuilt.
    pub(super) reallocated: bool,
    pub(super) bytes: u64,
}

pub(super) struct MeshArena {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
    rows: Vec<MeshRow>,
    limits: ArenaLimits,
    /// Element ranges written since the last upload.
    dirty_vertices: Vec<Range<usize>>,
    dirty_indices: Vec<Range<usize>>,
    /// Vertices and indices left behind by replacements that moved.
    unused: (u32, u32),
    gpu: Option<ArenaBuffers>,
}

impl MeshArena {
    /// An arena holding the cube (row 0) and its reversed winding (row 1).
    pub(super) fn new(limits: ArenaLimits) -> Result<Self, ArenaError> {
        let mut arena = Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            rows: Vec::new(),
            limits,
            dirty_vertices: Vec::new(),
            dirty_indices: Vec::new(),
            unused: (0, 0),
            gpu: None,
        };
        let (vertices, indices) = cube();
        let cube = arena.insert(&vertices, &indices)?;
        let reversed: Vec<u32> = indices
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[a, b, c]| [a, c, b])
            .collect();
        let reversed = arena.insert_winding(cube, &reversed)?;
        debug_assert_eq!((cube, reversed), (MeshId::CUBE, MeshId::CUBE_REVERSED));
        Ok(arena)
    }

    pub(super) fn row(&self, mesh: MeshId) -> Option<&MeshRow> {
        self.rows.get(mesh.0 as usize)
    }

    pub(super) fn rows(&self) -> &[MeshRow] {
        &self.rows
    }

    /// Vertices and indices no row uses since replacements moved.
    pub(super) fn unused(&self) -> (u32, u32) {
        self.unused
    }

    /// Add a mesh at the end of the arena.
    pub(super) fn insert(
        &mut self,
        vertices: &[MeshVertex],
        indices: &[u32],
    ) -> Result<MeshId, ArenaError> {
        let bounds = check(vertices, indices)?;
        let id = MeshId(u32::try_from(self.rows.len()).map_err(|_| ArenaError::Capacity)?);
        let row = self.append(vertices, indices, bounds)?;
        self.rows.push(row);
        Ok(id)
    }

    /// Add an index range over an existing mesh's vertices.
    fn insert_winding(&mut self, of: MeshId, indices: &[u32]) -> Result<MeshId, ArenaError> {
        let base = *self.row(of).ok_or(ArenaError::UnknownMesh)?;
        let vertices = &self.vertices[base.vertices()];
        let bounds = check(vertices, indices)?;
        let id = MeshId(u32::try_from(self.rows.len()).map_err(|_| ArenaError::Capacity)?);
        let (_, index_count) = counts(vertices, indices)?;
        let first_index = u32::try_from(self.indices.len()).map_err(|_| ArenaError::Capacity)?;
        if first_index
            .checked_add(index_count)
            .is_none_or(|end| end > self.limits.indices)
        {
            return Err(ArenaError::Capacity);
        }
        let row = MeshRow {
            first_index,
            index_count,
            ..base
        };
        self.indices.extend_from_slice(indices);
        self.dirty_indices.push(row.indices());
        self.rows.push(MeshRow { bounds, ..row });
        Ok(id)
    }

    /// Replace a mesh's contents, keeping its `MeshId`.
    pub(super) fn replace(
        &mut self,
        mesh: MeshId,
        vertices: &[MeshVertex],
        indices: &[u32],
    ) -> Result<(), ArenaError> {
        let old = *self.row(mesh).ok_or(ArenaError::UnknownMesh)?;
        if mesh <= MeshId::CUBE_REVERSED {
            return Err(ArenaError::Fixed);
        }
        let bounds = check(vertices, indices)?;
        let (vertex_count, index_count) = counts(vertices, indices)?;
        let row = if vertex_count <= old.vertex_count && index_count <= old.index_count {
            let base = old.base_vertex as usize;
            let first = old.first_index as usize;
            self.vertices[base..base + vertices.len()].copy_from_slice(vertices);
            self.indices[first..first + indices.len()].copy_from_slice(indices);
            self.dirty_vertices.push(base..base + vertices.len());
            self.dirty_indices.push(first..first + indices.len());
            self.unused.0 += old.vertex_count - vertex_count;
            self.unused.1 += old.index_count - index_count;
            MeshRow {
                first_index: old.first_index,
                index_count,
                base_vertex: old.base_vertex,
                vertex_count,
                bounds,
            }
        } else {
            let row = self.append(vertices, indices, bounds)?;
            self.unused.0 += old.vertex_count;
            self.unused.1 += old.index_count;
            row
        };
        self.rows[mesh.0 as usize] = row;
        Ok(())
    }

    fn append(
        &mut self,
        vertices: &[MeshVertex],
        indices: &[u32],
        bounds: [[f32; 3]; 2],
    ) -> Result<MeshRow, ArenaError> {
        let (vertex_count, index_count) = counts(vertices, indices)?;
        let base_vertex = u32::try_from(self.vertices.len()).map_err(|_| ArenaError::Capacity)?;
        let first_index = u32::try_from(self.indices.len()).map_err(|_| ArenaError::Capacity)?;
        if base_vertex
            .checked_add(vertex_count)
            .is_none_or(|end| end > self.limits.vertices)
            || first_index
                .checked_add(index_count)
                .is_none_or(|end| end > self.limits.indices)
        {
            return Err(ArenaError::Capacity);
        }
        let row = MeshRow {
            first_index,
            index_count,
            base_vertex,
            vertex_count,
            bounds,
        };
        self.vertices.extend_from_slice(vertices);
        self.indices.extend_from_slice(indices);
        self.dirty_vertices.push(row.vertices());
        self.dirty_indices.push(row.indices());
        Ok(row)
    }

    pub(super) fn buffers(&self) -> Option<&ArenaBuffers> {
        self.gpu.as_ref()
    }

    /// Write what changed since the last upload, first creating larger
    /// buffers (at least doubling) when the arrays have outgrown them. A new
    /// buffer receives the whole arrays at unchanged offsets.
    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> ArenaUpload {
        let vertex_bytes = self.vertices.len() as u64 * VERTEX_BYTES;
        let index_bytes = self.indices.len() as u64 * INDEX_BYTES;
        let fits = self.gpu.as_ref().is_some_and(|gpu| {
            gpu.vertices.size() >= vertex_bytes && gpu.indices.size() >= index_bytes
        });
        let mut bytes = 0;
        if fits {
            for range in std::mem::take(&mut self.dirty_vertices) {
                bytes += self.write_vertices(queue, range);
            }
            for range in std::mem::take(&mut self.dirty_indices) {
                bytes += self.write_indices(queue, range);
            }
            return ArenaUpload {
                reallocated: false,
                bytes,
            };
        }
        let grown = |current: Option<u64>, needed: u64, limit: u64| {
            needed
                .max(current.map_or(0, |size| size * 2))
                .clamp(4, limit.max(4))
        };
        let current = self.gpu.as_ref();
        let vertex_size = grown(
            current.map(|gpu| gpu.vertices.size()),
            vertex_bytes,
            u64::from(self.limits.vertices) * VERTEX_BYTES,
        );
        let index_size = grown(
            current.map(|gpu| gpu.indices.size()),
            index_bytes,
            u64::from(self.limits.indices) * INDEX_BYTES,
        );
        let usage = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        self.gpu = Some(ArenaBuffers {
            vertices: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic mesh arena vertices"),
                size: vertex_size,
                usage,
                mapped_at_creation: false,
            }),
            indices: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic mesh arena indices"),
                size: index_size,
                usage: usage | wgpu::BufferUsages::INDEX,
                mapped_at_creation: false,
            }),
        });
        self.dirty_vertices.clear();
        self.dirty_indices.clear();
        bytes += self.write_vertices(queue, 0..self.vertices.len());
        bytes += self.write_indices(queue, 0..self.indices.len());
        ArenaUpload {
            reallocated: true,
            bytes,
        }
    }

    fn write_vertices(&self, queue: &wgpu::Queue, range: Range<usize>) -> u64 {
        let Some(gpu) = &self.gpu else { return 0 };
        let mut bytes = Vec::with_capacity(range.len() * VERTEX_STRIDE);
        for vertex in &self.vertices[range.clone()] {
            vertex.pack(&mut bytes);
        }
        queue.write_buffer(&gpu.vertices, range.start as u64 * VERTEX_BYTES, &bytes);
        bytes.len() as u64
    }

    fn write_indices(&self, queue: &wgpu::Queue, range: Range<usize>) -> u64 {
        let Some(gpu) = &self.gpu else { return 0 };
        // Index writes must be 4-byte multiples; u32 indices always are.
        let bytes: Vec<u8> = self.indices[range.clone()]
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect();
        queue.write_buffer(&gpu.indices, range.start as u64 * INDEX_BYTES, &bytes);
        bytes.len() as u64
    }
}

fn counts(vertices: &[MeshVertex], indices: &[u32]) -> Result<(u32, u32), ArenaError> {
    Ok((
        u32::try_from(vertices.len()).map_err(|_| ArenaError::Capacity)?,
        u32::try_from(indices.len()).map_err(|_| ArenaError::Capacity)?,
    ))
}

/// Check a mesh and return its local bounds.
fn check(vertices: &[MeshVertex], indices: &[u32]) -> Result<[[f32; 3]; 2], ArenaError> {
    if indices.is_empty() || !indices.len().is_multiple_of(3) {
        return Err(ArenaError::NotTriangles);
    }
    let (vertex_count, _) = counts(vertices, indices)?;
    if let Some(&index) = indices.iter().find(|&&index| index >= vertex_count) {
        return Err(ArenaError::IndexOutOfRange {
            index,
            vertex_count,
        });
    }
    if !vertices.iter().all(MeshVertex::finite) {
        return Err(ArenaError::NonFinite);
    }
    let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
    // Only referenced vertices bound the mesh.
    for &index in indices {
        let position = vertices[index as usize].position;
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(position[axis]);
            bounds[1][axis] = bounds[1][axis].max(position[axis]);
        }
    }
    Ok(bounds)
}

/// The unit cube `synthetic.wgsl` evaluates from `vertex_index`: four corners
/// per face, and the triangles of `CUBE_INDICES`.
fn cube() -> (Vec<MeshVertex>, Vec<u32>) {
    const NORMALS: [[f32; 3]; 6] = [
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    const TANGENTS: [[f32; 3]; 6] = [
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    const CORNERS: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
    let mut vertices = Vec::with_capacity(24);
    for (n, t) in NORMALS.into_iter().zip(TANGENTS) {
        let b = [
            n[1] * t[2] - n[2] * t[1],
            n[2] * t[0] - n[0] * t[2],
            n[0] * t[1] - n[1] * t[0],
        ];
        for p in CORNERS {
            vertices.push(MeshVertex {
                position: std::array::from_fn(|axis| n[axis] + t[axis] * p[0] + b[axis] * p[1]),
                normal: n,
                uv: [p[0] * 0.5 + 0.5, p[1] * 0.5 + 0.5],
                tangent: [t[0], t[1], t[2], 1.0],
            });
        }
    }
    let indices = super::CUBE_INDICES
        .iter()
        .map(|&index| u32::from(index))
        .collect();
    (vertices, indices)
}

/// Each physical row's mesh row, from the upload's physical order and the
/// per-logical-slot meshes. The camera-visible list holds physical rows, so
/// this is the table a compacted draw reads.
pub(super) fn physical_meshes(physical_to_slot: &[u32], slot_meshes: &[MeshId]) -> Vec<u32> {
    physical_to_slot
        .iter()
        .map(|&slot| slot_meshes[slot as usize].row())
        .collect()
}

#[cfg(test)]
mod tests;
