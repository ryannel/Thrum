//! Private GPU camera visibility. A compute pass tests each active instance's
//! conservative world bounds against the camera frustum and writes an
//! order-preserving visible list plus one `DrawIndexedIndirectArgs` per
//! mesh/pipeline bucket. Outputs belong to a frame slot's target. Sun casters
//! never read this list. The direct draw, which never reads it, is the oracle.

use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, Pass, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::timing::{GpuTimings, TimingError};

/// Invocations per workgroup; must match `visibility.wgsl`.
const WORKGROUP: u32 = 256;
/// `DrawIndexedIndirectArgs` bytes.
pub(crate) const ARGS_BYTES: u64 = 20;
const MESH_LEN: usize = 48;
const PARAMS_LEN: usize = 16;
const PARAMS_BYTES: u64 = PARAMS_LEN as u64;
const VIEW_BYTES: u64 = 160;
/// Pipeline role inside the synthetic closure.
pub(crate) const ROLE: u32 = 0x5649;
/// Graph pass name.
pub(crate) const PASS: &str = "camera visibility";

/// Stable mesh row: index range and local bounds. Rows never move once
/// assigned, so a bucket's first index and base vertex stay fixed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeshRecord {
    pub(crate) first_index: u32,
    pub(crate) index_count: u32,
    pub(crate) base_vertex: i32,
    pub(crate) bounds_min: [f32; 3],
    pub(crate) bounds_max: [f32; 3],
}

impl MeshRecord {
    fn pack(self) -> [u8; MESH_LEN] {
        let mut bytes = [0; MESH_LEN];
        bytes[0..4].copy_from_slice(&self.first_index.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.index_count.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.base_vertex.to_le_bytes());
        for (axis, (low, high)) in self.bounds_min.iter().zip(self.bounds_max).enumerate() {
            bytes[16 + axis * 4..20 + axis * 4].copy_from_slice(&low.to_le_bytes());
            bytes[32 + axis * 4..36 + axis * 4].copy_from_slice(&high.to_le_bytes());
        }
        bytes
    }
}

/// How the camera's opaque instances are drawn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CullMode {
    /// Every active instance through today's direct draw with no visibility
    /// fetch; the oracle and the unchanged baseline.
    Direct,
    /// GPU frustum compaction and an indexed indirect draw.
    Frustum,
}

/// Device-local pipelines and the mesh table.
pub(crate) struct VisibilityPrepared {
    layout: wgpu::BindGroupLayout,
    count: wgpu::ComputePipeline,
    offsets: wgpu::ComputePipeline,
    scatter: wgpu::ComputePipeline,
    meshes: wgpu::Buffer,
    params: wgpu::Buffer,
    capacity: u32,
    active: u32,
    key: PipelineKey,
}

/// Frame-slot-owned outputs and the bind group that produces them.
pub(crate) struct VisibilityTarget {
    visible: wgpu::Buffer,
    args: wgpu::Buffer,
    group: wgpu::BindGroup,
}

/// Graph identities of one target's visibility outputs.
#[derive(Clone)]
pub(crate) struct VisibilityIds {
    /// Visible slot list, read by the forward vertex stage.
    pub(crate) visible: ResourceId,
    /// Indirect arguments, read by the forward draw.
    pub(crate) args: ResourceId,
}

impl VisibilityPrepared {
    /// Prepare for at most `capacity` instance slots and a stable mesh table.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: PipelineKey,
        capacity: u32,
        meshes: &[MeshRecord],
        active: u32,
    ) -> Self {
        let entry = |binding, ty, dynamic| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let read = wgpu::BufferBindingType::Storage { read_only: true };
        let write = wgpu::BufferBindingType::Storage { read_only: false };
        let uniform = wgpu::BufferBindingType::Uniform;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera visibility"),
            entries: &[
                entry(0, uniform, true),
                entry(1, read, false),
                entry(2, uniform, false),
                entry(3, read, false),
                entry(4, uniform, false),
                entry(5, write, false),
                entry(6, write, false),
                entry(7, write, false),
                entry(8, write, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera visibility"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("camera visibility"),
            source: wgpu::ShaderSource::Wgsl(include_str!("visibility.wgsl").into()),
        });
        let pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("camera visibility"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let mesh_bytes: Vec<u8> = meshes.iter().flat_map(|mesh| mesh.pack()).collect();
        let meshes = storage(device, "visibility mesh table", mesh_bytes.len() as u64);
        queue.write_buffer(&meshes, 0, &mesh_bytes);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visibility parameters"),
            size: PARAMS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let prepared = Self {
            count: pipeline("count"),
            offsets: pipeline("offsets"),
            scatter: pipeline("scatter"),
            layout,
            meshes,
            params,
            capacity,
            active,
            key,
        };
        prepared.write_params(queue);
        prepared
    }

    pub(crate) fn key(&self) -> PipelineKey {
        self.key
    }

    /// Bytes of one target's visible slot list.
    pub(crate) fn visible_bytes(&self) -> u64 {
        u64::from(self.capacity.max(1)) * 4
    }

    fn groups(&self) -> u32 {
        self.active.div_ceil(WORKGROUP).max(1)
    }

    fn write_params(&self, queue: &wgpu::Queue) {
        let mut bytes = [0_u8; PARAMS_LEN];
        bytes[..4].copy_from_slice(&self.active.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.groups().to_le_bytes());
        queue.write_buffer(&self.params, 0, &bytes);
    }

    /// Record a new active prefix. Writes only when it changes; queue order
    /// keeps already submitted frames on their own count.
    pub(crate) fn set_active(&mut self, queue: &wgpu::Queue, active: u32) {
        let active = active.min(self.capacity);
        if active != self.active {
            self.active = active;
            self.write_params(queue);
        }
    }

    /// Allocate one target's outputs.
    pub(crate) fn target(
        &self,
        device: &wgpu::Device,
        view: &wgpu::Buffer,
        instances: &wgpu::Buffer,
        serials: &wgpu::Buffer,
    ) -> VisibilityTarget {
        let max_groups = self.capacity.div_ceil(WORKGROUP).max(1);
        let group_offsets = storage(
            device,
            "visibility group offsets",
            u64::from(max_groups) * 4,
        );
        // One bit per slot, written by `count` and read by `scatter`, so the
        // instance records are read and tested once.
        let masks = storage(
            device,
            "visibility slot masks",
            u64::from(max_groups) * u64::from(WORKGROUP / 32) * 4,
        );
        let visible = storage(
            device,
            "visibility visible slots",
            u64::from(self.capacity.max(1)) * 4,
        );
        let args = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visibility indirect arguments"),
            size: ARGS_BYTES,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera visibility"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: view,
                        offset: 0,
                        size: wgpu::BufferSize::new(VIEW_BYTES),
                    }),
                },
                binding(1, instances),
                binding(2, serials),
                binding(3, &self.meshes),
                binding(4, &self.params),
                binding(5, &group_offsets),
                binding(6, &visible),
                binding(7, &args),
                binding(8, &masks),
            ],
        });
        VisibilityTarget {
            visible,
            args,
            group,
        }
    }

    /// Declare the compute pass and its outputs. `reads` carries the view,
    /// instance and serial accesses from the forward pass's own resources.
    pub(crate) fn append_graph(
        &self,
        graph: &mut FrameGraph,
        mut reads: Vec<Access>,
    ) -> Result<VisibilityIds, GraphError> {
        let visible_bytes = self.visible_bytes();
        let visible = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: visible_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        let args = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: ARGS_BYTES,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        reads.extend([
            Access {
                resource: visible.clone(),
                range: 0..visible_bytes,
                kind: AccessKind::StorageWrite,
                store: true,
            },
            Access {
                resource: args.clone(),
                range: 0..ARGS_BYTES,
                kind: AccessKind::StorageWrite,
                store: true,
            },
        ]);
        graph.pass(Pass {
            name: PASS.into(),
            scope: PassScope::View(1),
            pipeline: Some(self.key),
            accesses: reads,
            external_side_effect: false,
        });
        Ok(VisibilityIds { visible, args })
    }

    /// Encode the three ordered dispatches into the graph's compute pass.
    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &VisibilityTarget,
        view_offset: u32,
        generation: u64,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<bool, TimingError> {
        let Some(mut pass) = compiled.begin_compute(encoder, generation, timings)? else {
            return Ok(false);
        };
        let groups = self.groups();
        pass.set_bind_group(0, &target.group, &[view_offset]);
        pass.set_pipeline(&self.count);
        pass.dispatch_workgroups(groups, 1, 1);
        pass.set_pipeline(&self.offsets);
        pass.dispatch_workgroups(1, 1, 1);
        pass.set_pipeline(&self.scatter);
        pass.dispatch_workgroups(groups, 1, 1);
        Ok(true)
    }
}

impl VisibilityTarget {
    pub(crate) fn visible(&self) -> &wgpu::Buffer {
        &self.visible
    }

    pub(crate) fn args(&self) -> &wgpu::Buffer {
        &self.args
    }
}

fn storage(device: &wgpu::Device, label: &str, bytes: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.max(4),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

fn binding(index: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: index,
        resource: buffer.as_entire_binding(),
    }
}

#[cfg(test)]
mod occlusion_reference;
mod pyramid;
#[cfg(test)]
mod tests;
