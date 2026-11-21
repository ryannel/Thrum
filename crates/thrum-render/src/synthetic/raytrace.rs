//! Scene-wide acceleration structures for explicitly requested ray queries.

use crate::graph::PassBody;
use crate::pass_key::PassKey;
use std::sync::Mutex;

use super::mesh_arena::{ArenaBuffers, MeshArena, MeshId, MeshRow, VERTEX_BYTES};
use super::{PreparedSynthetic, RenderError, SceneFamily, mesh_rows::LiveRows};
use crate::capabilities::DeviceCapabilities;
use crate::contribution::{RayInstances, SceneContribution};
use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, PassKind, PassScope, ResourceDescriptor,
    ResourceId, ResourceScope,
};
use crate::graph_record::RecordBinding;

mod view;
pub(super) use view::TracedView;
const SHADOW_AND_AMBIENT: u8 = 0b11;
const MAX_IDENTITIES: u32 = 1 << 24;

pub(super) async fn prepare(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    arena: &mut MeshArena,
    slots: u32,
    retained: Option<RetainedRayMeshes>,
) -> Option<SceneStructure> {
    let device = capabilities.site();
    let scopes = [
        wgpu::ErrorFilter::Internal,
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Validation,
    ]
    .map(|filter| device.push_error_scope(filter));
    let structure = (|| {
        let scene = SceneStructure::new(capabilities, slots).ok()?;
        if let Some(RetainedRayMeshes(meshes)) = retained {
            // The candidate belongs to the same host and device. At publication
            // live rows transfer their whole arena, including any later edits.
            if meshes.len() != arena.rows().len() {
                return None;
            }
            arena.enable_ray_demand(meshes);
            return Some(scene);
        }
        let meshes = arena
            .rows()
            .iter()
            .map(|&row| MeshStructure::new(device, row))
            .collect::<Result<_, _>>()
            .ok()?;
        arena.enable_ray_demand(meshes);
        arena.upload(device, queue);
        let buffers = arena.buffers()?;
        let builds: Vec<_> = arena
            .mesh_structures()
            .iter()
            .map(|mesh| mesh.build(buffers))
            .collect();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("prepare resident ray meshes"),
        });
        scene.record_initial_meshes(&mut encoder, &builds);
        let submitted = queue.submit([encoder.finish()]);
        // Apple tile is native. Complete the arena builds before the host can
        // publish this preparation, including a detached replacement device.
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(crate::runtime::GPU_WAIT),
            })
            .ok()?;
        Some(scene)
    })();
    let mut gpu_error = false;
    for scope in scopes.into_iter().rev() {
        gpu_error |= scope.pop().await.is_some();
    }
    if gpu_error || structure.is_none() {
        arena.discard_ray_preparation();
        None
    } else {
        structure
    }
}

pub(super) fn binding(id: usize) -> RecordBinding<SceneFamily> {
    RecordBinding::<SceneFamily>::new(id, PassKind::AccelerationBuild, |frame, _| {
        frame
            .prepared
            .record_ray_structure(frame.encoder, frame.contribution.as_deref())
    })
}

/// A same-device visual candidate's snapshot of the host's prepared meshes.
pub(crate) struct RetainedRayMeshes(Vec<MeshStructure>);

#[derive(Clone)]
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
    slots: u32,
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    probe: Option<RayBuildProbe>,
}

struct SceneInstances {
    tlas: wgpu::Tlas,
    count: usize,
    contributed: Vec<Option<wgpu::TlasInstance>>,
    traced_group: Option<wgpu::BindGroup>,
    light_groups: Option<[wgpu::BindGroup; 2]>,
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    mesh_rows: Vec<u32>,
}

impl SceneStructure {
    fn new(capabilities: &DeviceCapabilities, slots: u32) -> Result<Self, RenderError> {
        let device = capabilities.site();
        if slots > MAX_IDENTITIES || slots > device.limits().max_tlas_instance_count {
            return Err(RenderError::SceneCapacity);
        }
        Ok(Self {
            state: Mutex::new(SceneInstances {
                tlas: create_tlas(device, slots),
                count: 0,
                contributed: Vec::new(),
                traced_group: None,
                light_groups: None,
                #[cfg(all(feature = "creation-counters", target_os = "macos"))]
                mesh_rows: Vec::with_capacity(slots as usize),
            }),
            slots,
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            probe: (capabilities.observe_ray_preparation
                && device.features().contains(wgpu::Features::TIMESTAMP_QUERY))
            .then(|| RayBuildProbe::new(device)),
        })
    }

    fn record_initial_meshes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        builds: &[wgpu::BlasBuildEntry<'_>],
    ) {
        #[cfg(all(feature = "creation-counters", target_os = "macos"))]
        if let Some(probe) = &self.probe {
            for (index, build) in builds.iter().enumerate() {
                probe.encode(encoder, format!("arena mesh {index}"), build);
            }
            return;
        }
        #[cfg(not(all(feature = "creation-counters", target_os = "macos")))]
        let _ = self;
        encoder.build_acceleration_structures(builds, []);
    }

    fn record(
        &self,
        scene: &PreparedSynthetic,
        arena: &MeshArena,
        encoder: &mut wgpu::CommandEncoder,
        contribution: Option<&dyn SceneContribution>,
    ) -> Result<(), RenderError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RenderError::GraphPassMismatch)?;
        let SceneInstances {
            tlas,
            count: previous_count,
            contributed,
            traced_group,
            light_groups,
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            mesh_rows,
        } = &mut *state;
        #[cfg(all(feature = "creation-counters", target_os = "macos"))]
        mesh_rows.clear();
        let entity_count = scene.instance_upload.active_count() as usize;
        if let Some(contribution) = contribution {
            let device = scene.capabilities.device();
            let limit = (device.limits().max_tlas_instance_count as usize - entity_count)
                .min((MAX_IDENTITIES - self.slots) as usize);
            let mut instances = RayInstances {
                entries: contributed,
                next_identity: self.slots,
                limit,
                overflowed: false,
            };
            contribution.ray_instances(&mut instances);
            if instances.overflowed {
                contributed.clear();
                return Err(RenderError::SceneCapacity);
            }
        }
        let count = entity_count + contributed.len();
        if count > tlas.get().len() {
            // The arena's slot capacity is already reserved. Round just the
            // placement addition so a few cooked meshes do not double it.
            let capacity = (self.slots as usize + contributed.len().next_power_of_two())
                .min(scene.capabilities.device().limits().max_tlas_instance_count as usize);
            *tlas = create_tlas(
                scene.capabilities.device(),
                u32::try_from(capacity).map_err(|_| RenderError::SceneCapacity)?,
            );
            *previous_count = 0;
            *traced_group = None;
            *light_groups = scene.create_ray_light_groups(tlas);
        }
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
        let (entities, placements) = active.split_at_mut(entity_count);
        for (entry, (slot, row)) in entities.iter_mut().zip(scene.instance_upload.active_rows()) {
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
        for (entry, placement) in placements.iter_mut().zip(contributed.drain(..)) {
            *entry = placement;
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            mesh_rows.push(u32::MAX);
        }
        retired.fill(None);
        *previous_count = count;
        let buffers = arena.buffers().ok_or(RenderError::GraphPassMismatch)?;
        let builds: Vec<_> = scene
            .row_updates
            .iter()
            .flat_map(LiveRows::pending_ray_builds)
            .map(|mesh| mesh.build(buffers))
            .collect();
        #[cfg(all(feature = "creation-counters", target_os = "macos"))]
        if let Some(probe) = &self.probe {
            for (index, build) in builds.iter().enumerate() {
                probe.encode(encoder, format!("arena mesh {index}"), build);
            }
            encoder.build_acceleration_structures([], std::iter::once(&*tlas));
            return Ok(());
        }
        encoder.build_acceleration_structures(&builds, std::iter::once(&*tlas));
        Ok(())
    }
}

fn create_tlas(device: &wgpu::Device, capacity: u32) -> wgpu::Tlas {
    device.create_tlas(&wgpu::CreateTlasDescriptor {
        label: Some("scene ray structure"),
        max_instances: capacity,
        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_BUILD,
        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
    })
}

fn transform(row: &crate::abi::InstanceRecord, alpha: f32) -> [f32; 12] {
    std::array::from_fn(|component| {
        let axis = component / 4;
        let column = component % 4;
        let current = row.current[column][axis];
        if column == 3 {
            let previous = row.previous[column][axis];
            previous * (1.0 - alpha) + current * alpha
        } else {
            current
        }
    })
}

impl PreparedSynthetic {
    pub(super) fn uses_shadow_rays(&self) -> bool {
        (self.shadow.is_some() && !self.surface_uses_cascades()) || self.lamp_shadows.is_some()
    }

    fn create_ray_light_groups(&self, tlas: &wgpu::Tlas) -> Option<[wgpu::BindGroup; 2]> {
        self.uses_shadow_rays().then(|| {
            super::forward::LightBindings {
                directional: &self.directional_buffer,
                environment_light: &self.environment_light,
                local: &self.local_light_buffer,
                heads: &self.cluster_head_buffer,
                indices: &self.cluster_index_buffer,
                atmosphere: self.atmosphere.as_ref(),
                cloud: self.cloud.as_ref(),
                aerial: None,
            }
            .bind(
                self.capabilities.device(),
                &self.surface.light_layout,
                &self.exposure.samples,
                Some(tlas),
                self.lamp_shadows.as_ref(),
            )
        })
    }

    pub(super) fn ray_light_group(&self, side: usize) -> Result<wgpu::BindGroup, RenderError> {
        let mut state = self
            .ray_structure
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .state
            .lock()
            .map_err(|_| RenderError::GraphPassMismatch)?;
        let SceneInstances {
            light_groups, tlas, ..
        } = &mut *state;
        if light_groups.is_none() {
            *light_groups = self.create_ray_light_groups(tlas);
        }
        light_groups
            .as_ref()
            .map(|groups| groups[side].clone())
            .ok_or(RenderError::GraphPassMismatch)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn retained_ray_meshes(&self) -> Option<RetainedRayMeshes> {
        self.ray_structure.as_ref()?;
        Some(RetainedRayMeshes(
            self.mesh_arena.mesh_structures().to_vec(),
        ))
    }
}

impl super::SyntheticTargetInputs {
    pub(super) fn append_ray_structure(
        &self,
        graph: &mut FrameGraph,
        mesh: &[Access; 2],
        contribution_products: Option<&ResourceId>,
    ) -> Result<Option<ResourceId>, GraphError> {
        if !self.ray_structure {
            return Ok(None);
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
        if let Some(products) = contribution_products {
            accesses.push(Access::new(
                products.clone(),
                0..super::contributed::CONTRIBUTION_TOKEN_BYTES,
                AccessKind::AccelerationInput,
            ));
        }
        accesses.push(Access::new(
            structure.clone(),
            0..1,
            AccessKind::AccelerationBuild,
        ));
        graph.declare(
            PassKey::RayStructureBuild,
            PassBody {
                scope: PassScope::Shared,
                pipeline: None,
                accesses,
                external_side_effect: true,
            },
        );
        Ok(Some(structure))
    }
}

impl PreparedSynthetic {
    pub(super) fn record_ray_structure(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        contribution: Option<&dyn SceneContribution>,
    ) -> Result<(), RenderError> {
        self.ray_structure
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .record(self, &self.mesh_arena, encoder, contribution)
    }
}

mod lamps;
pub(super) use lamps::{LampShadows, structure_layout_entry};

#[cfg(all(feature = "creation-counters", target_os = "macos"))]
mod diagnostics;
#[cfg(all(feature = "creation-counters", target_os = "macos"))]
pub use diagnostics::RayStructureInstance;
#[cfg(all(feature = "creation-counters", target_os = "macos"))]
mod build_probe;
#[cfg(all(feature = "creation-counters", target_os = "macos"))]
pub use build_probe::{RayBuildProbe, RayBuildSample};
