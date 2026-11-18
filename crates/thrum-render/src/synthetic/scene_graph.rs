//! Compiles and binds the prepared scene and capture graph.

use super::{
    AUX_BYTES, CONTRIBUTION_UPLOAD, CaptureSource, DEPTH_FORMAT, ExposureResolve, LIGHT_BYTES,
    MOTION_FORMAT, PreToneInput, PreparedSynthetic, RenderError, SCENE_FORMAT, SceneFamily,
    SceneGraphs, TargetResources, VIEW_BYTES, air_shadow, camera_culling, exposure, history_gpu,
    shadow,
};
use crate::graph::{
    Access, AccessKind, CompiledGraph, FrameGraph, GraphError, Pass, PassKind, PassScope,
    PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::graph_record::{ExecutableGraph, RecordBinding};
use crate::visibility::VisibilityPrepared;
use std::collections::BTreeSet;

pub(super) struct CompiledScene {
    pub(super) day: SceneGraphs<CompiledGraph>,
    pub(super) night: Option<SceneGraphs<CompiledGraph>>,
    pub(super) ids: TargetResources,
}

struct FrameEnvironment {
    directional: ResourceId,
    view: ResourceId,
    atmosphere_inputs: Option<(ResourceId, ResourceId)>,
    late_uniforms: Vec<Access>,
}

impl SceneGraphs<CompiledGraph> {
    fn compile_night(
        graph: &mut FrameGraph,
        stars: Option<usize>,
        readback: Pass,
        capabilities: &crate::capabilities::DeviceCapabilities,
        keys: &BTreeSet<PipelineKey>,
        capture: CaptureSource,
        surface: Option<&(ResourceId, Pass)>,
        consumers: [Option<usize>; 4],
    ) -> Result<Option<Self>, RenderError> {
        let Some(stars) = stars else {
            return Ok(None);
        };
        for consumer in consumers.into_iter().flatten() {
            graph.dependency(stars, consumer)?;
        }
        graph.validate_capabilities(capabilities)?;
        Ok(Some(Self::compile(
            graph,
            readback,
            capabilities.generation(),
            keys,
            (capture == CaptureSource::SceneLinear).then_some(stars),
            surface,
        )?))
    }

    fn compile(
        graph: &mut FrameGraph,
        readback: Pass,
        generation: u64,
        keys: &BTreeSet<PipelineKey>,
        producer: Option<usize>,
        surface: Option<&(ResourceId, Pass)>,
    ) -> Result<Self, GraphError> {
        let mut compile = |readback: Option<Pass>| {
            graph.compile_surface_variant(readback, generation, keys, producer, surface.cloned())
        };
        Ok(Self {
            scene: compile(None)?,
            readback: compile(Some(readback))?,
        })
    }

    pub(super) fn bind(
        self,
        prepared: &PreparedSynthetic,
    ) -> Result<SceneGraphs<ExecutableGraph<SceneFamily>>, RenderError> {
        Ok(SceneGraphs {
            scene: prepared.bind_scene_graph(self.scene)?,
            readback: prepared.bind_scene_graph(self.readback)?,
        })
    }
}

/// Role of a cleared full-view attachment in the forward pass.
#[derive(Clone, Copy)]
enum AttachmentUse {
    Depth,
    Scene,
    Motion,
    Reactive,
}

impl PreparedSynthetic {
    pub(super) fn check_owner(&self, target: &super::OffscreenTarget) -> Result<(), RenderError> {
        if !std::sync::Arc::ptr_eq(&self.owner, &target.prepared_owner)
            || target.generation != self.key.device_generation
        {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }

    pub(super) fn check_target(&self, target: &super::OffscreenTarget) -> Result<(), RenderError> {
        self.check_owner(target)?;
        if target.frame_revision != self.frame_revision {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }

    /// Inspect the prepared scene or its scene-plus-readback graph.
    /// # Errors
    /// Refuses a stale target generation or invalid typed resource/pass declarations.
    pub fn graph(
        &self,
        target: &super::OffscreenTarget,
        readback: bool,
    ) -> Result<CompiledGraph, RenderError> {
        self.check_target(target)?;
        Ok(target.graph(readback).plan().clone())
    }

    fn append_forward_epilogues(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward: usize,
        stats: &[ResourceId],
        (instance, history_write, history_bytes): (ResourceId, ResourceId, u64),
        simulate: &[usize],
    ) -> Result<(), RenderError> {
        for &pass in simulate {
            graph.dependency(pass, forward)?;
        }
        let bytes = self.visibility.as_ref().map_or(
            crate::visibility::ARGS_BYTES,
            VisibilityPrepared::args_bytes,
        );
        Self::append_camera_stats(graph, stats, bytes)?;
        self.append_history_publish(graph, keys, forward, instance, history_write, history_bytes)
    }
    /// Declares one cleared full-view attachment and its forward access.
    fn view_attachment(
        (width, height, graph, accesses): (u32, u32, &mut FrameGraph, &mut Vec<Access>),
        format: wgpu::TextureFormat,
        role: AttachmentUse,
        sampled_depth: bool,
        capture_source: CaptureSource,
        native: bool,
    ) -> Result<ResourceId, RenderError> {
        let usage = match role {
            AttachmentUse::Depth => {
                let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
                if sampled_depth {
                    usage | wgpu::TextureUsages::TEXTURE_BINDING
                } else {
                    usage
                }
            }
            AttachmentUse::Scene => {
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | if capture_source == CaptureSource::SceneLinear || native {
                        wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::empty()
                    }
            }
            AttachmentUse::Reactive => {
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | if sampled_depth {
                        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::empty()
                    }
            }
            AttachmentUse::Motion => {
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
            }
        };
        let resource = graph.resource(
            ResourceDescriptor::Texture {
                dimension: crate::graph::TextureKind::D2,
                width,
                height,
                layers: 1,
                mips: 1,
                samples: 1,
                format,
                usage,
            },
            ResourceScope::Transient,
        )?;
        accesses.push(Access {
            view: None,
            resource: resource.clone(),
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            store: !matches!(role, AttachmentUse::Depth | AttachmentUse::Reactive),
        });
        Ok(resource)
    }

    /// The arena's vertex (storage) and index reads, shared by every camera
    /// pass that draws meshes.
    fn mesh_arena_reads(&self, graph: &mut FrameGraph) -> Result<[Access; 2], RenderError> {
        let buffers = self
            .mesh_arena
            .buffers()
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut read = |buffer: &wgpu::Buffer, usage, kind| {
            let bytes = buffer.size();
            graph
                .resource(
                    ResourceDescriptor::Buffer {
                        bytes,
                        usage: usage | (buffer.usage() & wgpu::BufferUsages::BLAS_INPUT),
                    },
                    ResourceScope::Imported { initialized: true },
                )
                .map(|resource| Access::new(resource, 0..bytes, kind))
        };
        Ok([
            read(
                &buffers.vertices,
                wgpu::BufferUsages::STORAGE,
                AccessKind::StorageRead,
            )?,
            read(
                &buffers.indices,
                wgpu::BufferUsages::INDEX,
                AccessKind::IndexRead,
            )?,
        ])
    }

    /// The bucket route's member list (storage) and bucket table (uniform).
    fn bucket_route_reads(&self, graph: &mut FrameGraph) -> Result<Vec<Access>, RenderError> {
        let Some(route) = &self.bucket_route else {
            return Ok(Vec::new());
        };
        let inputs = route.member_inputs();
        [
            (
                inputs.members,
                wgpu::BufferUsages::STORAGE,
                AccessKind::StorageRead,
            ),
            (
                inputs.table,
                wgpu::BufferUsages::UNIFORM,
                AccessKind::UniformRead,
            ),
        ]
        .into_iter()
        .map(|(buffer, usage, kind)| {
            let bytes = buffer.size();
            let resource = graph.resource(
                ResourceDescriptor::Buffer { bytes, usage },
                ResourceScope::Imported { initialized: true },
            )?;
            Ok(Access::new(resource, 0..bytes, kind))
        })
        .collect()
    }

    fn instance_material_resources(
        &self,
        graph: &mut FrameGraph,
    ) -> Result<(ResourceId, ResourceId), RenderError> {
        let material = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.material_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let instance = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.instance_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        Ok((material, instance))
    }

    fn geometry_accesses(
        &self,
        material: ResourceId,
        instance: &ResourceId,
        history_read: ResourceId,
        history_bytes: u64,
        mesh: &[Access; 2],
        route: &[Access],
    ) -> Vec<Access> {
        let mut accesses = vec![
            Access::new(material, 0..self.material_bytes, AccessKind::StorageRead),
            Access::new(
                instance.clone(),
                0..self.instance_bytes,
                AccessKind::StorageRead,
            ),
            Access::new(history_read, 0..history_bytes, AccessKind::StorageRead),
        ];
        accesses.extend_from_slice(mesh);
        // Direct's bucket draws read the member list; the culled draws read
        // the visible list the compaction writes from it.
        if self.visibility.is_none() {
            accesses.extend(route.first().cloned());
        }
        accesses
    }

    pub(super) fn compile_graph(
        &self,
        width: u32,
        height: u32,
        row_bytes: u32,
        capture_source: CaptureSource,
        profile: Option<crate::present::SurfaceProfile>,
    ) -> Result<CompiledScene, RenderError> {
        let mut graph = FrameGraph::default();
        let (material, instance) = self.instance_material_resources(&mut graph)?;
        let (history_read, history_write, history_bytes) = self.history_resources(&mut graph)?;
        let mesh = self.mesh_arena_reads(&mut graph)?;
        let route = self.bucket_route_reads(&mut graph)?;
        let mut accesses = self.geometry_accesses(
            material,
            &instance,
            history_read,
            history_bytes,
            &mesh,
            &route,
        );
        let mut prepared_keys = BTreeSet::from([self.key, self.display_key(), self.exposure_key()]);
        let FrameEnvironment {
            directional,
            view,
            atmosphere_inputs,
            mut late_uniforms,
        } = self.append_frame_environment(&mut graph, &mut prepared_keys, &mut accesses)?;
        self.append_ray_structure(&mut graph, &mesh)?;
        let contribution_start = accesses.len();
        let effect_resources =
            self.append_contribution(&mut graph, &mut accesses, &mut prepared_keys)?;
        let contribution_reads = accesses[contribution_start..].to_vec();
        let shadow = self.append_shadow_passes(
            &mut graph,
            &instance,
            (&mesh, &route),
            &mut accesses,
            &mut prepared_keys,
            &contribution_reads,
        )?;
        let air_shadow = self.append_air_shadow(
            &mut graph,
            &mut prepared_keys,
            shadow.as_ref(),
            &directional,
            &mut accesses,
        )?;
        self.append_cluster_reads(&mut graph, &mut accesses, &mut prepared_keys)?;
        self.append_material_textures(&mut graph, &mut accesses)?;
        let secondary_shared = self.secondary_inputs(&accesses, &route);
        let [depth, scene, motion, reactive] =
            self.forward_attachments(&mut graph, &mut accesses, [width, height], capture_source)?;
        let camera = self.append_camera_visibility(
            &mut graph,
            (view, &instance, &mesh, &route),
            [width, height],
            &mut accesses,
            &mut prepared_keys,
        )?;
        let exposure_sample =
            ExposureResolve::append_graph(&mut graph, self.exposure_key(), directional)?;
        accesses.push(Access::new(
            exposure_sample.clone(),
            0..exposure::SAMPLE_BYTES,
            AccessKind::UniformRead,
        ));
        late_uniforms.extend(accesses.last().cloned());
        let secondary = self.append_secondary_consumers(
            &mut graph,
            &mut prepared_keys,
            secondary_shared,
            &exposure_sample,
            &mut accesses,
            &mut late_uniforms,
        )?;
        let forward = graph.pass(Pass {
            name: "synthetic forward opaque".into(),
            scope: PassScope::View(1),
            pipeline: Some(self.key),
            accesses,
            external_side_effect: true,
        });
        self.append_forward_epilogues(
            &mut graph,
            &mut prepared_keys,
            forward,
            &camera.stats,
            (instance, history_write, history_bytes),
            &effect_resources.simulate,
        )?;
        // Late writers need a stable colour predecessor. Prepare the optional star
        // pass before them; it records only when this frame selects stars. Empty
        // effect declarations retain the original day/night graph variants below.
        let early_stars = if self.effect_graph.after_opaque.is_empty() {
            None
        } else {
            self.append_optional_stars(
                &mut graph,
                &mut prepared_keys,
                forward,
                atmosphere_inputs.as_ref(),
                (&depth, &scene, &motion, &reactive, &exposure_sample),
            )?
        };
        let late = self.append_after_opaque(
            &mut graph,
            &mut prepared_keys,
            [&depth, &scene, &motion, &reactive],
            [width, height],
            &effect_resources,
            &late_uniforms,
            early_stars.unwrap_or(forward),
        )?;
        let temporal = self.append_temporal(
            &mut graph,
            &mut prepared_keys,
            [&depth, &motion, &reactive],
            &exposure_sample,
            [width, height],
        )?;
        let native = self
            .native
            .as_ref()
            .map(|_| super::native_witness::graph::append(&mut graph, &scene, [width, height]))
            .transpose()?;
        let taau = self.append_taau(
            &mut graph,
            &mut prepared_keys,
            [&scene, &depth, &motion, &reactive],
            &exposure_sample,
            [width, height],
        )?;
        late.order_consumers(
            &mut graph,
            [
                native.as_ref().map(|native| native.pass),
                taau.as_ref().map(|taau| taau.pass),
            ],
        )?;
        let pre_tone = self.pre_tone_input(
            (&scene, &motion, &exposure_sample),
            &temporal,
            taau.as_ref(),
            native.as_ref(),
            [width, height],
        )?;
        let (output, display_pass) = self.display.append_graph(&mut graph, &pre_tone)?;
        late.order_consumers(&mut graph, [Some(display_pass), temporal.witness_pass])?;
        let source = Self::capture_input(capture_source, &output, &scene, native.as_ref())?;
        let readback =
            Self::readback_pass(&mut graph, source, &exposure_sample, height, row_bytes)?;
        let generation = self.key.device_generation;
        let surface =
            Self::surface_pass(&mut graph, &output, generation, &mut prepared_keys, profile)?;
        graph.validate_capabilities(&self.capabilities)?;
        let day = SceneGraphs::compile(
            &mut graph,
            readback.clone(),
            generation,
            &prepared_keys,
            (capture_source == CaptureSource::SceneLinear).then_some(late.colour),
            surface.as_ref(),
        )?;
        let stars = if early_stars.is_some() {
            None
        } else {
            self.append_optional_stars(
                &mut graph,
                &mut prepared_keys,
                forward,
                atmosphere_inputs.as_ref(),
                (&depth, &scene, &motion, &reactive, &exposure_sample),
            )?
        };
        let night = SceneGraphs::compile_night(
            &mut graph,
            stars,
            readback,
            &self.capabilities,
            &prepared_keys,
            capture_source,
            surface.as_ref(),
            [
                native.as_ref().map(|native| native.pass),
                Some(display_pass),
                temporal.witness_pass,
                taau.as_ref().map(|taau| taau.pass),
            ],
        )?;
        Ok(CompiledScene {
            day,
            night,
            ids: TargetResources {
                secondary,
                native,
                refraction: late.copy,
                pre_tone,
                temporal,
                taau,
                depth,
                scene,
                motion,
                reactive,
                output,
                shadow,
                air_shadow,
                prepass_depth: camera.prepass_depth,
            },
        })
    }

    fn capture_input<'a>(
        source: CaptureSource,
        output: &'a ResourceId,
        scene: &'a ResourceId,
        native: Option<&'a super::native_witness::graph::Resources>,
    ) -> Result<&'a ResourceId, RenderError> {
        Ok(match source {
            CaptureSource::Sdr => output,
            CaptureSource::SceneLinear => scene,
            CaptureSource::NativeOutput => &native.ok_or(RenderError::GraphPassMismatch)?.output,
            CaptureSource::NativeHistory => &native.ok_or(RenderError::GraphPassMismatch)?.next,
        })
    }

    fn pre_tone_input(
        &self,
        (scene, motion, exposure): (&ResourceId, &ResourceId, &ResourceId),
        temporal: &super::temporal::TemporalResources,
        taau: Option<&super::taau::TaauResources>,
        native: Option<&super::native_witness::graph::Resources>,
        extent: [u32; 2],
    ) -> Result<PreToneInput, RenderError> {
        let colour = if let Some(native) = native {
            native.output.clone()
        } else if self
            .pending_view
            .unwrap_or(self.view_sample)
            .temporal_diagnostic()
        {
            temporal
                .witness
                .clone()
                .ok_or(RenderError::GraphPassMismatch)?
        } else if let Some(taau) = taau {
            taau.output.clone()
        } else {
            scene.clone()
        };
        Ok(PreToneInput {
            colour,
            motion: motion.clone(),
            exposure: exposure.clone(),
            extent,
        })
    }

    fn forward_attachments(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
        [width, height]: [u32; 2],
        source: CaptureSource,
    ) -> Result<[ResourceId; 4], RenderError> {
        let source = if self
            .effect_graph
            .after_opaque
            .iter()
            .any(|pass| pass.refraction)
        {
            CaptureSource::SceneLinear
        } else {
            source
        };
        let mut attachment = |format, role, sampled| {
            Self::view_attachment(
                (width, height, graph, accesses),
                format,
                role,
                sampled,
                source,
                self.native.is_some(),
            )
        };
        Ok([
            attachment(
                DEPTH_FORMAT,
                AttachmentUse::Depth,
                self.stars.is_some()
                    || self.temporal_inputs
                    || self.effect_graph.after_opaque.iter().any(|pass| pass.depth),
            )?,
            attachment(SCENE_FORMAT, AttachmentUse::Scene, false)?,
            attachment(MOTION_FORMAT, AttachmentUse::Motion, false)?,
            attachment(
                super::REACTIVE_FORMAT,
                AttachmentUse::Reactive,
                self.temporal_inputs,
            )?,
        ])
    }

    fn surface_pass(
        graph: &mut FrameGraph,
        output: &ResourceId,
        generation: u64,
        prepared_keys: &mut BTreeSet<PipelineKey>,
        profile: Option<crate::present::SurfaceProfile>,
    ) -> Result<Option<(ResourceId, Pass)>, GraphError> {
        profile
            .map(|profile| {
                prepared_keys.insert(crate::present::pipeline_key(generation));
                crate::present::surface_pass(graph, output, profile, generation)
            })
            .transpose()
    }

    fn append_optional_stars(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward: usize,
        atmosphere: Option<&(ResourceId, ResourceId)>,
        (depth, scene, motion, reactive, exposure): (
            &ResourceId,
            &ResourceId,
            &ResourceId,
            &ResourceId,
            &ResourceId,
        ),
    ) -> Result<Option<usize>, RenderError> {
        if let Some((transmittance, observer)) = atmosphere {
            self.append_stars(
                graph,
                keys,
                forward,
                (
                    depth,
                    scene,
                    motion,
                    reactive,
                    exposure,
                    transmittance,
                    observer,
                ),
            )
        } else {
            Ok(None)
        }
    }

    fn append_frame_uniform_reads(
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
    ) -> Result<(ResourceId, ResourceId), GraphError> {
        let directional = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: LIGHT_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            directional.clone(),
            0..LIGHT_BYTES as u64,
            AccessKind::UniformRead,
        ));
        let view = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: VIEW_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            view.clone(),
            0..VIEW_BYTES as u64,
            AccessKind::UniformRead,
        ));
        let aux = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: AUX_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            aux,
            0..AUX_BYTES as u64,
            AccessKind::UniformRead,
        ));
        Ok((directional, view))
    }

    fn history_resources(
        &self,
        graph: &mut FrameGraph,
    ) -> Result<(ResourceId, ResourceId, u64), RenderError> {
        let bytes = u64::from(self.instance_count) * history_gpu::ENTRY_BYTES;
        let descriptor = ResourceDescriptor::Buffer {
            bytes,
            usage: wgpu::BufferUsages::STORAGE,
        };
        let scope = ResourceScope::History {
            owner: 1,
            epoch: self.key.device_generation,
            initialized: true,
        };
        Ok((
            graph.resource(descriptor.clone(), scope)?,
            graph.resource(descriptor, scope)?,
            bytes,
        ))
    }

    fn append_history_publish(
        &self,
        graph: &mut FrameGraph,
        prepared_keys: &mut BTreeSet<PipelineKey>,
        forward: usize,
        instance: ResourceId,
        history_write: ResourceId,
        history_bytes: u64,
    ) -> Result<(), RenderError> {
        prepared_keys.insert(self.history_key());
        let publish = graph.pass(Pass {
            name: history_gpu::PASS_NAME.into(),
            scope: PassScope::View(1),
            pipeline: Some(self.history_key()),
            accesses: vec![
                Access::new(instance, 0..self.instance_bytes, AccessKind::StorageRead),
                Access::new(history_write, 0..history_bytes, AccessKind::StorageWrite),
            ],
            external_side_effect: false,
        });
        graph.dependency(forward, publish)?;
        Ok(())
    }

    fn readback_pass(
        graph: &mut FrameGraph,
        output: &ResourceId,
        exposure: &ResourceId,
        height: u32,
        row_bytes: u32,
    ) -> Result<Pass, RenderError> {
        let bytes = u64::from(row_bytes) * u64::from(height) + exposure::SAMPLE_BYTES;
        let destination = graph.resource(
            ResourceDescriptor::Buffer {
                bytes,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        Ok(Pass {
            name: "output readback".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![
                Access::new(
                    exposure.clone(),
                    0..exposure::SAMPLE_BYTES,
                    AccessKind::CopyRead,
                ),
                Access::new(output.clone(), 0..1, AccessKind::CopyRead),
                Access::new(destination, 0..bytes, AccessKind::CopyWrite),
            ],
            external_side_effect: true,
        })
    }

    fn append_material_textures(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
    ) -> Result<(), GraphError> {
        for class in self.classes.arrays() {
            let texture = graph.resource(
                ResourceDescriptor::Texture {
                    dimension: crate::graph::TextureKind::D2,
                    width: 1,
                    height: 1,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: class.class.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access::new(texture, 0..1, AccessKind::SampledRead));
        }
        Ok(())
    }

    fn append_shadow_passes(
        &self,
        graph: &mut FrameGraph,
        instance: &ResourceId,
        (mesh, route): (&[Access], &[Access]),
        accesses: &mut Vec<Access>,
        prepared_keys: &mut BTreeSet<PipelineKey>,
        contribution_reads: &[Access],
    ) -> Result<Option<ResourceId>, RenderError> {
        if self.shadow.is_none() {
            return Ok(None);
        }

        let maps = graph.resource(
            ResourceDescriptor::Texture {
                dimension: crate::graph::TextureKind::D2,
                width: shadow::SIZE,
                height: shadow::SIZE,
                layers: shadow::LAYERS,
                mips: 1,
                samples: 1,
                format: shadow::FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | if cfg!(all(test, feature = "creation-counters")) {
                        wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::empty()
                    },
            },
            ResourceScope::Transient,
        )?;
        // Every caster pass interpolates translations by the history
        // serials' presentation alpha.
        let serials = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: history_gpu::SERIAL_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        // Each pass's caster view: light matrix and upstream clip flag.
        let matrices = (0..shadow::LAYERS)
            .map(|_| {
                graph.resource(
                    ResourceDescriptor::Buffer {
                        bytes: shadow::CASTER_VIEW_RANGE,
                        usage: wgpu::BufferUsages::UNIFORM,
                    },
                    ResourceScope::Imported { initialized: true },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let uniform = |resource: &ResourceId, bytes| {
            Access::new(resource.clone(), 0..bytes, AccessKind::UniformRead)
        };
        // Culled camera modes on the bucket route: one compaction lists each
        // pass's casters against that pass's own view, which it reads.
        let lists = match &self.caster_culling {
            Some((culling, _)) => {
                let mut cull = vec![
                    Access::new(
                        instance.clone(),
                        0..self.instance_bytes,
                        AccessKind::StorageRead,
                    ),
                    uniform(&serials, history_gpu::SERIAL_BYTES),
                ];
                cull.extend(
                    matrices
                        .iter()
                        .map(|matrix| uniform(matrix, shadow::CASTER_VIEW_RANGE)),
                );
                cull.extend_from_slice(route);
                let ids = culling.append_outputs(graph, &mut cull)?;
                let key = self.caster_cull_key();
                prepared_keys.insert(key);
                graph.pass(Pass {
                    name: crate::visibility::casters::PASS.into(),
                    scope: PassScope::View(1),
                    pipeline: Some(key),
                    accesses: cull,
                    external_side_effect: false,
                });
                Some(
                    ids.lists
                        .iter()
                        .map(|list| culling.list_reads(list))
                        .collect::<Vec<_>>(),
                )
            }
            None => None,
        };
        for (index, matrix) in (0..shadow::LAYERS).zip(matrices) {
            // Unculled casters read the route's member list; culled ones
            // read their own pass's list and records.
            let caster_reads: Vec<Access> = match &lists {
                Some(lists) => mesh
                    .iter()
                    .cloned()
                    .chain(lists[index as usize].iter().cloned())
                    .collect(),
                None => mesh.iter().chain(route.first()).cloned().collect(),
            };
            let key = self.shadow_key(index);
            prepared_keys.insert(key);
            graph.pass(Pass {
                name: match index {
                    0 => "synthetic Sun shadow near",
                    1 => "synthetic Sun shadow far",
                    _ => "synthetic Sun horizon heights",
                }
                .into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses: [
                    Access::new(
                        instance.clone(),
                        0..self.instance_bytes,
                        AccessKind::StorageRead,
                    ),
                    Access::new(
                        matrix,
                        0..shadow::CASTER_VIEW_RANGE,
                        AccessKind::UniformRead,
                    ),
                    Access::new(
                        serials.clone(),
                        0..history_gpu::SERIAL_BYTES,
                        AccessKind::UniformRead,
                    ),
                    Access::new(
                        maps.clone(),
                        u64::from(index)..u64::from(index + 1),
                        AccessKind::AttachmentClear,
                    ),
                ]
                .into_iter()
                .chain(caster_reads)
                .chain(contribution_reads.iter().cloned())
                .collect(),
                external_side_effect: false,
            });
        }
        let sample = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: shadow::SAMPLE_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            sample,
            0..shadow::SAMPLE_BYTES as u64,
            AccessKind::UniformRead,
        ));
        accesses.push(Access::new(
            maps.clone(),
            0..u64::from(shadow::LAYERS),
            AccessKind::SampledRead,
        ));
        Ok(Some(maps))
    }
    fn append_cluster_reads(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
        prepared_keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<(), RenderError> {
        let mut resources = Vec::with_capacity(3);
        for (index, bytes) in [
            self.local_light_bytes,
            self.cluster_head_bytes,
            self.cluster_index_bytes,
        ]
        .into_iter()
        .enumerate()
        {
            let storage = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::Imported {
                    initialized: index == 0 || self.cluster_compute.is_none(),
                },
            )?;
            accesses.push(Access::new(
                storage.clone(),
                0..bytes,
                AccessKind::StorageRead,
            ));
            resources.push(storage);
        }
        if self.cluster_compute.is_some() {
            let bounds = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: u64::try_from(self.clusters.bounds.len())
                        .map_err(|_| RenderError::SceneCapacity)?,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            prepared_keys.insert(self.cluster_key());
            graph.pass(Pass {
                name: "synthetic cluster build".into(),
                scope: PassScope::View(1),
                pipeline: Some(self.cluster_key()),
                accesses: vec![
                    Access {
                        view: None,
                        resource: bounds,
                        range: 0..u64::try_from(self.clusters.bounds.len())
                            .map_err(|_| RenderError::SceneCapacity)?,
                        kind: AccessKind::StorageRead,
                        store: true,
                    },
                    Access::new(
                        resources[1].clone(),
                        0..self.cluster_head_bytes,
                        AccessKind::StorageWrite,
                    ),
                    Access::new(
                        resources[2].clone(),
                        0..self.cluster_index_bytes,
                        AccessKind::StorageWrite,
                    ),
                ],
                external_side_effect: false,
            });
        }
        Ok(())
    }

    fn append_frame_environment(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        accesses: &mut Vec<Access>,
    ) -> Result<FrameEnvironment, RenderError> {
        let (directional, view) = Self::append_frame_uniform_reads(graph, accesses)?;
        let mut late_uniforms = accesses
            .iter()
            .filter(|a| a.kind == AccessKind::UniformRead)
            .cloned()
            .collect::<Vec<_>>();
        let start = accesses.len();
        let atmosphere_inputs = self.append_environment(graph, keys, accesses, &directional)?;
        // Late consumers borrow these same producers through EnvironmentProducts.
        late_uniforms.extend(accesses[start..].iter().cloned());
        Ok(FrameEnvironment {
            directional,
            view,
            atmosphere_inputs,
            late_uniforms,
        })
    }

    fn append_environment(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        accesses: &mut Vec<Access>,
        directional: &ResourceId,
    ) -> Result<Option<(ResourceId, ResourceId)>, RenderError> {
        if let Some(cloud) = &self.cloud {
            cloud.append_reads(graph, accesses)?;
        }
        Ok(self
            .atmosphere
            .as_ref()
            .map(|atmosphere| atmosphere.append_graph(graph, keys, accesses, directional.clone()))
            .transpose()?)
    }

    fn append_air_shadow(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        shadow: Option<&ResourceId>,
        directional: &ResourceId,
        accesses: &mut Vec<Access>,
    ) -> Result<Option<ResourceId>, RenderError> {
        let key = self.air_shadow_key();
        shadow
            .filter(|_| self.air_shadow.is_some())
            .map(|maps| {
                keys.insert(key);
                air_shadow::append_graph(graph, key, [maps.clone(), directional.clone()], accesses)
            })
            .transpose()
    }

    fn surface_binding(
        graph: &CompiledGraph,
        id: usize,
    ) -> Result<RecordBinding<SceneFamily>, RenderError> {
        let surface = graph
            .surface_resource()
            .ok_or(RenderError::GraphPassMismatch)?;
        Ok(RecordBinding::<SceneFamily>::new(
            id,
            PassKind::Render,
            move |frame, pass| {
                let (presenter, binding, view, extent) =
                    frame.surface.ok_or(RenderError::GraphPassMismatch)?;
                presenter
                    .encode_compiled(
                        frame.encoder,
                        binding,
                        view,
                        extent,
                        frame.timings.as_deref_mut(),
                        pass,
                        &surface,
                    )
                    .map_err(RenderError::Present)
            },
        ))
    }

    pub(super) fn bind_scene_graph(
        &self,
        graph: CompiledGraph,
    ) -> Result<ExecutableGraph<SceneFamily>, RenderError> {
        let mut bindings = Vec::with_capacity(graph.pass_order.len());
        for (id, pass) in graph.pass_bindings() {
            let binding = match pass.pipeline {
                Some(key) if super::secondary::index(key, self.secondary.len()).is_some() => {
                    let (index, stage) = super::secondary::index(key, self.secondary.len())
                        .ok_or(RenderError::GraphPassMismatch)?;
                    RecordBinding::<SceneFamily>::new(id, stage.kind(), move |frame, pass| {
                        frame.secondary_stage(index, stage, pass)
                    })
                }
                Some(key)
                    if key.role >= 3000
                        && key.role
                            < 3000
                                + u32::try_from(self.effect_graph.after_opaque.len())
                                    .unwrap_or(0) =>
                {
                    let index = (key.role - 3000) as usize;
                    let kind = match self.effect_graph.after_opaque[index].kind {
                        crate::contribution::AfterOpaqueKind::Render => PassKind::Render,
                        crate::contribution::AfterOpaqueKind::Compute => PassKind::Compute,
                    };
                    RecordBinding::<SceneFamily>::new(id, kind, move |frame, pass| {
                        frame.after_opaque(index, pass)
                    })
                }
                Some(key)
                    if key.role >= 2000
                        && key.role
                            < 2000 + u32::try_from(self.effect_graph.passes.len()).unwrap_or(0) =>
                {
                    let index = (key.role - 2000) as usize;
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, move |frame, pass| {
                        frame.effect(index, pass)
                    })
                }

                Some(key) if key == crate::present::pipeline_key(self.key.device_generation) => {
                    Self::surface_binding(&graph, id)?
                }
                Some(key)
                    if self
                        .atmosphere
                        .as_ref()
                        .is_some_and(|atmosphere| atmosphere.stage_index(key.role).is_some()) =>
                {
                    let index = self
                        .atmosphere
                        .as_ref()
                        .and_then(|atmosphere| atmosphere.stage_index(key.role))
                        .ok_or(RenderError::GraphPassMismatch)?;
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, move |frame, pass| {
                        frame.atmosphere(index, pass)
                    })
                }
                Some(key) if (0..shadow::LAYERS).any(|index| key == self.shadow_key(index)) => {
                    let index = (0..shadow::LAYERS)
                        .find(|&index| key == self.shadow_key(index))
                        .and_then(|index| usize::try_from(index).ok())
                        .ok_or(RenderError::GraphPassMismatch)?;
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, move |frame, pass| {
                        frame.shadow(index, pass)
                    })
                }
                Some(key) if self.caster_culling.is_some() && key == self.caster_cull_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.caster_cull(pass)
                    })
                }
                Some(key) if Some(key) == self.visibility.as_ref().map(VisibilityPrepared::key) => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.visibility(pass)
                    })
                }
                Some(key)
                    if Some(key) == self.visibility.as_ref().map(VisibilityPrepared::final_key) =>
                {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.final_visibility(pass)
                    })
                }
                Some(key)
                    if Some(key)
                        == self
                            .visibility
                            .as_ref()
                            .map(VisibilityPrepared::pyramid_key) =>
                {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.depth_pyramid(pass)
                    })
                }
                Some(key) if key == self.depth_prepass_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, |frame, pass| {
                        frame.depth_prepass(pass)
                    })
                }
                Some(key) if key == self.key => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, |frame, pass| {
                        frame.opaque(pass)
                    })
                }
                Some(key) if key == self.cluster_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.cluster(pass)
                    })
                }
                Some(key) if key == self.air_shadow_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, |frame, pass| {
                        frame.air_shadow(pass)
                    })
                }
                Some(key) if key == self.history_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.publish_history(pass)
                    })
                }
                Some(key) if key == self.temporal_key(false) || key == self.temporal_key(true) => {
                    let witness = key == self.temporal_key(true);
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, move |frame, pass| {
                        frame.temporal(pass, witness)
                    })
                }
                Some(key) if key == self.taau_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.taau(pass)
                    })
                }
                Some(key) if key == self.exposure_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.exposure(pass)
                    })
                }
                Some(key) if key == self.star_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, |frame, pass| {
                        frame.stars(pass)
                    })
                }
                Some(key) if key == self.display_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Render, |frame, pass| {
                        frame.tone(pass)
                    })
                }
                None if pass.kind == Some(PassKind::AccelerationBuild) => {
                    super::raytrace::binding(id)
                }
                None if pass.name == super::native_witness::RESET
                    || pass.name == super::native_witness::PASS =>
                {
                    Self::bind_native(id, pass.kind)
                }
                None if pass.kind == Some(PassKind::Copy)
                    && pass.name == camera_culling::STATS_PASS =>
                {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Copy, |frame, _| {
                        frame.culling_stats()
                    })
                }
                None if pass.kind == Some(PassKind::Copy) && pass.name == CONTRIBUTION_UPLOAD => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Copy, |frame, _| {
                        frame.contribution_upload();
                        Ok(())
                    })
                }
                Some(key) if key == self.contribution_key() => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Compute, |frame, pass| {
                        frame.contribution_compute(pass)
                    })
                }
                None if pass.kind == Some(PassKind::Copy)
                    && pass.name == super::after_opaque::COPY =>
                {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Copy, |frame, _| {
                        frame.refraction_copy()
                    })
                }
                None if pass.kind == Some(PassKind::Copy) => {
                    RecordBinding::<SceneFamily>::new(id, PassKind::Copy, |frame, pass| {
                        frame.copy(pass)
                    })
                }
                _ => return Err(RenderError::GraphPassMismatch),
            };
            bindings.push(binding);
        }
        graph.bind(bindings).map_err(RenderError::Graph)
    }

    fn bind_native(id: usize, kind: Option<PassKind>) -> RecordBinding<SceneFamily> {
        if kind == Some(PassKind::Native) {
            RecordBinding::<SceneFamily>::new(id, PassKind::Native, |frame, _| {
                let native = frame.native.ok_or(RenderError::GraphPassMismatch)?;
                native.encode(
                    &frame.target.device,
                    frame.encoder,
                    frame
                        .segments
                        .as_deref_mut()
                        .ok_or(RenderError::GraphPassMismatch)?,
                );
                Ok(())
            })
        } else {
            RecordBinding::<SceneFamily>::new(id, PassKind::Copy, |frame, _| {
                frame
                    .native
                    .ok_or(RenderError::GraphPassMismatch)?
                    .reset(frame.encoder);
                Ok(())
            })
        }
    }
}
