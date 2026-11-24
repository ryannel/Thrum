//! Secondary opaque draws share the scene's world data and shading. Slot
//! preparation is separate from asking, and an idle slot begins no GPU pass.

use super::{
    DEPTH_FORMAT, PreparedSynthetic, RenderError, SCENE_FORMAT, SceneContext, SyntheticView,
};
use crate::binding;
use crate::graph::PassBody;
use crate::pass_key::{PassKey, PipelineRole, SecondaryStage};
use crate::shaders::wgsl;
use crate::{
    contribution::{
        SecondaryView, SecondaryViewError, SecondaryViewRecord, SecondaryViewRequest,
        SecondaryViewSlot, ViewAir,
    },
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, GraphDepthAttachment,
        PassScope, PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
    },
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

/// One name template, used by the graph, timing and observers.
pub(super) const PASS: &str = "secondary view opaque";
const RECORD_BYTES: u64 = size_of::<SecondaryViewRecord>() as u64;
mod lighting;
mod sun;

fn shader_input_counts(atmosphere: bool, shadows: bool, cloud: bool) -> [u32; 2] {
    let atmosphere = u32::from(atmosphere);
    let shadows = u32::from(shadows);
    let cloud = u32::from(cloud);
    [
        6 + shadows + atmosphere + cloud,
        4 + 5 * atmosphere + shadows * (2 + atmosphere) + cloud,
    ]
}

pub(super) use crate::pass_key::SecondaryStage as Stage;

impl Stage {
    pub(super) fn kind(self) -> crate::graph::PassKind {
        match self {
            Self::Opaque | Self::SunNear | Self::SunFar => crate::graph::PassKind::Render,
            Self::Clusters | Self::Aerial | Self::Visibility | Self::Casters => {
                crate::graph::PassKind::Compute
            }
        }
    }
}

fn shader_source(base: &str) -> String {
    format!(
        "{base}\n{}\n{}",
        SecondaryView::record_wgsl(),
        wgsl!("synthetic/secondary.wgsl")
    )
}

pub(crate) struct SecondarySet {
    prepared: Vec<SecondaryPrepared>,
    handles: Vec<SecondaryView>,
    sources: Vec<super::SyntheticPointLight>,
}

impl SecondarySet {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn handles(&self) -> &[SecondaryView] {
        &self.handles
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl super::SyntheticTargetInputs {
    pub(crate) fn resize_secondary(&mut self, extents: &[[u32; 2]]) -> SecondarySet {
        assert_eq!(self.secondary.len(), extents.len());
        for (slot, &extent) in self.secondary.iter_mut().zip(extents) {
            if slot.handle.maximum_extent() != extent {
                let device = self.capabilities.device();
                slot.handle.colour = secondary_texture(device, extent, SCENE_FORMAT);
                slot.handle.depth = secondary_texture(device, extent, DEPTH_FORMAT);
                slot.handle.colour_view = slot
                    .handle
                    .colour
                    .create_view(&wgpu::TextureViewDescriptor::default());
                slot.handle.depth_view = slot
                    .handle
                    .depth
                    .create_view(&wgpu::TextureViewDescriptor::default());
                slot.handle.maximum_extent = extent;
                slot.handle.request = Arc::new(Mutex::new(crate::contribution::ViewRequestState {
                    camera: self.view_sample.camera_position(),
                    asked: None,
                }));
            }
        }
        SecondarySet {
            prepared: self.secondary.clone(),
            handles: self
                .secondary
                .iter()
                .map(|slot| slot.handle.clone())
                .collect(),
            sources: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub(super) struct SecondaryPrepared {
    handle: SecondaryView,
    queue: wgpu::Queue,
    view: wgpu::Buffer,
    aux: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::RenderPipeline; 3],
    source: Arc<str>,
    key: PipelineKey,
    lighting: lighting::Lighting,
    aerial: Option<crate::atmosphere::SecondaryAerial>,
    sun: Option<sun::Sun>,
    culling: Option<(
        crate::visibility::SecondaryVisibility,
        crate::visibility::VisibilityTarget,
    )>,
}

pub(super) struct Resources {
    colour: ResourceId,
    depth: ResourceId,
    sun: Option<ResourceId>,
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn publish_secondary_extent(&mut self, mut set: SecondarySet) {
        set.sources = std::mem::take(&mut self.secondary_sources);
        self.restore_secondary(set);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn secondary_extents(&self) -> impl Iterator<Item = [u32; 2]> + '_ {
        self.secondary_handles
            .iter()
            .map(SecondaryView::maximum_extent)
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
        if !slots.is_empty()
            && self.shadow.is_some()
            && self.capabilities.limits().max_bindings_per_bind_group < 8
        {
            return Err(SecondaryViewError::RecordLimit);
        }
        let [uniforms, textures] = shader_input_counts(
            self.atmosphere.is_some(),
            self.shadow.is_some(),
            self.cloud.is_some(),
        );
        if !slots.is_empty()
            && self
                .capabilities
                .limits()
                .max_uniform_buffers_per_shader_stage
                < uniforms
        {
            return Err(SecondaryViewError::RecordLimit);
        }
        // Material arrays (4), atmosphere products (5), own cascades and the
        // shared height field (2), atmospheric Sun visibility (1), and cloud.
        if !slots.is_empty()
            && self
                .capabilities
                .limits()
                .max_sampled_textures_per_shader_stage
                < textures
        {
            return Err(SecondaryViewError::ShaderLimit);
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
}

impl super::SyntheticTargetInputs {
    pub(super) fn append_secondary_views(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        shared: &[Access],
    ) -> Result<(Vec<Resources>, Vec<Access>), RenderError> {
        let mut resources = Vec::with_capacity(self.secondary.len());
        let mut reads = Vec::with_capacity(self.secondary.len() * 3);
        for (index, slot) in self.secondary.iter().enumerate() {
            let (ids, mut inputs) =
                slot.append(graph, keys, shared, index, self.visibility.as_ref())?;
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
            inputs.extend_from_slice(route);
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

fn secondary_texture(
    device: &wgpu::Device,
    extent: [u32; 2],
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
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
}

impl SecondaryPrepared {
    /// Pipeline key of one of this view's stages.
    fn stage_key(&self, index: usize, stage: Stage) -> PipelineKey {
        PipelineRole::Secondary {
            view: crate::pass_key::pass_index(index),
            stage,
        }
        .key(self.key)
    }

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
        let colour = secondary_texture(device, extent, SCENE_FORMAT);
        let depth = secondary_texture(device, extent, DEPTH_FORMAT);
        let view = buffer("secondary raster view", super::VIEW_BYTES as u64);
        let handle = SecondaryView {
            colour_view: colour.create_view(&wgpu::TextureViewDescriptor::default()),
            depth_view: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            colour,
            depth,
            record: buffer("secondary view record", RECORD_BYTES),
            request: Arc::new(Mutex::new(crate::contribution::ViewRequestState {
                camera: prepared.view_sample.camera_position(),
                asked: None,
            })),
            maximum_extent: extent,
            generation: prepared.key.device_generation,
        };
        let mut entries = super::forward::view_entries(
            prepared.bucket_route.is_some(),
            prepared.camera_culling(),
            prepared.shadow.is_some(),
            prepared.atmosphere.is_some(),
        );
        entries.push(binding::uniform(
            6,
            wgpu::ShaderStages::FRAGMENT,
            RECORD_BYTES,
        ));
        if prepared.shadow.is_some() {
            let mut height = *entries
                .iter()
                .find(|entry| entry.binding == 3)
                .expect("shadow layout includes its map array");
            height.binding = 7;
            entries.push(height);
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &entries,
        });
        let instances = prepared.pipeline.get_bind_group_layout(3);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[
                Some(&layout),
                Some(&prepared.map_light_layout),
                Some(&prepared.surface.material_layout),
                Some(&instances),
            ],
            immediate_size: 0,
        });
        let variant = crate::shaders::SceneVariant {
            atmosphere: crate::shaders::AtmosphereVariant::new(
                prepared.atmosphere.is_some(),
                prepared.air_shadow.is_some(),
            ),
            cloud: match prepared.cloud.as_ref() {
                Some(field) if field.gpu_target().is_some() => crate::shaders::CloudVariant::Gpu,
                Some(_) => crate::shaders::CloudVariant::Sampled,
                None => crate::shaders::CloudVariant::Absent,
            },
            shadows: prepared.shadow.is_some().into(),
            sun: crate::capabilities::SunShadowMethod::Maps,
            traced_lamps: false,
            soft_sun: false,
        };
        let (source, shader) = crate::shaders::Shader::SecondaryScene(variant)
            .compile_with_source(device, PASS, false);
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
                    entry_point: Some(
                        if prepared.bucket_route.is_some() || prepared.visibility.is_some() {
                            "compacted_vertex"
                        } else {
                            "vertex"
                        },
                    ),
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
        let aerial = prepared
            .atmosphere
            .as_ref()
            .map(|world| world.secondary_aerial(device, queue));
        let lighting = lighting::Lighting::new(
            prepared,
            queue,
            sources,
            aerial
                .as_ref()
                .map(crate::atmosphere::SecondaryAerial::sampled),
        )?;
        let culling = prepared.visibility.as_ref().map(|world| {
            (
                crate::visibility::SecondaryVisibility::new(world, device, &handle.record),
                world.member_target(
                    device,
                    (
                        &view,
                        prepared.instance_upload.buffer(),
                        prepared.history.serial_buffer(),
                    ),
                    prepared
                        .bucket_route
                        .as_ref()
                        .map(super::BucketRoute::member_inputs),
                ),
            )
        });
        Ok(Self {
            handle,
            queue: queue.clone(),
            view,
            aux: buffer("secondary raster auxiliary", super::AUX_BYTES as u64),
            layout,
            pipelines,
            source: source.into(),
            key: PipelineRole::Secondary {
                view: index,
                stage: Stage::Opaque,
            }
            .key(prepared.key),
            lighting,
            aerial,
            sun: prepared
                .shadow
                .as_ref()
                .map(|world| sun::Sun::new(world, prepared, queue))
                .transpose()
                .map_err(|_| SecondaryViewError::RecordLimit)?,
            culling,
        })
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        main: &[wgpu::BindGroupEntry<'_>],
        route: Option<&super::mesh_route::TargetMembers>,
    ) -> wgpu::BindGroup {
        let mut entries = main.to_vec();
        if let Some(sun) = &self.sun {
            let mut height = main
                .iter()
                .find(|entry| entry.binding == 3)
                .expect("main Sun maps")
                .clone();
            height.binding = 7;
            entries.push(height);
            for entry in &mut entries {
                match entry.binding {
                    2 => entry.resource = sun.prepared.sample_buffer.as_entire_binding(),
                    3 => {
                        entry.resource = wgpu::BindingResource::TextureView(&sun.target.array_view);
                    }
                    _ => (),
                }
            }
        }
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
                    if let Some((_, target)) = &self.culling {
                        let size = route.map(|route| route.segment_bytes);
                        entry.resource = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: target.visible(),
                            offset: 0,
                            size: size.and_then(wgpu::BufferSize::new),
                        });
                    } else if let Some(route) = route {
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
        visibility: Option<&crate::visibility::VisibilityPrepared>,
    ) -> Result<(Resources, Vec<Access>), RenderError> {
        let imported = ResourceScope::Imported { initialized: true };
        let texture = |graph: &mut FrameGraph, texture: &wgpu::Texture| {
            graph.resource(ResourceDescriptor::of_texture(texture), imported)
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
        if self.culling.is_some() {
            let key = self.stage_key(index, Stage::Visibility);
            keys.insert(key);
            let mut reads = shared.to_vec();
            reads.push(Access::new(
                record.clone(),
                0..RECORD_BYTES,
                AccessKind::UniformRead,
            ));
            // The descriptor shape is the same as the main visibility buffers;
            // the physical outputs and tests belong to this slot.
            crate::visibility::SecondaryVisibility::append(
                visibility.ok_or(RenderError::GraphPassMismatch)?,
                graph,
                key,
                index,
                reads,
                &mut accesses,
            )?;
        }
        let sun = self
            .sun
            .as_ref()
            .map(|sun| sun.append(graph, keys, self.key, index, shared, &mut accesses))
            .transpose()?;
        self.lighting.append(
            graph,
            keys,
            self.stage_key(index, Stage::Clusters),
            index,
            shared,
            &mut accesses,
        )?;
        if let Some(aerial) = &self.aerial {
            aerial.append(
                graph,
                keys,
                self.stage_key(index, Stage::Aerial),
                index,
                shared,
                &mut accesses,
            )?;
        }
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
        graph.declare(
            PassKey::Secondary {
                view: crate::pass_key::pass_index(index),
                stage: SecondaryStage::Opaque,
            },
            PassBody {
                scope: PassScope::View(index as u64 + 2),
                pipeline: Some(self.key),
                accesses,
                external_side_effect: true,
            },
        );
        let reads = vec![
            Access::new(colour.clone(), 0..1, AccessKind::SampledRead),
            Access::new(depth.clone(), 0..1, AccessKind::SampledRead),
            Access::new(record, 0..RECORD_BYTES, AccessKind::UniformRead),
        ];
        Ok((Resources { colour, depth, sun }, reads))
    }

    fn upload(
        &self,
        request: &SecondaryViewRequest,
        view: &SyntheticView,
        exposure: f32,
    ) -> SecondaryViewRecord {
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
        record
    }
}

impl SecondaryPrepared {
    pub(super) fn update_caster_meshes(
        &self,
        queue: &wgpu::Queue,
        meshes: &[crate::visibility::MeshRecord],
    ) {
        if let Some((culling, _)) = self.sun.as_ref().and_then(|sun| sun.culling.as_ref()) {
            culling.update_meshes(queue, meshes);
        }
    }

    pub(super) fn set_caster_member_groups(&mut self, groups: u32) {
        if let Some((culling, _)) = self.sun.as_mut().and_then(|sun| sun.culling.as_mut()) {
            culling.set_member_groups(groups);
        }
    }

    pub(super) fn surface<'a>(
        &'a self,
        index: usize,
        prepared: &'a PreparedSynthetic,
    ) -> crate::contribution::SecondarySceneSurface<'a> {
        crate::contribution::SecondarySceneSurface {
            index,
            shader_source: &self.source,
            view_layout: &self.layout,
            light_layout: &prepared.map_light_layout,
            material_layout: &prepared.surface.material_layout,
            fragment_entry: if self.sun.is_some() {
                "secondary_shadowed_fragment"
            } else {
                "secondary_fragment"
            },
            colour_format: SCENE_FORMAT,
            depth_format: DEPTH_FORMAT,
            shadow: self.sun.as_ref().map(|sun| sun.prepared.surface()),
        }
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

impl PreparedSynthetic {
    pub(super) fn upload_secondary_views(
        &self,
    ) -> (
        [crate::contribution::SecondaryViewFrame; SecondaryViewSlot::MAX_COUNT],
        usize,
    ) {
        let mut frames =
            [crate::contribution::SecondaryViewFrame::default(); SecondaryViewSlot::MAX_COUNT];
        let mut count = 0;
        for (index, slot) in self.secondary.iter().enumerate() {
            let Some(request) = slot.handle.asked() else {
                continue;
            };
            let camera = SecondaryPrepared::camera(self, &request);
            let record = slot.upload(&request, &camera, f32::from_bits(self.exposure_bits));
            frames[count] = crate::contribution::SecondaryViewFrame {
                index,
                record,
                shadow_count: request.cascades,
                shadow_reach: request.shadow_reach,
            };
            count += 1;
        }
        (frames, count)
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
            Stage::Casters => self.secondary_casters(index, compiled),
            Stage::Visibility => {
                let prepared = self.prepared;
                let slot = &prepared.secondary[index];
                let Some(_) = slot.handle.asked() else {
                    return Ok(());
                };
                let (culling, target) = slot
                    .culling
                    .as_ref()
                    .ok_or(RenderError::GraphPassMismatch)?;
                culling
                    .encode(
                        prepared
                            .visibility
                            .as_ref()
                            .ok_or(RenderError::GraphPassMismatch)?,
                        target,
                        self.encoder,
                        self.generation,
                        compiled,
                        self.timings.as_deref_mut(),
                    )?
                    .then_some(())
                    .ok_or(RenderError::GraphPassMismatch)
            }
            Stage::SunNear | Stage::SunFar => {
                self.secondary_sun(index, usize::from(matches!(stage, Stage::SunFar)), compiled)
            }
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
            Stage::Aerial => {
                let prepared = self.prepared;
                let slot = &prepared.secondary[index];
                let Some(request) = slot.handle.asked() else {
                    return Ok(());
                };
                let view = SecondaryPrepared::camera(prepared, &request);
                let mut observer = view.camera_position()[1];
                if request.air == ViewAir::Plane
                    && let Some(plane) = request.clip
                {
                    let equation = plane.equation();
                    let camera = view.camera_position();
                    let side = equation[0] * camera[0]
                        + equation[1] * camera[1]
                        + equation[2] * camera[2]
                        + equation[3];
                    if side < 0.0 {
                        observer = -equation[3] / equation[1];
                    }
                }
                let aerial = slot.aerial.as_ref().ok_or(RenderError::GraphPassMismatch)?;
                let world = prepared
                    .atmosphere
                    .as_ref()
                    .ok_or(RenderError::GraphPassMismatch)?;
                aerial.update(&slot.queue, world, &view, observer);
                aerial.encode(
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
        if let Some(sun) = &slot.sun {
            sun.prepared.write_secondary(
                prepared
                    .shadow
                    .as_ref()
                    .ok_or(RenderError::GraphPassMismatch)?,
                &request.camera,
                (request.cascades, request.shadow_reach),
                self.contribution
                    .as_deref()
                    .and_then(crate::contribution::SceneContribution::shadow_bounds),
            )?;
        }
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
                slot.culling.as_ref().map(|(_, target)| target.args()),
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
            if let Some((_, target)) = &slot.culling {
                pass.draw_indexed_indirect(target.args(), 0);
            } else {
                pass.draw_indexed(
                    prepared.cube_draw.0.clone(),
                    prepared.cube_draw.1,
                    0..prepared.instance_upload.active_count(),
                );
            }
        }
        if let Some(contribution) = self
            .contribution
            .as_deref()
            .filter(|entry| entry.secondary_views_enabled())
        {
            let offsets: &[u32] = if prepared.bucket_route.is_some() {
                &[0, 0]
            } else {
                &[0]
            };
            pass.set_bind_group(0, &self.target.secondary[index], offsets);
            contribution.draw_secondary(index, &mut pass);
        }
        Ok(())
    }
}

pub(crate) fn composed_source(variant: crate::shaders::SceneVariant) -> String {
    super::scene_source::cloud_variant(
        shader_source(&super::scene_source::secondary(
            variant.atmosphere.present(),
            variant.atmosphere.air_shadow(),
            variant.cloud.present(),
            variant.shadows,
        )),
        variant.cloud.gpu(),
    )
}

#[cfg(test)]
mod tests;
