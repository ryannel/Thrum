//! Private GPU camera visibility. A compute pass tests each active instance's
//! conservative world bounds against the camera frustum and writes an
//! order-preserving visible list plus one `DrawIndexedIndirectArgs` per
//! mesh/pipeline bucket. Outputs belong to a frame slot's target. Sun casters
//! never read this list. The direct draw, which never reads it, is the oracle.
//!
//! Occlusion runs the same kernels twice per frame. The prepass list holds
//! slots in view that were visible last frame; they draw depth only into a
//! private target, and the depth pyramid is built from it. The final cull
//! then lists, in slot order, every slot in view that the pyramid does not
//! hide, and records that list as the next last-visible history. The colour
//! pass draws only the final list, so its draw sequence is Direct's minus
//! slots strictly behind depth drawn this frame, which win no pixel.

use crate::binding;
use crate::graph::PassBody;
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::pass_key::{PassKey, PipelineRole};
use crate::shader_constants::{
    PHASE_FINAL, PHASE_FRUSTUM, PHASE_PREPASS, VISIBILITY_WORKGROUP as WORKGROUP,
};
use crate::timing::{GpuTimings, TimingError};

/// `DrawIndexedIndirectArgs` bytes.
pub(crate) const ARGS_BYTES: u64 = 20;
const MESH_LEN: usize = size_of::<crate::records::MeshRecord>();
const PARAMS_LEN: usize = size_of::<crate::records::VisibilityParams>();
/// One parameter record per phase, at the portable uniform offset alignment.
const PARAMS_STRIDE: u32 = 256;
const PHASES: u32 = PHASE_FINAL + 1;
/// Last-visible bits per workgroup of slots.
const WORDS: u32 = WORKGROUP / u32::BITS;
/// Graph owner of the last-visible history pair.
const HISTORY_OWNER: u64 = 0x5649_4849;
const VIEW_BYTES: u64 = size_of::<crate::records::ViewRecord>() as u64;
/// Graph pass name of the frustum compaction.
pub(crate) const PASS: &str = PassKey::CameraVisibility.constant_name();
/// Graph pass name of the occlusion prepass list.
pub(crate) const PREPASS_LIST_PASS: &str = PassKey::PrepassList.constant_name();
/// Graph pass name of the final occlusion cull.
pub(crate) const FINAL_PASS: &str = PassKey::FinalCull.constant_name();
pub(crate) use pyramid::PASS as PYRAMID_PASS;
mod secondary;
pub(crate) use secondary::SecondaryVisibility;

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
        let pad = |[x, y, z]: [f32; 3]| [x, y, z, 0.0];
        bytemuck::cast(crate::records::MeshRecord {
            first_index: self.first_index,
            index_count: self.index_count,
            base_vertex: self.base_vertex,
            padding: 0,
            bounds_min: pad(self.bounds_min),
            bounds_max: pad(self.bounds_max),
        })
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
    /// Occlusion: last frame's visible set draws depth only, and every slot
    /// in view is then tested against that depth's pyramid. The colour pass
    /// draws the survivors in slot order, so the image equals Direct's.
    Occlusion,
}

/// Which slots one target's compaction lists; selects its parameter record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum Phase {
    /// Every slot in view.
    Frustum = PHASE_FRUSTUM,
    /// Slots in view that were visible last frame: the depth prepass.
    Prepass = PHASE_PREPASS,
    /// Slots in view that this frame's pyramid does not hide.
    Final = PHASE_FINAL,
}

/// Device-local pipelines and the mesh table.
#[derive(Clone)]
pub(crate) struct VisibilityPrepared {
    layout: wgpu::BindGroupLayout,
    count: wgpu::ComputePipeline,
    offsets: wgpu::ComputePipeline,
    scatter: wgpu::ComputePipeline,
    meshes: wgpu::Buffer,
    params: wgpu::Buffer,
    /// The camera's last-visible history, two sides of one bit per slot,
    /// paired with the transform history's sides: a frame reads the committed
    /// side and its final cull writes the other, so only a committed display
    /// frame advances it and a capture or cancelled frame cannot. Zero after
    /// creation, so a new device or capacity starts with an empty prepass.
    history: [wgpu::Buffer; 2],
    /// Bound wherever no pyramid is read: frustum and prepass targets.
    idle_pyramid: wgpu::TextureView,
    /// Occlusion only: the depth pyramid build.
    pyramid: Option<pyramid::PyramidPrepared>,
    /// The bucket route: compaction over its member list, one argument
    /// record per bucket.
    members: Option<MemberCompaction>,
    capacity: u32,
    active: u32,
    key: PipelineKey,
}

/// The member-indexed compaction's fixed shape and its current layout.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MemberShape {
    /// Buckets, each with its own argument record (at most 64).
    pub(crate) buckets: u32,
    /// The largest member list any layout can need, in slots.
    pub(crate) worst_members: u32,
    /// Bytes each draw binds from its bucket's offset in the visible list.
    pub(crate) segment_bytes: u64,
}

#[derive(Clone)]
struct MemberCompaction {
    shape: MemberShape,
    /// Workgroups in the current member list.
    groups: u32,
}

/// The route's member list and uniform bucket table, bound by member targets.
#[derive(Clone, Copy)]
pub(crate) struct MemberInputs<'a> {
    pub(crate) members: &'a wgpu::Buffer,
    pub(crate) table: &'a wgpu::Buffer,
}

/// Frame-slot-owned outputs and the bind group that produces them.
#[derive(Clone)]
pub(crate) struct VisibilityTarget {
    visible: wgpu::Buffer,
    args: wgpu::Buffer,
    /// Indexed by the history side the frame writes.
    groups: [wgpu::BindGroup; 2],
    phase: Phase,
}

/// Graph identities of one target's visibility outputs.
#[derive(Clone)]
pub(crate) struct VisibilityIds {
    /// Visible slot list, read by the forward vertex stage.
    pub(crate) visible: ResourceId,
    /// Indirect arguments, read by the forward draw.
    pub(crate) args: ResourceId,
    /// Occlusion only: the camera's last-visible history pair, the committed
    /// side read by the prepass list and the next side written by the final
    /// cull.
    history: Option<HistoryIds>,
}

/// The graph's two sides of the last-visible history.
#[derive(Clone, Debug)]
pub(crate) struct HistoryIds {
    committed: ResourceId,
    next: ResourceId,
}

/// One frame slot's occlusion state: the prepass list and the pyramid built
/// from the depth it draws.
pub(crate) struct OcclusionTarget {
    prepass: VisibilityTarget,
    pyramid: pyramid::PyramidTarget,
}

impl VisibilityPrepared {
    pub(crate) fn update_meshes(&self, queue: &wgpu::Queue, meshes: &[MeshRecord]) {
        let bytes: Vec<u8> = meshes.iter().flat_map(|mesh| mesh.pack()).collect();
        queue.write_buffer(&self.meshes, 0, &bytes);
    }

    /// Prepare for at most `capacity` instance slots and a stable mesh table.
    pub(crate) fn new(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        key: PipelineKey,
        capacity: u32,
        meshes: &[MeshRecord],
        active: u32,
        mode: CullMode,
    ) -> Self {
        Self::build(device, queue, key, (capacity, active), meshes, mode, None)
    }

    /// Prepare the member-indexed compaction for the bucket route: the
    /// member list replaces the slot range, and each bucket writes its own
    /// argument record. History follows logical slot identity.
    pub(crate) fn new_members(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        key: PipelineKey,
        (capacity, active): (u32, u32),
        meshes: &[MeshRecord],
        shape: MemberShape,
        mode: CullMode,
    ) -> Self {
        Self::build(
            device,
            queue,
            key,
            (capacity, active),
            meshes,
            mode,
            Some(shape),
        )
    }

    fn build(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        key: PipelineKey,
        (capacity, active): (u32, u32),
        meshes: &[MeshRecord],
        mode: CullMode,
        shape: Option<MemberShape>,
    ) -> Self {
        let stages = wgpu::ShaderStages::COMPUTE;
        let entry = |binding, ty| binding::buffer(binding, stages, ty, 0);
        // Bindings 0 and 4 take a dynamic offset, so they keep a literal.
        let dynamic_uniform = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: stages,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            count: None,
        };
        let read = wgpu::BufferBindingType::Storage { read_only: true };
        let write = wgpu::BufferBindingType::Storage { read_only: false };
        let uniform = wgpu::BufferBindingType::Uniform;
        let mut entries = vec![
            dynamic_uniform(0),
            entry(1, read),
            entry(2, uniform),
            entry(3, read),
            dynamic_uniform(4),
            entry(5, write),
            entry(6, write),
            entry(7, write),
            entry(9, read),
            entry(11, write),
            binding::texture(
                10,
                wgpu::ShaderStages::COMPUTE,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
        ];
        // WebGPU allows 8 storage buffers per stage: the member path trades
        // the mask buffer for its member list and reads its table as a uniform.
        if shape.is_some() {
            entries.extend([entry(12, read), entry(13, uniform)]);
        } else {
            entries.push(entry(8, write));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera visibility"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera visibility"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = crate::shaders::Shader::Visibility.compile(device, "camera visibility");
        let pipeline = |entry: &str| {
            device.compute(&wgpu::ComputePipelineDescriptor {
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
            size: u64::from(PARAMS_STRIDE * PHASES),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let history = std::array::from_fn(|_| {
            storage(
                device,
                "visibility last-visible history",
                bits_bytes(capacity),
            )
        });
        let idle_pyramid = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("visibility idle pyramid"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: pyramid::PYRAMID_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let pyramid = (mode == CullMode::Occlusion).then(|| pyramid::PyramidPrepared::new(device));
        let suffix = if shape.is_some() { "_members" } else { "" };
        let prepared = Self {
            count: pipeline(&format!("count{suffix}")),
            offsets: pipeline(&format!("offsets{suffix}")),
            scatter: pipeline(&format!("scatter{suffix}")),
            members: shape.map(|shape| MemberCompaction { shape, groups: 0 }),
            layout,
            meshes,
            params,
            history,
            idle_pyramid,
            pyramid,
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

    /// Frustum, or Occlusion when the pyramid is prepared.
    pub(crate) fn mode(&self) -> CullMode {
        if self.pyramid.is_some() {
            CullMode::Occlusion
        } else {
            CullMode::Frustum
        }
    }

    /// Occlusion only: the final cull's pipeline key.
    pub(crate) fn final_key(&self) -> PipelineKey {
        PipelineRole::FinalCull.key(self.key)
    }

    /// Occlusion only: the pyramid build's pipeline key.
    pub(crate) fn pyramid_key(&self) -> PipelineKey {
        PipelineRole::DepthPyramid.key(self.key)
    }

    /// Bytes of one target's visible slot list. The member path leaves room
    /// to bind a full segment at any bucket's offset.
    pub(crate) fn visible_bytes(&self) -> u64 {
        match &self.members {
            Some(members) => {
                u64::from(members.shape.worst_members) * 4 + members.shape.segment_bytes
            }
            None => u64::from(self.capacity.max(1)) * 4,
        }
    }

    /// Bytes of one target's indirect arguments: one record per bucket.
    pub(crate) fn args_bytes(&self) -> u64 {
        let records = self
            .members
            .as_ref()
            .map_or(1, |members| members.shape.buckets);
        u64::from(records.max(1)) * ARGS_BYTES
    }

    /// The bucket route: record the current member list's workgroup count.
    /// The member list and table themselves are the route's queue writes.
    pub(crate) fn set_member_groups(&mut self, groups: u32) {
        if let Some(members) = &mut self.members {
            members.groups = groups;
        }
    }

    /// The bucket route: bytes each draw binds at its bucket's offset.
    pub(crate) fn segment_bytes(&self) -> Option<u64> {
        self.members
            .as_ref()
            .map(|members| members.shape.segment_bytes)
    }

    fn groups(&self) -> u32 {
        self.active.div_ceil(WORKGROUP).max(1)
    }

    fn write_params(&self, queue: &wgpu::Queue) {
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Slot {
            record: crate::records::VisibilityParams,
            padding: [[u32; 4]; (PARAMS_STRIDE as usize - PARAMS_LEN) / 16],
        }

        let slots: [Slot; PHASES as usize] = std::array::from_fn(|phase| Slot {
            record: crate::records::VisibilityParams {
                active_count: self.active,
                groups: self.groups(),
                mesh: 0,
                phase: u32::try_from(phase).expect("visibility phase fits u32"),
            },
            padding: [[0; 4]; (PARAMS_STRIDE as usize - PARAMS_LEN) / 16],
        });
        queue.write_buffer(&self.params, 0, bytemuck::cast_slice(&slots));
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

    /// One frame slot's frustum target, over the route's member list when
    /// the compaction is member-indexed.
    pub(crate) fn member_target(
        &self,
        device: &wgpu::Device,
        (view, instances, serials): (&wgpu::Buffer, &wgpu::Buffer, &wgpu::Buffer),
        members: Option<MemberInputs<'_>>,
    ) -> VisibilityTarget {
        let scratch = self.scratch(device);
        self.phase_target(
            device,
            Inputs {
                view,
                instances,
                serials,
                pyramid: &self.idle_pyramid,
                members,
            },
            &scratch,
            Phase::Frustum,
        )
    }

    /// Occlusion only: one frame slot's final target, and its prepass list
    /// with the pyramid of the prepass `depth` (extent `extent`).
    pub(crate) fn occlusion_target(
        &self,
        device: &wgpu::Device,
        view: &wgpu::Buffer,
        instances: &wgpu::Buffer,
        serials: &wgpu::Buffer,
        depth: &wgpu::TextureView,
        extent: [u32; 2],
        members: Option<MemberInputs<'_>>,
    ) -> Option<(VisibilityTarget, OcclusionTarget)> {
        let pyramid = self.pyramid.as_ref()?.target(device, depth, extent);
        let mips = pyramid
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let [prepass, last] =
            self.occlusion_targets(device, view, instances, serials, &mips, members);
        Some((last, OcclusionTarget { prepass, pyramid }))
    }

    /// Allocate one frame slot's prepass and final targets. They share
    /// scratch, which their ordered passes use in turn, and the camera's
    /// history; the final target reads `pyramid`, every mip of this slot's
    /// depth pyramid.
    pub(crate) fn occlusion_targets(
        &self,
        device: &wgpu::Device,
        view: &wgpu::Buffer,
        instances: &wgpu::Buffer,
        serials: &wgpu::Buffer,
        pyramid: &wgpu::TextureView,
        members: Option<MemberInputs<'_>>,
    ) -> [VisibilityTarget; 2] {
        let scratch = self.scratch(device);
        let inputs = Inputs {
            view,
            instances,
            serials,
            pyramid,
            members,
        };
        let prepass = Inputs {
            pyramid: &self.idle_pyramid,
            ..inputs
        };
        [
            self.phase_target(device, prepass, &scratch, Phase::Prepass),
            self.phase_target(device, inputs, &scratch, Phase::Final),
        ]
    }

    fn scratch(&self, device: &wgpu::Device) -> [wgpu::Buffer; 2] {
        // The member path keeps its masks after the counts.
        let group_offsets = match &self.members {
            Some(members) => {
                let groups = members.shape.worst_members.div_ceil(WORKGROUP).max(1);
                u64::from(groups) * u64::from(1 + WORDS) * 4
            }
            None => u64::from(self.capacity.div_ceil(WORKGROUP).max(1)) * 4,
        };
        let group_offsets = storage(device, "visibility group offsets", group_offsets);
        // One bit per slot, written by `count` and read by `scatter`, so the
        // instance records are read and tested once.
        let masks = storage(device, "visibility slot masks", bits_bytes(self.capacity));
        [group_offsets, masks]
    }

    fn phase_target(
        &self,
        device: &wgpu::Device,
        inputs: Inputs<'_>,
        [group_offsets, masks]: &[wgpu::Buffer; 2],
        phase: Phase,
    ) -> VisibilityTarget {
        let Inputs {
            view,
            instances,
            serials,
            pyramid,
            members,
        } = inputs;
        let visible = storage(device, "visibility visible slots", self.visible_bytes());
        let args = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visibility indirect arguments"),
            size: self.args_bytes(),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let bind = |pyramid: &wgpu::TextureView| -> [wgpu::BindGroup; 2] {
            std::array::from_fn(|write| {
                let mut entries = vec![
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.params,
                            offset: 0,
                            size: wgpu::BufferSize::new(PARAMS_LEN as u64),
                        }),
                    },
                    binding(5, group_offsets),
                    binding(6, &visible),
                    binding(7, &args),
                    binding(9, &self.history[1 - write]),
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(pyramid),
                    },
                    binding(11, &self.history[write]),
                ];
                if let Some(members) = members {
                    entries.extend([binding(12, members.members), binding(13, members.table)]);
                } else {
                    entries.push(binding(8, masks));
                }
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("camera visibility"),
                    layout: &self.layout,
                    entries: &entries,
                })
            })
        };
        VisibilityTarget {
            groups: bind(pyramid),
            visible,
            args,
            phase,
        }
    }

    /// Declare the frustum or prepass-list compute pass and its outputs. `reads`
    /// carries the view, instance and serial accesses from the forward
    /// pass's own resources.
    pub(crate) fn append_graph(
        &self,
        graph: &mut FrameGraph,
        mut reads: Vec<Access>,
    ) -> Result<VisibilityIds, GraphError> {
        let history = self
            .pyramid
            .is_some()
            .then(|| self.history_ids(graph))
            .transpose()?;
        reads.extend(history.iter().map(|history| {
            Access::new(
                history.committed.clone(),
                0..bits_bytes(self.capacity),
                AccessKind::StorageRead,
            )
        }));
        let _name = if history.is_some() {
            PREPASS_LIST_PASS
        } else {
            PASS
        };
        let (visible, args) = self.outputs(graph, &mut reads)?;
        graph.declare(
            if history.is_some() {
                PassKey::PrepassList
            } else {
                PassKey::CameraVisibility
            },
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(self.key),
                accesses: reads,
                external_side_effect: false,
            },
        );
        Ok(VisibilityIds {
            visible,
            args,
            history,
        })
    }

    /// Both sides of the last-visible history, as a graph history pair like
    /// the transform history: the executor binds the committed side for
    /// reads and the alternate side for the final cull's publication.
    fn history_ids(&self, graph: &mut FrameGraph) -> Result<HistoryIds, GraphError> {
        let descriptor = ResourceDescriptor::Buffer {
            bytes: bits_bytes(self.capacity),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        };
        let scope = ResourceScope::History {
            owner: HISTORY_OWNER,
            epoch: self.key.device_generation,
            initialized: true,
        };
        Ok(HistoryIds {
            committed: graph.resource(descriptor.clone(), scope)?,
            next: graph.resource(descriptor, scope)?,
        })
    }

    /// Occlusion only: declare the pyramid build from the prepass `depth`
    /// and the final cull. `reads` are the same view, instance and serial
    /// accesses the prepass list was given.
    pub(crate) fn append_final_graph(
        &self,
        graph: &mut FrameGraph,
        mut reads: Vec<Access>,
        prepass: &VisibilityIds,
        depth: &ResourceId,
        extent: [u32; 2],
    ) -> Result<VisibilityIds, GraphError> {
        let history = prepass
            .history
            .clone()
            .ok_or(GraphError::InvalidDependency)?;
        let size = [
            pyramid::previous_pow2(extent[0]),
            pyramid::previous_pow2(extent[1]),
        ];
        let mips = size[0].max(size[1]).ilog2() + 1;
        let texture = graph.resource(
            ResourceDescriptor::Texture {
                dimension: crate::graph::TextureKind::D2,
                width: size[0],
                height: size[1],
                layers: 1,
                mips,
                samples: 1,
                format: pyramid::PYRAMID_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        graph.declare(
            PassKey::DepthPyramid,
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(self.pyramid_key()),
                accesses: vec![
                    Access::new(depth.clone(), 0..1, AccessKind::SampledRead),
                    Access::new(
                        texture.clone(),
                        0..u64::from(mips),
                        AccessKind::StorageWrite,
                    ),
                ],
                external_side_effect: false,
            },
        );
        reads.extend([
            Access::new(texture, 0..u64::from(mips), AccessKind::SampledRead),
            Access::new(
                history.committed.clone(),
                0..bits_bytes(self.capacity),
                AccessKind::StorageRead,
            ),
            // The dispatched words and the cleared tail define every word.
            Access::new(
                history.next.clone(),
                0..bits_bytes(self.capacity),
                AccessKind::StorageWrite,
            ),
        ]);
        let (visible, args) = self.outputs(graph, &mut reads)?;
        graph.declare(
            PassKey::FinalCull,
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(self.final_key()),
                accesses: reads,
                external_side_effect: false,
            },
        );
        Ok(VisibilityIds {
            visible,
            args,
            history: Some(history),
        })
    }

    fn outputs(
        &self,
        graph: &mut FrameGraph,
        reads: &mut Vec<Access>,
    ) -> Result<(ResourceId, ResourceId), GraphError> {
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
                bytes: self.args_bytes(),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::INDIRECT
                    | wgpu::BufferUsages::COPY_SRC,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        reads.extend([
            Access::new(visible.clone(), 0..visible_bytes, AccessKind::StorageWrite),
            Access::new(args.clone(), 0..self.args_bytes(), AccessKind::StorageWrite),
        ]);
        Ok((visible, args))
    }

    /// Encode the three ordered dispatches into the graph's compute pass,
    /// with the target's phase record. The frame reads history side
    /// `1 - write_side` and its final cull writes `write_side`.
    pub(crate) fn encode_phase(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &VisibilityTarget,
        write_side: usize,
        (view_offset, generation): (u32, u64),
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<bool, TimingError> {
        let (groups, records) = match &self.members {
            Some(members) => (members.groups.max(1), members.shape.buckets),
            None => (self.groups(), 1),
        };
        let phase = target.phase;
        // Logical slots from different buckets can share a word. Clear the
        // next side before the atomic ORs; layout shrink and inactive slots
        // must not retain bits from an earlier submission.
        if phase == Phase::Final {
            encoder.clear_buffer(&self.history[write_side & 1], 0, None);
        }
        let Some(mut pass) = compiled.begin_compute(encoder, generation, timings)? else {
            return Ok(false);
        };
        let params_offset = phase as u32 * PARAMS_STRIDE;
        pass.set_bind_group(
            0,
            &target.groups[write_side & 1],
            &[view_offset, params_offset],
        );
        pass.set_pipeline(&self.count);
        pass.dispatch_workgroups(groups, 1, 1);
        pass.set_pipeline(&self.offsets);
        pass.dispatch_workgroups(records, 1, 1);
        pass.set_pipeline(&self.scatter);
        pass.dispatch_workgroups(groups, 1, 1);
        Ok(true)
    }
}

impl OcclusionTarget {
    pub(crate) fn prepass(&self) -> &VisibilityTarget {
        &self.prepass
    }
}

impl VisibilityPrepared {
    /// Occlusion only: build the slot's pyramid in the graph's compute pass.
    pub(crate) fn encode_pyramid(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OcclusionTarget,
        generation: u64,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<bool, TimingError> {
        let Some(prepared) = &self.pyramid else {
            return Ok(false);
        };
        let Some(mut pass) = compiled.begin_compute(encoder, generation, timings)? else {
            return Ok(false);
        };
        prepared.encode(&mut pass, &target.pyramid);
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

/// The per-target inputs every phase binds.
#[derive(Clone, Copy)]
struct Inputs<'a> {
    view: &'a wgpu::Buffer,
    instances: &'a wgpu::Buffer,
    serials: &'a wgpu::Buffer,
    pyramid: &'a wgpu::TextureView,
    members: Option<MemberInputs<'a>>,
}

/// Bytes of one bit per slot, rounded up to whole workgroups.
fn bits_bytes(capacity: u32) -> u64 {
    u64::from(capacity.div_ceil(WORKGROUP).max(1)) * u64::from(WORDS) * 4
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

pub(crate) mod casters;
#[cfg(test)]
mod occlusion_reference;
mod pyramid;
#[cfg(test)]
mod tests;
