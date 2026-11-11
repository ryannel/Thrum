//! Sun caster culling (G3c M3c, experimental): for scenes on the bucket
//! route, each Sun pass draws only the members whose conservative world box
//! can reach the volume that pass rasterizes, from a compaction against the
//! pass's own light matrix. Membership never comes from the camera's list,
//! so off-screen casters inside a light volume still draw. The kernels are
//! the camera's member-indexed compaction with the caster test in place of
//! the frustum test; each pass gets its own list and argument records in
//! the bucket route's segment layout. Unculled casters are the oracle.

use super::{MemberInputs, MemberShape, MeshRecord, WORDS, WORKGROUP, binding, storage};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, ResourceDescriptor, ResourceId,
    ResourceScope,
};
use crate::timing::{GpuTimings, TimingError};

/// Graph pass name of the caster compaction, which runs every Sun pass's list.
pub(crate) const PASS: &str = "synthetic Sun caster cull";
/// Bytes of the caster view each Sun pass draws with (`ShadowView`).
const CASTER_VIEW_BYTES: u64 = 80;

/// The prepared inputs every Sun pass's list reads.
pub(crate) struct CasterInputs<'a> {
    pub(crate) instances: &'a wgpu::Buffer,
    pub(crate) serials: &'a wgpu::Buffer,
    pub(crate) members: MemberInputs<'a>,
    /// Each Sun pass's caster view, in pass order.
    pub(crate) views: Vec<&'a wgpu::Buffer>,
}

/// One Sun pass's culled list: members in the route's segment layout and one
/// indexed indirect record per bucket.
pub(crate) struct CasterList {
    visible: wgpu::Buffer,
    args: wgpu::Buffer,
    group: wgpu::BindGroup,
}

pub(crate) struct CasterCulling {
    count: wgpu::ComputePipeline,
    offsets: wgpu::ComputePipeline,
    scatter: wgpu::ComputePipeline,
    lists: Vec<CasterList>,
    shape: MemberShape,
    /// Workgroups in the route's current member list.
    groups: u32,
}

/// Graph identities of each Sun pass's list and records.
pub(crate) struct CasterIds {
    pub(crate) lists: Vec<(ResourceId, ResourceId)>,
}

impl CasterCulling {
    /// Prepare one list per caster view over the route's member list.
    pub(crate) fn new(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        meshes: &[MeshRecord],
        shape: MemberShape,
        inputs: &CasterInputs<'_>,
    ) -> Self {
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let read = wgpu::BufferBindingType::Storage { read_only: true };
        let write = wgpu::BufferBindingType::Storage { read_only: false };
        let uniform = wgpu::BufferBindingType::Uniform;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sun caster cull"),
            entries: &[
                entry(1, read),
                entry(2, uniform),
                entry(3, read),
                entry(5, write),
                entry(6, write),
                entry(7, write),
                entry(12, read),
                entry(13, uniform),
                entry(14, uniform),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sun caster cull"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sun caster cull"),
            source: wgpu::ShaderSource::Wgsl(
                [
                    include_str!("../visibility.wgsl"),
                    super::pyramid::OCCLUSION_WGSL,
                    include_str!("casters.wgsl"),
                ]
                .concat()
                .into(),
            ),
        });
        let pipeline = |entry: &str| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("Sun caster cull"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let mesh_bytes: Vec<u8> = meshes.iter().flat_map(|mesh| mesh.pack()).collect();
        let mesh_table = storage(device, "Sun caster mesh table", mesh_bytes.len() as u64);
        queue.write_buffer(&mesh_table, 0, &mesh_bytes);
        let groups = shape.worst_members.div_ceil(WORKGROUP).max(1);
        let lists = inputs
            .views
            .iter()
            .map(|view| {
                // Per-group counts, then the slot masks from `mask_base`.
                let group_offsets = storage(
                    device,
                    "Sun caster group offsets",
                    u64::from(groups) * u64::from(1 + WORDS) * 4,
                );
                let visible = storage(
                    device,
                    "Sun caster list",
                    u64::from(shape.worst_members) * 4 + shape.segment_bytes,
                );
                let args = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Sun caster indirect arguments"),
                    size: u64::from(shape.buckets.max(1)) * super::ARGS_BYTES,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Sun caster cull"),
                    layout: &layout,
                    entries: &[
                        binding(1, inputs.instances),
                        binding(2, inputs.serials),
                        binding(3, &mesh_table),
                        binding(5, &group_offsets),
                        binding(6, &visible),
                        binding(7, &args),
                        binding(12, inputs.members.members),
                        binding(13, inputs.members.table),
                        wgpu::BindGroupEntry {
                            binding: 14,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: view,
                                offset: 0,
                                size: wgpu::BufferSize::new(CASTER_VIEW_BYTES),
                            }),
                        },
                    ],
                });
                CasterList {
                    visible,
                    args,
                    group,
                }
            })
            .collect();
        Self {
            count: pipeline("count_casters"),
            offsets: pipeline("offsets_members"),
            scatter: pipeline("scatter_members"),
            lists,
            shape,
            groups: 0,
        }
    }

    /// Record the route's current member-list workgroup count.
    pub(crate) fn set_member_groups(&mut self, groups: u32) {
        self.groups = groups;
    }

    /// Bytes each draw binds at its bucket's offset.
    pub(crate) fn segment_bytes(&self) -> u64 {
        self.shape.segment_bytes
    }

    fn visible_bytes(&self) -> u64 {
        u64::from(self.shape.worst_members) * 4 + self.shape.segment_bytes
    }

    fn args_bytes(&self) -> u64 {
        u64::from(self.shape.buckets.max(1)) * super::ARGS_BYTES
    }

    /// Each Sun pass's list and records, in pass order.
    pub(crate) fn lists(&self) -> impl Iterator<Item = (&wgpu::Buffer, &wgpu::Buffer)> {
        self.lists.iter().map(|list| (&list.visible, &list.args))
    }

    /// Declare every list's outputs into `accesses`, the compaction pass's
    /// own, and return their identities for the Sun passes' reads.
    pub(crate) fn append_outputs(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
    ) -> Result<CasterIds, GraphError> {
        let mut lists = Vec::with_capacity(self.lists.len());
        for _ in &self.lists {
            let visible = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: self.visible_bytes(),
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::Imported { initialized: false },
            )?;
            let args = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: self.args_bytes(),
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_SRC,
                },
                ResourceScope::Imported { initialized: false },
            )?;
            accesses.extend([
                Access {
                    view: None,
                    resource: visible.clone(),
                    range: 0..self.visible_bytes(),
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
                Access {
                    view: None,
                    resource: args.clone(),
                    range: 0..self.args_bytes(),
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
            ]);
            lists.push((visible, args));
        }
        Ok(CasterIds { lists })
    }

    /// One Sun pass's reads of its list (storage) and records (indirect).
    pub(crate) fn list_reads(&self, (visible, args): &(ResourceId, ResourceId)) -> [Access; 2] {
        [
            Access {
                view: None,
                resource: visible.clone(),
                range: 0..self.visible_bytes(),
                kind: AccessKind::StorageRead,
                store: true,
            },
            Access {
                view: None,
                resource: args.clone(),
                range: 0..self.args_bytes(),
                kind: AccessKind::IndirectRead,
                store: true,
            },
        ]
    }

    /// Encode every list's three ordered dispatches into the graph's compute
    /// pass. The caster views must already hold this recording's fit.
    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<bool, TimingError> {
        let Some(mut pass) = compiled.begin_compute(encoder, generation, timings)? else {
            return Ok(false);
        };
        let groups = self.groups.max(1);
        for list in &self.lists {
            pass.set_bind_group(0, &list.group, &[]);
            pass.set_pipeline(&self.count);
            pass.dispatch_workgroups(groups, 1, 1);
            pass.set_pipeline(&self.offsets);
            pass.dispatch_workgroups(self.shape.buckets, 1, 1);
            pass.set_pipeline(&self.scatter);
            pass.dispatch_workgroups(groups, 1, 1);
        }
        Ok(true)
    }
}
