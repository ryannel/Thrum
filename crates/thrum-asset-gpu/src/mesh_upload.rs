//! CPU source arrays retained until a mesh's staged upload completes.

use std::sync::Arc;
use thrum_asset::MeshProduct;

/// Ready-to-stage index data and per-meshlet draw ranges for a cooked mesh.
/// The product stays alive for the entire upload.
pub struct MeshUpload {
    mesh: Arc<MeshProduct>,
    indices: Vec<u32>,
    ranges: Vec<[u32; 2]>,
}

impl MeshUpload {
    /// Prepare the meshlet-ordered draw data of a validated cooked product.
    pub fn new(mesh: Arc<MeshProduct>) -> Self {
        let vertices = mesh.meshlet_vertices();
        let triangles = mesh.meshlet_triangles();
        let mut indices = Vec::with_capacity(triangles.len());
        let mut ranges = Vec::with_capacity(mesh.meshlets().len());
        for meshlet in mesh.meshlets() {
            let first = u32::try_from(indices.len()).unwrap_or(u32::MAX);
            let start = meshlet.triangle_offset as usize;
            let local = &triangles[start..start + meshlet.triangle_count as usize * 3];
            indices.extend(
                local
                    .iter()
                    .map(|&index| vertices[meshlet.vertex_offset as usize + usize::from(index)]),
            );
            ranges.push([first, meshlet.triangle_count * 3]);
        }
        Self {
            mesh,
            indices,
            ranges,
        }
    }

    /// Global vertex indices in meshlet draw order.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// First index and count for each meshlet, in table order.
    pub fn ranges(&self) -> &[[u32; 2]] {
        &self.ranges
    }

    /// Heap bytes owned by the prepared index and range arrays, excluding the product mapping.
    pub fn allocated_bytes(&self) -> usize {
        self.indices.capacity() * size_of::<u32>() + self.ranges.capacity() * size_of::<[u32; 2]>()
    }

    /// Cooked product retained by these upload sources.
    pub fn product(&self) -> &Arc<MeshProduct> {
        &self.mesh
    }
}
