//! Scene-wide acceleration structures for explicitly requested ray queries.

use std::sync::Mutex;

use super::mesh_arena::{ArenaBuffers, MeshArena, MeshId, MeshRow, VERTEX_BYTES};
use super::{PreparedSynthetic, RenderError, SceneFamily, mesh_rows::LiveRows};
use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, Pass, PassKind, PassScope, ResourceDescriptor,
    ResourceScope,
};
use crate::graph_record::RecordBinding;

pub(super) const PASS: &str = "ray structure build";
const SHADOW_AND_AMBIENT: u8 = 0b11;
const MAX_IDENTITIES: u32 = 1 << 24;

pub(super) fn binding(id: usize) -> RecordBinding<SceneFamily> {
    RecordBinding::<SceneFamily>::new(id, PassKind::AccelerationBuild, |frame, _| {
        frame.prepared.record_ray_structure(frame.encoder)
    })
}

pub(super) struct MeshStructure {
    pub(super) blas: wgpu::Blas,
    size: wgpu::BlasTriangleGeometrySizeDescriptor,
    row: MeshRow,
}

impl MeshStructure {
    pub(super) fn new(device: &wgpu::Device, row: MeshRow) -> Result<Self, RenderError> {
        if row.index_count / 3 > device.limits().max_blas_primitive_count {
            return Err(RenderError::SceneCapacity);
        }
        let size = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: row.vertex_count,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(row.index_count),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("mesh ray structure"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size.clone()],
            },
        );
        Ok(Self { blas, size, row })
    }

    pub(super) fn build<'a>(&'a self, buffers: &'a ArenaBuffers) -> wgpu::BlasBuildEntry<'a> {
        wgpu::BlasBuildEntry {
            blas: &self.blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                size: &self.size,
                vertex_buffer: &buffers.vertices,
                first_vertex: self.row.base_vertex,
                vertex_stride: VERTEX_BYTES,
                index_buffer: Some(&buffers.indices),
                first_index: Some(self.row.first_index),
                transform_buffer: None,
                transform_buffer_offset: None,
            }]),
        }
    }
}

pub(super) struct SceneStructure {
    // Recording snapshots the wgpu instance list into the command buffer.
    // This cursor belongs to that CPU list and survives an abandoned encoder.
    state: Mutex<SceneInstances>,
}

struct SceneInstances {
    tlas: wgpu::Tlas,
    count: usize,
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    mesh_rows: Vec<u32>,
}

impl SceneStructure {
    pub(super) fn new(device: &wgpu::Device, slots: u32) -> Result<Self, RenderError> {
        if slots > MAX_IDENTITIES || slots > device.limits().max_tlas_instance_count {
            return Err(RenderError::SceneCapacity);
        }
        let tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("scene ray structure"),
            max_instances: slots,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_BUILD,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });
        Ok(Self {
            state: Mutex::new(SceneInstances {
                tlas,
                count: 0,
                #[cfg(all(feature = "creation-counters", target_os = "macos"))]
                mesh_rows: Vec::with_capacity(slots as usize),
            }),
        })
    }

    fn record(
        &self,
        scene: &PreparedSynthetic,
        arena: &MeshArena,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RenderError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RenderError::GraphPassMismatch)?;
        let SceneInstances {
            tlas,
            count: previous_count,
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            mesh_rows,
        } = &mut *state;
        #[cfg(all(feature = "creation-counters", target_os = "macos"))]
        mesh_rows.clear();
        let count = scene.instance_upload.active_count() as usize;
        let list = tlas
            .get_mut_slice(0..count.max(*previous_count))
            .ok_or(RenderError::SceneCapacity)?;
        let (active, retired) = list.split_at_mut(count);
        let alpha = scene.history.presentation_alpha();
        let meshes = arena.mesh_structures();
        let cube = if matches!(
            scene
                .pending_view
                .as_ref()
                .unwrap_or(&scene.view_sample)
                .projection(),
            super::Projection::OrthographicFixture
        ) {
            MeshId::CUBE_REVERSED
        } else {
            MeshId::CUBE
        };
        for (entry, (slot, row)) in active.iter_mut().zip(scene.instance_upload.active_rows()) {
            let mesh = scene
                .bucket_route
                .as_ref()
                .map_or(cube, |route| route.mesh_for_slot(slot));
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            mesh_rows.push(mesh.row());
            let blas = &meshes[mesh.row() as usize].blas;
            *entry = Some(wgpu::TlasInstance::new(
                blas,
                transform(row, alpha),
                slot,
                SHADOW_AND_AMBIENT,
            ));
        }
        retired.fill(None);
        *previous_count = count;
        let buffers = arena.buffers().ok_or(RenderError::GraphPassMismatch)?;
        let replacements = scene
            .row_updates
            .iter()
            .flat_map(LiveRows::pending_ray_builds);
        let builds: Vec<_> = arena
            .pending_mesh_builds()
            .chain(replacements)
            .map(|mesh| mesh.build(buffers))
            .collect();
        encoder.build_acceleration_structures(&builds, std::iter::once(&*tlas));
        Ok(())
    }
}

fn transform(row: &[u8; crate::abi::INSTANCE_BYTES], alpha: f32) -> [f32; 12] {
    let words = row.as_chunks::<4>().0;
    let read = |index: usize| f32::from_le_bytes(words[index]);
    std::array::from_fn(|component| {
        let axis = component / 4;
        let column = component % 4;
        let current = read(column * 4 + axis);
        if column == 3 {
            let previous = read(28 + axis);
            previous * (1.0 - alpha) + current * alpha
        } else {
            current
        }
    })
}

impl PreparedSynthetic {
    pub(super) fn append_ray_structure(
        &self,
        graph: &mut FrameGraph,
        mesh: &[Access; 2],
    ) -> Result<(), GraphError> {
        if self.ray_structure.is_none() {
            return Ok(());
        }
        let structure = graph.resource(
            ResourceDescriptor::AccelerationStructure,
            ResourceScope::Imported { initialized: false },
        )?;
        let mut accesses: Vec<_> = mesh
            .iter()
            .map(|read| {
                Access::new(
                    read.resource.clone(),
                    read.range.clone(),
                    AccessKind::AccelerationInput,
                )
            })
            .collect();
        accesses.push(Access::new(structure, 0..1, AccessKind::AccelerationBuild));
        graph.pass(Pass {
            name: PASS.into(),
            scope: PassScope::Shared,
            pipeline: None,
            accesses,
            external_side_effect: true,
        });
        Ok(())
    }

    pub(super) fn record_ray_structure(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RenderError> {
        self.ray_structure
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .record(self, &self.mesh_arena, encoder)
    }
}

#[cfg(all(feature = "creation-counters", target_os = "macos"))]
mod diagnostics;
#[cfg(all(feature = "creation-counters", target_os = "macos"))]
pub use diagnostics::RayStructureInstance;
