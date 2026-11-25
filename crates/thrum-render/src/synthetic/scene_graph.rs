//! Compiles and binds the prepared scene and capture graph.

use super::{
    AUX_BYTES, CaptureSource, DEPTH_FORMAT, DIRECTIONAL_BYTES, ExposureResolve, MOTION_FORMAT,
    PreToneInput, PreparedSynthetic, RenderError, SCENE_FORMAT, SceneFamily, SceneGraphs,
    TargetResources, VIEW_BYTES, air_shadow, exposure, history_gpu, shadow,
};
use crate::graph::PassBody;
use crate::graph::{
    Access, AccessKind, CompiledGraph, FrameGraph, GraphError, PassKind, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::graph_record::{ExecutableGraph, RecordBinding};
use crate::pass_key::{PassKey, PipelineRole};
use crate::sprite::interface;
use crate::visibility::VisibilityPrepared;
use std::collections::BTreeSet;

pub(crate) struct CompiledScene {
    pub(super) capture_layout: super::readback::CaptureLayout,
    pub(super) day: SceneGraphs<CompiledGraph>,
    pub(super) night: Option<SceneGraphs<CompiledGraph>>,
    pub(super) ids: TargetResources,
}

impl CompiledScene {
    pub(crate) fn capture_extent(&self) -> [u32; 2] {
        self.capture_layout.extent
    }

    /// The most timestamped passes any variant records.
    pub(super) fn pipeline_pass_count(&self) -> usize {
        [Some(&self.day), self.night.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|graphs| [&graphs.scene, &graphs.readback])
            .map(CompiledGraph::pipeline_pass_count)
            .max()
            .unwrap_or(0)
    }
}

struct FrameEnvironment {
    light: [ResourceId; 2],
    view: ResourceId,
    atmosphere_inputs: Option<(ResourceId, ResourceId)>,
    late_uniforms: Vec<Access>,
}

impl SceneGraphs<CompiledGraph> {
    fn compile_night(
        graph: &mut FrameGraph,
        stars: Option<usize>,
        readback: PassBody,
        capabilities: &crate::capabilities::DeviceCapabilities,
        keys: &BTreeSet<PipelineKey>,
        capture: CaptureSource,
        interface_pass: Option<usize>,
        surface: Option<&(ResourceId, PassBody)>,
        consumers: impl IntoIterator<Item = Option<usize>>,
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
            match capture {
                CaptureSource::SceneLinear => Some(stars),
                CaptureSource::Sdr => interface_pass,
                _ => None,
            },
            surface,
        )?))
    }

    fn compile(
        graph: &mut FrameGraph,
        readback: PassBody,
        generation: u64,
        keys: &BTreeSet<PipelineKey>,
        producer: Option<usize>,
        surface: Option<&(ResourceId, PassBody)>,
    ) -> Result<Self, GraphError> {
        let mut compile = |readback: Option<PassBody>| {
            graph.compile_surface_variant(
                readback.map(|body| (PassKey::Readback, body)),
                generation,
                keys,
                producer,
                surface.cloned(),
            )
        };
        Ok(Self {
            scene: compile(None)?,
            readback: compile(Some(readback))?,
        })
    }

    pub(super) fn bind(
        self,
        prepared: &super::SyntheticTargetInputs,
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
    AmbientReceiver,
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
}

impl super::SyntheticTargetInputs {
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
        PreparedSynthetic::append_camera_stats(graph, stats, bytes)?;
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
            AttachmentUse::AmbientReceiver => {
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
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
        let buffers = &self.mesh_buffers;
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

    #[expect(
        clippy::too_many_lines,
        reason = "graph assembly follows the full frame's dependency order"
    )]
    pub(crate) fn compile_graph(
        &self,
        width: u32,
        height: u32,
        capture_source: CaptureSource,
        profile: Option<crate::present::SurfaceProfile>,
    ) -> Result<CompiledScene, RenderError> {
        self.validate_target_extent([width, height])?;
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
        let geometry_end = accesses.len();
        let mut prepared_keys = BTreeSet::from([self.key, self.display_key(), self.exposure_key()]);
        let FrameEnvironment {
            light,
            view,
            atmosphere_inputs,
            mut late_uniforms,
        } = self.append_frame_environment(&mut graph, &mut prepared_keys, &mut accesses)?;
        let contribution_start = accesses.len();
        let (effect_resources, ray_structure) =
            self.append_contribution(&mut graph, &mut accesses, &mut prepared_keys, &mesh)?;
        let contribution_reads = accesses[contribution_start..].to_vec();
        let shadow = self.append_shadow_passes(
            &mut graph,
            &instance,
            (&mesh, &route),
            &mut accesses,
            &mut prepared_keys,
            &contribution_reads,
        )?;
        if let (Some(atmosphere), Some((_, uniform))) = (&self.atmosphere, &atmosphere_inputs) {
            atmosphere.append_view_volume(
                &mut graph,
                &mut prepared_keys,
                uniform,
                shadow
                    .as_ref()
                    .map(|maps| (maps, 2, shadow::SAMPLE_BYTES as u64)),
                self.cloud.as_ref(),
                effect_resources.sun_path_reads.as_ref(),
                effect_resources.medium_read.clone(),
                effect_resources.ground_band_read.as_ref(),
            )?;
        }
        let air_shadow = self.append_air_shadow(
            &mut graph,
            &mut prepared_keys,
            shadow.as_ref(),
            &light[0],
            &mut accesses,
        )?;
        self.append_cluster_reads(&mut graph, &mut accesses, &mut prepared_keys)?;
        self.append_material_textures(&mut graph, &mut accesses)?;
        let secondary_shared = self.secondary_inputs(&accesses, &route);
        if let Some(structure) = ray_structure.as_ref().filter(|_| self.uses_shadow_rays) {
            accesses.push(Access::new(structure.clone(), 0..1, AccessKind::RayQuery));
        }
        let [depth, scene, motion, reactive] =
            self.forward_attachments(&mut graph, &mut accesses, [width, height], capture_source)?;
        let camera_start = accesses.len();
        let camera = self.append_camera_visibility(
            &mut graph,
            (view.clone(), &instance, &mesh, &route),
            [width, height],
            &mut accesses,
            &mut prepared_keys,
        )?;
        let ambient_receiver = self
            .ambient
            .as_ref()
            .map(|_| {
                let mut receiver_accesses = accesses[..geometry_end].to_vec();
                receiver_accesses.extend(contribution_reads.iter().cloned());
                receiver_accesses.extend(accesses[camera_start..].iter().cloned());
                receiver_accesses.extend([
                    Access::new(view, 0..VIEW_BYTES as u64, AccessKind::UniformRead),
                    Access::new(depth.clone(), 0..1, AccessKind::DepthAttachmentRead),
                ]);
                let receiver = Self::view_attachment(
                    (width, height, &mut graph, &mut receiver_accesses),
                    super::raytrace::ambient::RECEIVER_FORMAT,
                    AttachmentUse::AmbientReceiver,
                    false,
                    capture_source,
                    false,
                )?;
                let key = PipelineRole::AmbientReceiver.key(self.key);
                prepared_keys.insert(key);
                let pass = graph.declare(
                    PassKey::AmbientReceiver,
                    PassBody {
                        scope: PassScope::View(1),
                        pipeline: Some(key),
                        accesses: receiver_accesses,
                        external_side_effect: false,
                    },
                );
                Ok::<_, RenderError>((receiver, pass))
            })
            .transpose()?;
        let mut incident_sun = Vec::new();
        if let (Some(cloud), Some((_, observer))) = (&self.cloud, &atmosphere_inputs)
            && cloud.band.is_some()
        {
            cloud.append_reads(
                &mut graph,
                &mut incident_sun,
                effect_resources.sun_path_reads.as_ref(),
                effect_resources.ground_band_read.as_ref(),
            )?;
            incident_sun.push(Access::new(
                observer.clone(),
                0..size_of::<crate::records::Atmosphere>() as u64,
                AccessKind::UniformRead,
            ));
        }
        let exposure_sample = ExposureResolve::append_graph(
            &mut graph,
            self.exposure_key(),
            light.clone(),
            incident_sun,
        )?;
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
        let mut ambient = ambient_receiver
            .as_ref()
            .map(|(receiver, _)| {
                self.append_ambient_visibility(
                    &mut graph,
                    &mut prepared_keys,
                    &mut accesses,
                    [width, height],
                    [&depth, &motion, receiver],
                    ray_structure.as_ref(),
                )
            })
            .transpose()?;
        let soft_sun = self.append_soft_visibility(
            &mut graph,
            &mut prepared_keys,
            &mut accesses,
            [width, height],
            &depth,
            &motion,
            &light[0],
            ray_structure.as_ref(),
            ambient
                .as_mut()
                .map(|ambient| std::mem::take(&mut ambient.combined_accesses)),
        )?;
        let forward = graph.declare(
            PassKey::Opaque,
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(self.key),
                accesses,
                external_side_effect: true,
            },
        );
        if let Some((_, receiver)) = &ambient_receiver {
            graph.dependency(forward, *receiver)?;
        }
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
        if let Some((_, passes)) = &soft_sun {
            if let Some(ambient) = &ambient {
                graph.dependency(passes[0], ambient.filter)?;
            }
            for pass in passes {
                graph.dependency(forward, *pass)?;
            }
            if let Some(first) = late.first {
                graph.dependency(passes[3], first)?;
            }
        }
        if let Some(ambient) = &ambient {
            if let Some(trace) = ambient.trace {
                graph.dependency(forward, trace)?;
            }
            if let Some(first) = late.first {
                graph.dependency(ambient.filter, first)?;
            }
        }
        let temporal = self.append_temporal(
            &mut graph,
            &mut prepared_keys,
            [&depth, &motion, &reactive],
            &exposure_sample,
            [width, height],
        )?;
        let attachments = [&scene, &depth, &motion, &reactive];
        let mut reconstruction = self.declare_taau(
            &mut graph,
            &mut prepared_keys,
            attachments,
            &exposure_sample,
            [width, height],
        )?;
        let use_native_reconstruction = self.view_sample.requests_metalfx();
        let native = self
            .native
            .as_ref()
            .map(|provider| {
                provider.declare(
                    &mut graph,
                    crate::apple_tile::NativeGraphInputs {
                        attachments,
                        exposure: &temporal.exposure,
                        reconstruction_twin: if use_native_reconstruction {
                            reconstruction.take()
                        } else {
                            None
                        },
                    },
                    [width, height],
                )
            })
            .transpose()?;
        let taau = if use_native_reconstruction {
            native.as_ref().map(|native| super::taau::TaauResources {
                output: native.output.clone(),
                pass: native.pass,
            })
        } else {
            reconstruction.map(|(output, body)| super::taau::TaauResources {
                output,
                pass: graph.declare(PassKey::Taau, body),
            })
        };
        late.order_consumers(
            &mut graph,
            [
                native.as_ref().map(|native| native.pass),
                taau.as_ref()
                    .filter(|_| !use_native_reconstruction)
                    .map(|taau| taau.pass),
            ],
        )?;
        let pre_tone = self.pre_tone_input(
            (&scene, &motion, &exposure_sample),
            &temporal,
            taau.as_ref(),
            native.as_ref(),
            [width, height],
        )?;
        let (output, display_pass) = self.display.append_graph(
            &mut graph,
            &pre_tone,
            ray_structure.filter(|_| self.traced_view),
        )?;
        late.order_consumers(&mut graph, [Some(display_pass), temporal.witness_pass])?;
        if let Some((_, passes)) = &soft_sun {
            graph.dependency(passes[3], display_pass)?;
        }
        if let Some(ambient) = ambient {
            graph.dependency(ambient.filter, display_pass)?;
        }
        let generation = self.key.device_generation;
        let interface_reads = self
            .interface_inputs
            .as_ref()
            .map(|inputs| inputs.reads(&mut graph))
            .transpose()?;
        let interface_pass = if let Some(reads) = &interface_reads {
            Some(interface::append_sdr(
                &mut graph,
                &mut prepared_keys,
                &output,
                reads.clone(),
                generation,
                display_pass,
                profile.is_none_or(|profile| profile.format != wgpu::TextureFormat::Rgba16Float),
            )?)
        } else {
            None
        };
        let source = Self::capture_input(capture_source, &output, &scene, native.as_ref())?;
        let Some(ResourceDescriptor::Texture { width, height, .. }) = graph.descriptor(source)
        else {
            return Err(RenderError::GraphPassMismatch);
        };
        let capture_layout = super::readback::CaptureLayout::new(
            [*width, *height],
            capture_source,
            self.capabilities.limits(),
        )?;
        let readback = Self::readback_pass(&mut graph, source, &exposure_sample, capture_layout)?;
        let surface = Self::surface_pass(
            &mut graph,
            &output,
            &pre_tone,
            generation,
            &mut prepared_keys,
            profile,
            interface_reads.as_deref().unwrap_or_default(),
        )?;
        graph.validate_capabilities(&self.capabilities)?;
        let day = SceneGraphs::compile(
            &mut graph,
            readback.clone(),
            generation,
            &prepared_keys,
            match capture_source {
                CaptureSource::SceneLinear => Some(late.colour),
                CaptureSource::Sdr => interface_pass,
                _ => None,
            },
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
            interface_pass,
            surface.as_ref(),
            [
                native.as_ref().map(|native| native.pass),
                Some(display_pass),
                temporal.witness_pass,
                taau.as_ref()
                    .filter(|_| !use_native_reconstruction)
                    .map(|taau| taau.pass),
            ],
        )?;
        Ok(CompiledScene {
            capture_layout,
            day,
            night,
            ids: TargetResources {
                secondary,
                native,
                refraction: late.copy,
                after_opaque_images: late.images,
                soft_sun: soft_sun.map(|(resources, _)| resources),
                pre_tone,
                temporal,
                taau,
                depth,
                scene,
                motion,
                reactive,
                ambient_receiver: ambient_receiver.map(|(id, _)| id),
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
        native: Option<&'a crate::apple_tile::NativeGraph>,
    ) -> Result<&'a ResourceId, RenderError> {
        Ok(match source {
            CaptureSource::Sdr => output,
            CaptureSource::Edr => return Err(RenderError::GraphPassMismatch),
            CaptureSource::SceneLinear => scene,
            CaptureSource::NativeOutput => &native.ok_or(RenderError::GraphPassMismatch)?.output,
            CaptureSource::NativeHistory => native
                .and_then(|provider| provider.history.as_ref())
                .ok_or(RenderError::GraphPassMismatch)?,
        })
    }

    fn pre_tone_input(
        &self,
        (scene, motion, exposure): (&ResourceId, &ResourceId, &ResourceId),
        temporal: &super::temporal::TemporalResources,
        taau: Option<&super::taau::TaauResources>,
        native: Option<&crate::apple_tile::NativeGraph>,
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
            motion_extent: extent,
            exposure: exposure.clone(),
            extent: native.map_or(extent, |provider| provider.extent),
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
                    || self.soft_sun.is_some()
                    || self.ambient.is_some()
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
        pre_tone: &PreToneInput,
        generation: u64,
        prepared_keys: &mut BTreeSet<PipelineKey>,
        profile: Option<crate::present::SurfaceProfile>,
        interface_reads: &[Access],
    ) -> Result<Option<(ResourceId, PassBody)>, GraphError> {
        profile
            .map(|profile| {
                prepared_keys.insert(crate::present::pipeline_key(generation));
                let edr = profile.format == wgpu::TextureFormat::Rgba16Float;
                let source = if edr { &pre_tone.colour } else { output };
                let (surface, mut pass) =
                    crate::present::surface_pass(graph, source, profile, generation)?;
                if edr {
                    pass.accesses.extend([
                        Access::new(pre_tone.motion.clone(), 0..1, AccessKind::SampledRead),
                        Access::new(
                            pre_tone.exposure.clone(),
                            0..exposure::SAMPLE_BYTES,
                            AccessKind::UniformRead,
                        ),
                    ]);
                    pass.accesses.extend_from_slice(interface_reads);
                }
                Ok((surface, pass))
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
                bytes: DIRECTIONAL_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            directional.clone(),
            0..DIRECTIONAL_BYTES,
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
        let publish = graph.declare(
            PassKey::History,
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(self.history_key()),
                accesses: vec![
                    Access::new(instance, 0..self.instance_bytes, AccessKind::StorageRead),
                    Access::new(history_write, 0..history_bytes, AccessKind::StorageWrite),
                ],
                external_side_effect: false,
            },
        );
        graph.dependency(forward, publish)?;
        Ok(())
    }

    fn readback_pass(
        graph: &mut FrameGraph,
        output: &ResourceId,
        exposure: &ResourceId,
        layout: super::readback::CaptureLayout,
    ) -> Result<PassBody, RenderError> {
        let bytes =
            u64::from(layout.row_bytes) * u64::from(layout.extent[1]) + exposure::SAMPLE_BYTES;
        let destination = graph.resource(
            ResourceDescriptor::Buffer {
                bytes,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        Ok(PassBody {
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
                    | if cfg!(feature = "creation-counters")
                        && (cfg!(test)
                            || self
                                .capabilities
                                .features()
                                .contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER))
                    {
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
        let first = if self.surface_uses_cascades()
            || self
                .atmosphere
                .as_ref()
                .is_some_and(|air| air.view_volume.is_some())
        {
            0
        } else {
            shadow::LAYERS - 1
        };
        let lists = match &self.caster_culling {
            Some(culling) => {
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
                        .skip(first as usize)
                        .map(|matrix| uniform(matrix, shadow::CASTER_VIEW_RANGE)),
                );
                cull.extend_from_slice(route);
                let ids = culling.append_outputs(graph, &mut cull)?;
                let key = self.caster_cull_key();
                prepared_keys.insert(key);
                graph.declare(
                    PassKey::CasterCull,
                    PassBody {
                        scope: PassScope::View(1),
                        pipeline: Some(key),
                        accesses: cull,
                        external_side_effect: false,
                    },
                );
                Some(
                    ids.lists
                        .iter()
                        .map(|list| culling.list_reads(list))
                        .collect::<Vec<_>>(),
                )
            }
            None => None,
        };
        for (index, matrix) in (0..shadow::LAYERS).zip(matrices).skip(first as usize) {
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
            graph.declare(
                match index {
                    0 => PassKey::ShadowNear,
                    1 => PassKey::ShadowFar,
                    _ => PassKey::HorizonHeights,
                },
                PassBody {
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
                },
            );
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
            u64::from(first)..u64::from(shadow::LAYERS),
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
                    initialized: index == 0 || self.cluster_bounds_bytes.is_none(),
                },
            )?;
            accesses.push(Access::new(
                storage.clone(),
                0..bytes,
                AccessKind::StorageRead,
            ));
            resources.push(storage);
        }
        if let Some(bounds_bytes) = self.cluster_bounds_bytes {
            let bounds = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: bounds_bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            prepared_keys.insert(self.cluster_key());
            graph.declare(
                PassKey::Cluster,
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(self.cluster_key()),
                    accesses: vec![
                        Access::new(bounds, 0..bounds_bytes, AccessKind::StorageRead),
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
                },
            );
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
        let environment_light = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: crate::light::ENVIRONMENT_BYTES,
                usage: self.environment_light.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            environment_light.clone(),
            0..crate::light::ENVIRONMENT_BYTES,
            AccessKind::UniformRead,
        ));
        let atmosphere_inputs =
            self.append_environment(graph, keys, accesses, &environment_light)?;
        // Late consumers borrow these same producers through EnvironmentProducts.
        late_uniforms.extend(accesses[start..].iter().cloned());
        Ok(FrameEnvironment {
            light: [directional, environment_light],
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
        environment_light: &ResourceId,
    ) -> Result<Option<(ResourceId, ResourceId)>, RenderError> {
        if let Some(cloud) = &self.cloud {
            cloud.append_reads(graph, accesses, None, None)?;
        }
        Ok(self
            .atmosphere
            .as_ref()
            .map(|atmosphere| {
                atmosphere.append_graph(graph, keys, accesses, environment_light.clone())
            })
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
                #[cfg(all(feature = "creation-counters", target_os = "macos"))]
                let copy_heights = self.capabilities.observe_ray_preparation;
                #[cfg(not(all(feature = "creation-counters", target_os = "macos")))]
                let copy_heights = false;
                air_shadow::append_graph(
                    graph,
                    key,
                    [maps.clone(), directional.clone()],
                    accesses,
                    copy_heights,
                )
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
                if presenter.profile().format == wgpu::TextureFormat::Rgba16Float {
                    presenter
                        .validate_output(binding, view, extent)
                        .map_err(RenderError::Present)?;
                    let input = &frame.target.display;
                    input.check_pass(pass)?;
                    return frame
                        .prepared
                        .display
                        .edr
                        .as_ref()
                        .ok_or(RenderError::GraphPassMismatch)?
                        .encode(
                            frame.encoder,
                            pass,
                            input.edr_binding(frame.exposure_side),
                            crate::graph::GraphColorAttachment {
                                resource: &surface,
                                range: 0..1,
                                view,
                                clear: wgpu::Color::BLACK,
                            },
                            frame.timings.as_deref_mut(),
                            (presenter, binding),
                            frame.interface,
                        );
                }
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
            // A pass declared without a key has no recorder.
            let key = pass.key.ok_or(RenderError::GraphPassMismatch)?;
            bindings.push(self.recorder(&graph, id, key)?);
        }
        graph.bind(bindings).map_err(RenderError::Graph)
    }

    /// The recorder a scene pass's key selects. Every key is listed, so a new
    /// key does not compile until it has a recorder here or is refused.
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "a method path fixes the scene context's lifetime; the recorder must accept any"
    )]
    fn recorder(
        &self,
        graph: &CompiledGraph,
        id: usize,
        key: PassKey,
    ) -> Result<RecordBinding<SceneFamily>, RenderError> {
        type B = RecordBinding<SceneFamily>;
        Ok(match key {
            PassKey::Opaque => B::new(id, PassKind::Render, |frame, pass| frame.opaque(pass)),
            PassKey::Display => B::new(id, PassKind::Render, |frame, pass| frame.tone(pass)),
            PassKey::Interface => B::new(id, PassKind::Render, |frame, pass| frame.interface(pass)),
            PassKey::ShadowNear | PassKey::ShadowFar | PassKey::HorizonHeights => {
                let index = match key {
                    PassKey::ShadowNear => 0,
                    PassKey::ShadowFar => 1,
                    _ => 2,
                };
                B::new(id, PassKind::Render, move |frame, pass| {
                    frame.shadow(index, pass)
                })
            }
            PassKey::AirShadow => {
                B::new(id, PassKind::Render, |frame, pass| frame.air_shadow(pass))
            }
            PassKey::CasterCull => {
                B::new(id, PassKind::Compute, |frame, pass| frame.caster_cull(pass))
            }
            PassKey::Cluster => B::new(id, PassKind::Compute, |frame, pass| frame.cluster(pass)),
            PassKey::History => B::new(id, PassKind::Compute, |frame, pass| {
                frame.publish_history(pass)
            }),
            PassKey::Exposure => B::new(id, PassKind::Compute, |frame, pass| frame.exposure(pass)),
            PassKey::Stars => B::new(id, PassKind::Render, |frame, pass| frame.stars(pass)),
            PassKey::Atmosphere(stage) => B::new(id, PassKind::Compute, move |frame, pass| {
                frame.atmosphere(stage, pass)
            }),
            PassKey::CameraVisibility | PassKey::PrepassList => {
                B::new(id, PassKind::Compute, |frame, pass| frame.visibility(pass))
            }
            PassKey::DepthPrepass => B::new(id, PassKind::Render, |frame, pass| {
                frame.depth_prepass(pass)
            }),
            PassKey::DepthPyramid => B::new(id, PassKind::Compute, |frame, pass| {
                frame.depth_pyramid(pass)
            }),
            PassKey::FinalCull => B::new(id, PassKind::Compute, |frame, pass| {
                frame.final_visibility(pass)
            }),
            PassKey::CullingStats => B::new(id, PassKind::Copy, |frame, _| frame.culling_stats()),
            PassKey::TemporalExposure | PassKey::TemporalWitness => {
                let witness = key == PassKey::TemporalWitness;
                B::new(id, PassKind::Render, move |frame, pass| {
                    frame.temporal(pass, witness)
                })
            }
            PassKey::Taau => B::new(id, PassKind::Compute, |frame, pass| frame.taau(pass)),
            PassKey::ContributionUpload => B::new(id, PassKind::Copy, |frame, _| {
                frame.contribution_upload();
                Ok(())
            }),
            PassKey::ContributionCompute => B::new(id, PassKind::Compute, |frame, pass| {
                frame.contribution_compute(pass)
            }),
            PassKey::Effect(index) => {
                let index = index as usize;
                B::new(id, PassKind::Compute, move |frame, pass| {
                    frame.effect(index, pass)
                })
            }
            PassKey::AfterOpaque(index) => {
                let index = index as usize;
                let kind = match self
                    .effect_graph
                    .after_opaque
                    .get(index)
                    .ok_or(RenderError::GraphPassMismatch)?
                    .kind
                {
                    crate::contribution::AfterOpaqueKind::Render => PassKind::Render,
                    crate::contribution::AfterOpaqueKind::Compute => PassKind::Compute,
                };
                B::new(id, kind, move |frame, pass| frame.after_opaque(index, pass))
            }
            PassKey::AfterOpaqueCopy => {
                B::new(id, PassKind::Copy, |frame, _| frame.refraction_copy())
            }
            PassKey::Secondary { view, stage } => {
                let view = view as usize;
                B::new(id, stage.kind(), move |frame, pass| {
                    frame.secondary_stage(view, stage, pass)
                })
            }
            PassKey::RayStructureBuild => super::raytrace::binding(id),
            PassKey::AmbientReceiver => B::new(id, PassKind::Render, |frame, pass| {
                frame.ambient_receiver(pass)
            }),
            PassKey::Ambient(stage) => B::new(id, PassKind::Compute, move |frame, pass| {
                frame.ambient_visibility(stage, pass)
            }),
            PassKey::SoftSun(stage) => B::new(id, PassKind::Compute, move |frame, pass| {
                frame.soft_sun(stage, pass)
            }),
            PassKey::CombinedVisibility => B::new(id, PassKind::Compute, |frame, pass| {
                frame.soft_sun(super::raytrace::soft::Stage::Trace, pass)
            }),
            PassKey::MetalFx => B::new(id, PassKind::Native, |frame, pass| {
                let native = frame.native.ok_or(RenderError::GraphPassMismatch)?;
                if native.uses_twin() {
                    frame.taau(pass)
                } else {
                    Self::encode_native(frame)
                }
            }),
            PassKey::NativeProviderBefore(_) => Self::bind_native(id, false),
            PassKey::NativeProvider(_) => Self::bind_native(id, true),
            PassKey::Readback => B::new(id, PassKind::Copy, |frame, pass| frame.copy(pass)),
            PassKey::Surface => Self::surface_binding(graph, id)?,
            PassKey::DisplayReferred | PassKey::Sprites => {
                return Err(RenderError::GraphPassMismatch);
            }
        })
    }

    fn encode_native(frame: &mut super::SceneContext<'_>) -> Result<(), RenderError> {
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
    }

    fn bind_native(id: usize, segment: bool) -> RecordBinding<SceneFamily> {
        if segment {
            RecordBinding::<SceneFamily>::new(id, PassKind::Native, |frame, _| {
                Self::encode_native(frame)
            })
        } else {
            RecordBinding::<SceneFamily>::new(id, PassKind::Copy, |frame, _| {
                frame
                    .native
                    .ok_or(RenderError::GraphPassMismatch)?
                    .record_before_native(frame.encoder);
                Ok(())
            })
        }
    }
}
