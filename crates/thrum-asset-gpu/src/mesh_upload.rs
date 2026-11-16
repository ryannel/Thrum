//! CPU source arrays retained until a mesh's staged upload completes.

use std::sync::Arc;
use thrum_asset::{AssetError, MeshProduct};

/// Ready-to-stage index data and per-meshlet draw ranges for a cooked mesh.
/// The product stays alive for the entire upload.
pub struct MeshUpload {
    mesh: Arc<MeshProduct>,
    ranges: Vec<[u32; 2]>,
}

impl MeshUpload {
    /// Prepare draw ranges without expanding or scanning the index section.
    /// # Errors
    /// Refuses a meshlet table whose triangle counts do not cover the index section.
    pub fn new(mesh: Arc<MeshProduct>) -> Result<Self, AssetError> {
        let mut ranges = Vec::with_capacity(mesh.meshlets().len());
        let mut first = 0_u32;
        for meshlet in mesh.meshlets() {
            let count = meshlet
                .triangle_count
                .checked_mul(3)
                .ok_or(AssetError::Malformed("meshlet index count overflow"))?;
            ranges.push([first, count]);
            first = first
                .checked_add(count)
                .ok_or(AssetError::Malformed("meshlet index range overflow"))?;
        }
        if first as usize != mesh.indices().len() {
            return Err(AssetError::Malformed("meshlet index coverage"));
        }
        Ok(Self { mesh, ranges })
    }

    /// Global vertex indices in meshlet draw order.
    pub fn indices(&self) -> &[u32] {
        self.mesh.indices()
    }

    /// First index and count for each meshlet, in table order.
    pub fn ranges(&self) -> &[[u32; 2]] {
        &self.ranges
    }

    /// Heap bytes owned by the prepared index and range arrays, excluding the product mapping.
    pub fn allocated_bytes(&self) -> usize {
        self.ranges.capacity() * size_of::<[u32; 2]>()
    }

    /// Cooked product retained by these upload sources.
    pub fn product(&self) -> &Arc<MeshProduct> {
        &self.mesh
    }
}
