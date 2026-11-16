//! Secondary opaque draws share the scene's world data and shading. Slot
//! preparation is separate from asking, and an idle slot begins no GPU pass.

use super::{
    DEPTH_FORMAT, PreparedSynthetic, RenderError, SCENE_FORMAT, SceneContext, SyntheticView,
};
use crate::{
    contribution::{
        SecondaryView, SecondaryViewError, SecondaryViewRecord, SecondaryViewRequest,
        SecondaryViewSlot, ViewAir,
    },
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, GraphDepthAttachment,
        Pass, PassScope, PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
    },
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

/// One name template, used by the graph, timing and observers.
pub(super) const PASS: &str = "secondary view opaque";
pub(super) const PREFIX: &str = "secondary view ";
const ROLE_BASE: u32 = 5000;
const STAGES: u32 = 2;
const RECORD_BYTES: u64 = size_of::<SecondaryViewRecord>() as u64;
mod lighting;

#[derive(Clone, Copy)]
pub(super) enum Stage {
    Opaque,
    Clusters,
}

impl Stage {
    pub(super) fn kind(self) -> crate::graph::PassKind {
        match self {
            Self::Opaque => crate::graph::PassKind::Render,
            Self::Clusters => crate::graph::PassKind::Compute,
        }
    }
}

fn shader_source(base: &str) -> String {
    format!(
        "{base}\n{}\n{}",
        SecondaryView::record_wgsl(),
        include_str!("secondary.wgsl")
    )
}

pub(crate) struct SecondarySet {
    prepared: Vec<SecondaryPrepared>,
    handles: Vec<SecondaryView>,
    sources: Vec<super::SyntheticPointLight>,
}

pub(super) struct SecondaryPrepared {
    handle: SecondaryView,
    queue: wgpu::Queue,
    view: wgpu::Buffer,
    aux: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::RenderPipeline; 3],
    key: PipelineKey,
    lighting: lighting::Lighting,
}

pub(super) struct Resources {
    colour: ResourceId,
    depth: ResourceId,
}

impl PreparedSynthetic {
    pub(crate) fn take_secondary(&mut self) -> SecondarySet {
        SecondarySet {
            prepared: std::mem::take(&mut self.secondary),
            handles: std::mem::take(&mut self.secondary_handles),
            sources: std::mem::take(&mut self.secondary_sources),
        }
    }

    pub(crate) fn restore_secondary(&mut self, set: SecondarySet) {
        self.secondary = set.prepared;
        self.secondary_handles = set.handles;
        self.secondary_sources = set.sources;
    }

    pub(crate) fn secondary_view_count(&self) -> usize {
        self.secondary.len()
    }
    /// Admit and prepare before a contribution sees its surface. No declaration
    /// takes the original preparation path and creates nothing for views.
    pub(crate) fn prepare_secondary_views(
        &mut self,
        queue: &wgpu::Queue,
        slots: &[SecondaryViewSlot],
        extent: [u32; 2],
        sources: &[super::SyntheticPointLight],
    ) -> Result<(), SecondaryViewError> {
        SecondaryViewSlot::validate(slots, extent, self.capabilities.limits())?;
        let uniforms = 5
            + u32::from(self.shadow.is_some())
            + u32::from(self.atmosphere.is_some())
            + u32::from(self.cloud.is_some());
        if !slots.is_empty()
            && self
                .capabilities
                .limits()
                .max_uniform_buffers_per_shader_stage
                < uniforms
        {
            return Err(SecondaryViewError::RecordLimit);
        }
        let prepared = slots
            .iter()
            .zip(0_u32..)
            .map(|(slot, index)| {
                SecondaryPrepared::new(self, queue, index, slot.extent(extent), sources)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.secondary_sources = if slots.is_empty() {
            Vec::new()
        } else {
            sources.to_vec()
        };
        self.secondary_handles = prepared.iter().map(|slot| slot.handle.clone()).collect();
        self.secondary = prepared;
        Ok(())
    }

    pub(super) fn append_secondary_views(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        shared: &[Access],
    ) -> Result<(Vec<Resources>, Vec<Access>), RenderError> {
        let mut resources = Vec::with_capacity(self.secondary.len());
        let mut reads = Vec::with_capacity(self.secondary.len() * 3);
        for (index, slot) in self.secondary.iter().enumerate() {
            let (ids, mut inputs) = slot.append(graph, keys, shared, index)?;
            resources.push(ids);
            reads.append(&mut inputs);
        }
        Ok((resources, reads))
    }

    pub(super) fn secondary_inputs(&self, accesses: &[Access], route: &[Access]) -> Vec<Access> {
        if self.secondary.is_empty() {
            return Vec::new();
        }
        let mut inputs = accesses.to_vec();
        if self.bucket_route.is_some() && self.visibility.is_some() {
            inputs.extend(route.first().cloned());
        }
        inputs
    }

    pub(super) fn append_secondary_consumers(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        mut shared: Vec<Access>,
        exposure: &ResourceId,
        opaque: &mut Vec<Access>,
        late: &mut Vec<Access>,
    ) -> Result<Vec<Resources>, RenderError> {
        if self.secondary.is_empty() {
            return Ok(Vec::new());
        }
        shared.push(Access::new(
            exposure.clone(),
            0..super::exposure::SAMPLE_BYTES,
            AccessKind::UniformRead,
        ));
        let (resources, reads) = self.append_secondary_views(graph, keys, &shared)?;
        opaque.extend_from_slice(&reads);
        late.extend(reads);
        Ok(resources)
    }
}

impl SecondaryPrepared {
    fn new(
        prepared: &PreparedSynthetic,
        queue: &wgpu::Queue,
        index: u32,
        extent: [u32; 2],
        sources: &[super::SyntheticPointLight],
    ) -> Result<Self, SecondaryViewError> {
        let device = prepared.capabilities.device();
        let buffer = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let texture = |format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(PASS),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let colour = texture(SCENE_FORMAT);
        let depth = texture(DEPTH_FORMAT);
        let handle = SecondaryView {
            colour_view: colour.create_view(&wgpu::TextureViewDescriptor::default()),
            depth_view: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            colour,
            depth,
            record: buffer("secondary view record", RECORD_BYTES),
            request: Arc::new(Mutex::new(None)),
            maximum_extent: extent,
            generation: prepared.key.device_generation,
        };
        let mut entries = super::forward::view_entries(
            prepared.bucket_route.is_some(),
            prepared.camera_culling(),
            prepared.shadow.is_some(),
            prepared.atmosphere.is_some(),
        );
        entries.push(super::uniform_layout(
            6,
            RECORD_BYTES,
            wgpu::ShaderStages::FRAGMENT,
        ));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &entries,
        });
        let instances = prepared.pipeline.get_bind_group_layout(3);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[
                Some(&layout),
                Some(&prepared.surface.light_layout),
                Some(&prepared.surface.material_layout),
                Some(&instances),
            ],
            immediate_size: 0,
        });
        let source = shader_source(&prepared.surface.shader_source);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipelines = [
            super::buckets::PipelineClass::Ordinary,
            super::buckets::PipelineClass::Mirrored,
            super::buckets::PipelineClass::TwoSided,
        ]
        .map(|class| {
            device.render(&wgpu::RenderPipelineDescriptor {
                label: Some(PASS),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if prepared.bucket_route.is_some() {
                        "compacted_vertex"
                    } else {
                        "vertex"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: class.primitive(false),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Greater),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if prepared.shadow.is_some() {
                        "secondary_shadowed_fragment"
                    } else {
                        "secondary_fragment"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: SCENE_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        let lighting = lighting::Lighting::new(prepared, queue, sources)?;
        Ok(Self {
            handle,
            queue: queue.clone(),
            view: buffer("secondary raster view", super::VIEW_BYTES as u64),
            aux: buffer("secondary raster auxiliary", super::AUX_BYTES as u64),
            layout,
            pipelines,
            key: PipelineKey {
                role: ROLE_BASE + index * STAGES,
                ..prepared.key
            },
            lighting,
        })
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        main: &[wgpu::BindGroupEntry<'_>],
        route: Option<&super::BucketRoute>,
    ) -> wgpu::BindGroup {
        let mut entries = main.to_vec();
        for entry in &mut entries {
            match entry.binding {
                0 => {
                    entry.resource = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.view,
                        offset: 0,
                        size: wgpu::BufferSize::new(super::VIEW_BYTES as u64),
                    });
                }
                1 => entry.resource = self.aux.as_entire_binding(),
                5 => {
                    if let Some(route) = route {
                        *entry = route.members_entry(5);
                    }
                }
                _ => (),
            }
        }
        entries.push(wgpu::BindGroupEntry {
            binding: 6,
            resource: self.handle.record.as_entire_binding(),
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &self.layout,
            entries: &entries,
        })
    }

    fn append(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        shared: &[Access],
        index: usize,
    ) -> Result<(Resources, Vec<Access>), RenderError> {
        let imported = ResourceScope::Imported { initialized: true };
        let texture = |graph: &mut FrameGraph, texture: &wgpu::Texture| {
            graph.resource(
                ResourceDescriptor::Texture {
                    dimension: crate::graph::TextureKind::D2,
                    width: texture.width(),
                    height: texture.height(),
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: texture.format(),
                    usage: texture.usage(),
                },
                imported,
            )
        };
        let colour = texture(graph, &self.handle.colour)?;
        let depth = texture(graph, &self.handle.depth)?;
        let record = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: RECORD_BYTES,
                usage: self.handle.record.usage(),
            },
            imported,
        )?;
        let mut accesses = shared.to_vec();
        self.lighting.append(
            graph,
            keys,
            PipelineKey {
                role: self.key.role + 1,
                ..self.key
            },
            index,
            shared,
            &mut accesses,
        )?;
        for bytes in [super::VIEW_BYTES as u64, super::AUX_BYTES as u64] {
            let uniform = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                },
                imported,
            )?;
            accesses.push(Access::new(uniform, 0..bytes, AccessKind::UniformRead));
        }
        accesses.extend([
            Access::new(record.clone(), 0..RECORD_BYTES, AccessKind::UniformRead),
            Access::new(colour.clone(), 0..1, AccessKind::AttachmentClear),
            Access::new(depth.clone(), 0..1, AccessKind::AttachmentClear),
        ]);
        keys.insert(self.key);
        graph.pass(Pass {
            name: format!("{PASS} {index}"),
            scope: PassScope::View(index as u64 + 2),
            pipeline: Some(self.key),
            accesses,
            external_side_effect: true,
        });
        let reads = vec![
            Access::new(colour.clone(), 0..1, AccessKind::SampledRead),
            Access::new(depth.clone(), 0..1, AccessKind::SampledRead),
            Access::new(record, 0..RECORD_BYTES, AccessKind::UniformRead),
        ];
        Ok((Resources { colour, depth }, reads))
    }

    fn upload(&self, request: &SecondaryViewRequest, view: &SyntheticView, exposure: f32) {
        let raster = view.raster_projection();
        let unjittered = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
        let [x, y, z] = view.camera_position();
        let record = SecondaryViewRecord {
            view_proj: raster.to_cols_array_2d(),
            inverse_view_proj: raster.inverse().to_cols_array_2d(),
            unjittered_view_proj: unjittered.to_cols_array_2d(),
            inverse_unjittered_view_proj: unjittered.inverse().to_cols_array_2d(),
            camera: [x, y, z, 0.0],
            extent: [
                request.extent[0] as f32,
                request.extent[1] as f32,
                self.handle.maximum_extent[0] as f32,
                self.handle.maximum_extent[1] as f32,
            ],
            clip_plane: request
                .clip
                .map_or([0.0; 4], crate::contribution::ViewClipPlane::equation),
            options: [
                f32::from(request.clip.is_some()),
                f32::from(request.air == ViewAir::Plane),
                request.least_pixels,
                0.0,
            ],
        };
        self.queue
            .write_buffer(&self.view, 0, &view.pack_view(view, exposure));
        self.queue.write_buffer(
            &self.aux,
            0,
            &view.pack_aux(request.extent[0], request.extent[1]),
        );
        self.queue
            .write_buffer(&self.handle.record, 0, bytemuck::bytes_of(&record));
    }
}

impl SecondaryPrepared {
    pub(super) fn pass_count(&self) -> u32 {
        1 + u32::from(self.lighting.gpu)
    }

    fn camera(prepared: &PreparedSynthetic, request: &SecondaryViewRequest) -> SyntheticView {
        let main = prepared
            .pending_view
            .as_ref()
            .unwrap_or(&prepared.view_sample);
        if main.raster_jitter_enabled() {
            request
                .camera
                .without_jitter()
                .with_jitter_phase(main.raster_phase(), request.extent)
        } else {
            request.camera.without_jitter()
        }
    }
}

impl SceneContext<'_> {
    pub(super) fn secondary_stage(
        &mut self,
        index: usize,
        stage: Stage,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        match stage {
            Stage::Opaque => self.secondary(index, compiled),
            Stage::Clusters => {
                let prepared = self.prepared;
                let slot = &prepared.secondary[index];
                let Some(request) = slot.handle.asked() else {
                    return Ok(());
                };
                let view = SecondaryPrepared::camera(prepared, &request);
                slot.lighting
                    .update(&slot.queue, &prepared.secondary_sources, &view)?;
                slot.lighting.encode(
                    self.encoder,
                    self.generation,
                    compiled,
                    self.timings.as_deref_mut(),
                )
            }
        }
    }
    pub(super) fn secondary(
        &mut self,
        index: usize,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        let prepared = self.prepared;
        let slot = &prepared.secondary[index];
        let Some(request) = slot.handle.asked() else {
            return Ok(());
        };
        let view = SecondaryPrepared::camera(prepared, &request);
        if !slot.lighting.gpu {
            slot.lighting
                .update(&slot.queue, &prepared.secondary_sources, &view)?;
        }
        slot.upload(&request, &view, f32::from_bits(prepared.exposure_bits));
        let ids = &self.target.ids.secondary[index];
        let mut pass = compiled
            .begin_render(
                self.encoder,
                [Some(GraphColorAttachment {
                    resource: &ids.colour,
                    range: 0..1,
                    view: &slot.handle.colour_view,
                    clear: wgpu::Color::TRANSPARENT,
                })],
                Some(GraphDepthAttachment {
                    resource: &ids.depth,
                    range: 0..1,
                    view: &slot.handle.depth_view,
                    clear: 0.0,
                }),
                self.generation,
                self.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_viewport(
            0.0,
            0.0,
            request.extent[0] as f32,
            request.extent[1] as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(1, &slot.lighting.groups[self.exposure_side], &[]);
        pass.set_bind_group(2, &prepared.materials, &[]);
        pass.set_bind_group(3, prepared.history.read_group(), &[]);
        if let Some(route) = &prepared.bucket_route {
            route.draw_secondary(
                &mut pass,
                &self.target.secondary[index],
                &prepared.mesh_arena,
                &slot.pipelines,
            )?;
        } else {
            pass.set_pipeline(
                &slot.pipelines[if prepared.instance_upload.two_sided_count() == 0 {
                    0
                } else {
                    2
                }],
            );
            pass.set_bind_group(0, &self.target.secondary[index], &[0]);
            let buffers = prepared
                .mesh_arena
                .buffers()
                .ok_or(RenderError::GraphPassMismatch)?;
            pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(
                prepared.cube_draw.0.clone(),
                prepared.cube_draw.1,
                0..prepared.instance_upload.active_count(),
            );
        }
        Ok(())
    }
}

pub(super) fn index(key: PipelineKey, count: usize) -> Option<(usize, Stage)> {
    let role = key.role.checked_sub(ROLE_BASE)?;
    let index = (role / STAGES) as usize;
    let stage = match role % STAGES {
        0 => Stage::Opaque,
        _ => Stage::Clusters,
    };
    (index < count).then_some((index, stage))
}

#[cfg(test)]
mod tests;
