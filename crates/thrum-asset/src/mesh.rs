use crate::container::Container;
use crate::layout::{
    MESH_MAGIC, MeshMeta, Meshlet, MeshletBounds, Submesh, VertexAttributes, mesh_tag,
};
use crate::{AssetError, ProductBytes};

/// Zero-copy view of a cooked meshlet mesh.
///
/// [`MeshProduct::open`] checks structure and section counts in time independent of mesh size;
/// that is all a draw needs, because the GPU reads indices with robust buffer access.
/// [`MeshProduct::validate_references`] adds the linear index/meshlet reference scan used by the
/// dev loop and tests.
#[derive(Debug)]
pub struct MeshProduct {
    bytes: ProductBytes,
    meta: MeshMeta,
}

impl MeshProduct {
    /// Validate container structure and counts.
    ///
    /// # Errors
    /// Refuses a malformed container or inconsistent section counts.
    pub fn open(bytes: ProductBytes) -> Result<Self, AssetError> {
        let meta = {
            let view = View::parse(&bytes)?;
            *view.meta
        };
        Ok(Self { bytes, meta })
    }

    /// Mesh-wide facts.
    pub fn meta(&self) -> &MeshMeta {
        &self.meta
    }

    /// Underlying product bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Vertex positions.
    pub fn positions(&self) -> &[[f32; 3]] {
        self.view().positions
    }

    /// Shading attributes.
    pub fn attributes(&self) -> &[VertexAttributes] {
        self.view().attributes
    }

    /// Triangle list in meshlet order.
    pub fn indices(&self) -> &[u32] {
        self.view().indices
    }

    /// Meshlet records.
    pub fn meshlets(&self) -> &[Meshlet] {
        self.view().meshlets
    }

    /// Mesh vertex indices referenced by meshlets.
    pub fn meshlet_vertices(&self) -> &[u32] {
        self.view().meshlet_vertices
    }

    /// Meshlet-local triangle corners.
    pub fn meshlet_triangles(&self) -> &[u8] {
        self.view().meshlet_triangles
    }

    /// Meshlet culling bounds.
    pub fn meshlet_bounds(&self) -> &[MeshletBounds] {
        self.view().meshlet_bounds
    }

    /// Submesh ranges.
    pub fn submeshes(&self) -> &[Submesh] {
        self.view().submeshes
    }

    /// Linear check that every index and meshlet reference is in range.
    ///
    /// # Errors
    /// Refuses the first out-of-range reference.
    pub fn validate_references(&self) -> Result<(), AssetError> {
        let view = self.view();
        let vertices = view.positions.len();
        if view.indices.iter().any(|&index| index as usize >= vertices) {
            return Err(AssetError::Malformed("mesh index out of range"));
        }
        if view
            .meshlet_vertices
            .iter()
            .any(|&index| index as usize >= vertices)
        {
            return Err(AssetError::Malformed("meshlet vertex out of range"));
        }
        for meshlet in view.meshlets {
            let vertex_end = meshlet.vertex_offset as usize + meshlet.vertex_count as usize;
            let triangle_end =
                meshlet.triangle_offset as usize + 3 * meshlet.triangle_count as usize;
            let local = view
                .meshlet_triangles
                .get(meshlet.triangle_offset as usize..triangle_end)
                .ok_or(AssetError::Malformed("meshlet triangles out of range"))?;
            if vertex_end > view.meshlet_vertices.len()
                || meshlet.vertex_count > self.meta.max_meshlet_vertices
                || meshlet.triangle_count > self.meta.max_meshlet_triangles
                || local
                    .iter()
                    .any(|&corner| u32::from(corner) >= meshlet.vertex_count)
            {
                return Err(AssetError::Malformed("meshlet reference out of range"));
            }
        }
        for submesh in view.submeshes {
            let index_end = submesh.index_offset as usize + submesh.index_count as usize;
            let meshlet_end = submesh.meshlet_offset as usize + submesh.meshlet_count as usize;
            if index_end > view.indices.len() || meshlet_end > view.meshlets.len() {
                return Err(AssetError::Malformed("submesh range"));
            }
        }
        Ok(())
    }

    fn view(&self) -> View<'_> {
        // `open` validated these immutable bytes; parsing again is constant-time.
        View::parse(&self.bytes).unwrap_or_else(|_| View::empty())
    }
}

struct View<'a> {
    meta: &'a MeshMeta,
    positions: &'a [[f32; 3]],
    attributes: &'a [VertexAttributes],
    indices: &'a [u32],
    meshlets: &'a [Meshlet],
    meshlet_vertices: &'a [u32],
    meshlet_triangles: &'a [u8],
    meshlet_bounds: &'a [MeshletBounds],
    submeshes: &'a [Submesh],
}

static EMPTY_META: MeshMeta = MeshMeta {
    bounds_min: [0.0; 3],
    vertex_count: 0,
    bounds_max: [0.0; 3],
    index_count: 0,
    meshlet_count: 0,
    max_meshlet_vertices: 0,
    max_meshlet_triangles: 0,
    submesh_count: 0,
};

impl<'a> View<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, AssetError> {
        let container = Container::parse(bytes, MESH_MAGIC)?;
        let view = Self {
            meta: container.one(mesh_tag::META)?,
            positions: container.slice(mesh_tag::POSITION)?,
            attributes: container.slice(mesh_tag::ATTRIBUTES)?,
            indices: container.slice(mesh_tag::INDICES)?,
            meshlets: container.slice(mesh_tag::MESHLETS)?,
            meshlet_vertices: container.slice(mesh_tag::MESHLET_VERTICES)?,
            meshlet_triangles: container.slice(mesh_tag::MESHLET_TRIANGLES)?,
            meshlet_bounds: container.slice(mesh_tag::MESHLET_BOUNDS)?,
            submeshes: container.slice(mesh_tag::SUBMESHES)?,
        };
        let meta = view.meta;
        let consistent = view.positions.len() == meta.vertex_count as usize
            && view.attributes.len() == view.positions.len()
            && view.indices.len() == meta.index_count as usize
            && view.indices.len().is_multiple_of(3)
            && view.meshlets.len() == meta.meshlet_count as usize
            && view.meshlet_bounds.len() == view.meshlets.len()
            && view.submeshes.len() == meta.submesh_count as usize
            && meta.max_meshlet_vertices <= 256
            && meta.max_meshlet_triangles <= 512;
        if consistent {
            Ok(view)
        } else {
            Err(AssetError::Malformed("mesh section counts"))
        }
    }

    fn empty() -> Self {
        Self {
            meta: &EMPTY_META,
            positions: &[],
            attributes: &[],
            indices: &[],
            meshlets: &[],
            meshlet_vertices: &[],
            meshlet_triangles: &[],
            meshlet_bounds: &[],
            submeshes: &[],
        }
    }
}
