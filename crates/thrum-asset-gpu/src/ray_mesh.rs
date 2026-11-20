//! The ray structure for one resident cooked mesh, built from its draw streams.

use thrum_asset::AssetError;

#[derive(Clone)]
pub(crate) struct RayMesh {
    pub(crate) blas: wgpu::Blas,
    size: wgpu::BlasTriangleGeometrySizeDescriptor,
}

impl RayMesh {
    pub(crate) fn new(
        device: &wgpu::Device,
        vertices: usize,
        index_count: u32,
    ) -> Result<Option<Self>, AssetError> {
        if index_count == 0 {
            return Ok(None);
        }
        let vertex_count = u32::try_from(vertices)
            .map_err(|_| AssetError::Malformed("too many ray mesh vertices"))?;
        if index_count / 3 > device.limits().max_blas_primitive_count {
            return Err(AssetError::Malformed(
                "ray mesh exceeds the device triangle limit",
            ));
        }
        let size = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(index_count),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("cooked mesh ray structure"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size.clone()],
            },
        );
        Ok(Some(Self { blas, size }))
    }

    pub(crate) fn build<'a>(&'a self, mesh: &'a crate::load::GpuMesh) -> wgpu::BlasBuildEntry<'a> {
        wgpu::BlasBuildEntry {
            blas: &self.blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                size: &self.size,
                vertex_buffer: &mesh.positions,
                first_vertex: 0,
                vertex_stride: 12,
                index_buffer: Some(&mesh.indices),
                first_index: Some(0),
                transform_buffer: None,
                transform_buffer_offset: None,
            }]),
        }
    }
}
