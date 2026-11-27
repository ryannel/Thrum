//! One preparation transaction for the primary scene's resources and pipelines.

use super::forward::CameraDraw;
use super::{
    AirShadowPrepared, ArenaLimits, BucketRoute, CasterBox, CasterCulling, CasterInputs,
    ClusterCompute, ClusterTable, ContributionPasses, CullMode, CullingSelection, DEPTH_FORMAT,
    DeviceCapabilities, ExposureResolve, FrameLighting, InstanceUpload, MOTION_FORMAT,
    MeshSelection, NativeWitness, PackedInstances, PreparedSynthetic, REACTIVE_FORMAT,
    RayStructureSelection, RenderError, RetainedRayMeshes, SCENE_FORMAT, SYNTHETIC_REVISION,
    SdrToneStage, ShadowPrepared, ShadowSelection, StarPrepared, SunOpticalDepth, SunShadowMethod,
    SurfaceParts, SyntheticScene, TextureClasses, TransformHistory, VIEW_BYTES, VIEW_SLOTS, abi,
    admit_meshes, camera_culling, caster_box, checked_display_gain, cluster, draw_witness_usage,
    forward, history_gpu, mesh_arena, mesh_rows, native_witness, pack_frame_lighting,
    packed_light_direction, raytrace, scene_source, shadow, storage_buffer, taau, temporal,
};
use crate::abi::AbiError;
use crate::atmosphere::{AtmospherePrepared, CloudPrepared, ground_band};
use crate::binding;
use crate::light::{DIRECTIONAL_BYTES, ENVIRONMENT_BINDING, ENVIRONMENT_BYTES};
use crate::pass_key::PipelineRole;
use crate::records::{CloudField, ClusterHead, LocalSource};
use std::sync::Arc;

impl PreparedSynthetic {
    pub(super) async fn prepare_with_meshes(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        exposure: f32,
        internal: Option<MeshSelection>,
        mut retained_ray_meshes: Option<RetainedRayMeshes>,
    ) -> Result<Self, RenderError> {
        capabilities.recipe().map_err(RenderError::Recipe)?;
        let mut admission = scene.ray_admission(capabilities);
        loop {
            match Self::prepare_candidate(
                capabilities,
                queue,
                scene,
                lighting,
                exposure,
                internal.clone(),
                retained_ray_meshes.take(),
                admission,
            )
            .await
            {
                Err(RenderError::RayPreparation | RenderError::Gpu(_))
                    if admission == RayStructureSelection::Enabled =>
                {
                    admission = RayStructureSelection::PreparationFailed;
                }
                result => return result,
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one finite pipeline preparation transaction keeps the layouts and validation together"
    )]
    pub(super) async fn prepare_candidate(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        lighting: FrameLighting,
        exposure: f32,
        internal: Option<MeshSelection>,
        retained_ray_meshes: Option<RetainedRayMeshes>,
        ray_structure_selection: RayStructureSelection,
    ) -> Result<Self, RenderError> {
        let culling_selection = CullingSelection::resolve(scene.camera_culling, capabilities);
        let camera_mode = culling_selection.resolved;
        let device = capabilities.device();
        // Admit the whole selection before GPU work.
        let (mut mesh_arena, admitted) = admit_meshes(
            scene,
            ArenaLimits::for_binding(device.limits().max_storage_buffer_binding_size),
        )?;
        let selection = internal.or(admitted);
        let meshes = selection.as_ref();
        let generation = capabilities.generation();
        checked_display_gain(exposure)?;
        let (light_data, background, presentation_light_clamps) = pack_frame_lighting(lighting)?;
        // Prepare the declared shadow recipe even while the Sun is dark, so a
        // later live light change can use the same pipelines and target maps.
        let shadow_enabled = scene.sun_shadows;
        let view_volume =
            matches!(lighting, FrameLighting::Atmosphere(frame) if frame.view_volume());
        let shadow_selection = ShadowSelection::resolve(
            scene.sun_shadow_method,
            scene.lamp_shadows,
            ray_structure_selection,
        );
        if scene
            .ambient_occlusion_radius
            .is_some_and(|radius| !radius.is_finite() || radius <= 0.0)
        {
            return Err(RenderError::InvalidLighting);
        }
        let ambient_selection = crate::capabilities::AmbientOcclusionSelection::resolve(
            scene.ambient_occlusion_radius,
            ray_structure_selection,
        );
        let trace_sun = shadow_enabled && shadow_selection.sun == SunShadowMethod::Traced;
        let trace_lights = trace_sun || shadow_selection.lamps;
        let traced_forward = trace_lights || ambient_selection.radius.is_some();
        let soft_sun_enabled = trace_sun && scene.sun_penumbra;
        let gpu_clusters = (1..=cluster::GPU_MAX_SOURCES).contains(&scene.local_lights.len());
        let clusters = if gpu_clusters {
            ClusterTable::build_gpu(&scene.local_lights, &scene.view)?
        } else {
            ClusterTable::build(&scene.local_lights, &scene.view)?
        };
        let light_data = light_data.with_local_lights(clusters.active);
        let [local_light_bytes, cluster_head_bytes, cluster_index_bytes] = clusters.capacities();
        let dropped_light_references = clusters.dropped_references;
        let limits = capabilities.limits();
        if scene.sun_optical_depth.is_some() && !matches!(lighting, FrameLighting::Atmosphere(_)) {
            return Err(RenderError::InvalidLighting);
        }
        let has_ground_band = scene
            .sun_optical_depth
            .as_ref()
            .is_some_and(SunOpticalDepth::has_ground_band);
        let highest_binding = if has_ground_band {
            ground_band::MAPPING_BINDING
        } else {
            ENVIRONMENT_BINDING
        };
        if limits.max_bindings_per_bind_group <= highest_binding
            || matches!(lighting, FrameLighting::Atmosphere(_))
                && (limits.max_sampled_textures_per_shader_stage
                    < 9 + 2 * u32::from(shadow_enabled)
                        + u32::from(scene.sun_optical_depth.is_some())
                        + u32::from(has_ground_band)
                        + u32::from(scene.wetness.is_some())
                        + 2 * u32::from(soft_sun_enabled)
                        + u32::from(ambient_selection.radius.is_some())
                    || limits.max_samplers_per_shader_stage < 3 + u32::from(shadow_enabled))
        {
            return Err(AbiError::DeviceLimits.into());
        }
        let wetness = scene
            .wetness
            .as_ref()
            .map(|field| crate::wetness::PreparedWetness::new(device, queue, field))
            .transpose()?;
        let cloud = scene
            .sun_optical_depth
            .as_ref()
            .map(|field| {
                CloudPrepared::new(
                    device,
                    queue,
                    field,
                    match lighting {
                        FrameLighting::Atmosphere(frame) => frame.simulation_time_seconds(),
                        _ => 0.0,
                    },
                )
            })
            .transpose()?;
        for (bytes, error) in [
            (local_light_bytes, RenderError::LocalLightBufferCapacity),
            (cluster_head_bytes, RenderError::ClusterHeadBufferCapacity),
            (cluster_index_bytes, RenderError::ClusterIndexBufferCapacity),
        ] {
            if bytes > limits.max_storage_buffer_binding_size {
                return Err(error);
            }
        }
        let classes = TextureClasses::defaults(limits)?;
        let (material_bytes, instance_bytes) =
            abi::table_bytes(scene.materials.len(), scene.instances.len(), limits)?;
        let material_count =
            u32::try_from(scene.materials.len()).map_err(|_| RenderError::MaterialTableCapacity)?;
        let instance_count =
            u32::try_from(scene.instances.len()).map_err(|_| RenderError::InstanceTableCapacity)?;
        if u64::from(instance_count) * history_gpu::ENTRY_BYTES
            > limits.max_storage_buffer_binding_size
        {
            return Err(RenderError::TransformHistoryCapacity);
        }
        let mut packed_materials = Vec::with_capacity(
            usize::try_from(material_bytes).map_err(|_| RenderError::MaterialTableCapacity)?,
        );
        for material in &scene.materials {
            // This is a single opaque static closure, not admission for all PBR flags.
            if material.flags != [0; 4] || material.texture_layers != [0; 4] {
                return Err(RenderError::UnsupportedSyntheticMaterial);
            }
            packed_materials.extend_from_slice(&material.pack(&classes)?);
        }
        let packed_instances = PackedInstances::new(&scene.instances, material_count)?;
        let needs_structure = traced_forward || scene.ray_structure || scene.view.traced_geometry();
        let ray_structure_selection = if needs_structure {
            ray_structure_selection
        } else if ray_structure_selection == RayStructureSelection::Enabled {
            RayStructureSelection::NotRequested
        } else {
            ray_structure_selection
        };
        let ray_structure = if ray_structure_selection == RayStructureSelection::Enabled {
            let structure = raytrace::prepare(
                capabilities,
                queue,
                &mut mesh_arena,
                instance_count,
                retained_ray_meshes,
            )
            .await;
            Some(structure.ok_or(RenderError::RayPreparation)?)
        } else {
            None
        };
        // Separate locals keep the scopes in stack order even on an early return.
        let internal_scope = ray_structure
            .is_some()
            .then(|| device.push_error_scope(wgpu::ErrorFilter::Internal));
        let memory_scope = ray_structure
            .is_some()
            .then(|| device.push_error_scope(wgpu::ErrorFilter::OutOfMemory));
        let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let material_buffer = storage_buffer(
            device,
            queue,
            "synthetic materials",
            &packed_materials,
            material_bytes,
        );
        let instance_buffer = storage_buffer(
            device,
            queue,
            "synthetic forward opaque",
            bytemuck::cast_slice(&packed_instances.rows),
            instance_bytes,
        );
        let mut instance_upload = InstanceUpload::new(
            instance_buffer,
            packed_instances,
            &scene.instances,
            material_count,
            instance_count,
        );
        let view_alignment = limits.min_uniform_buffer_offset_alignment.max(1);
        let view_stride = u32::try_from(VIEW_BYTES)
            .expect("view record fits u32")
            .checked_add(view_alignment - 1)
            .map(|bytes| bytes / view_alignment * view_alignment)
            .filter(|stride| *stride <= u32::MAX / VIEW_SLOTS)
            .ok_or(RenderError::ViewBufferCapacity)?;
        let view_capacity = u64::from(view_stride) * u64::from(VIEW_SLOTS);
        if view_capacity > limits.max_buffer_size {
            return Err(RenderError::ViewBufferCapacity);
        }
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic ABI2 view"),
            size: view_capacity,
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | draw_witness_usage(),
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &view_buffer,
            0,
            &scene.view.pack_view(&scene.view, exposure),
        );
        let view_entries = forward::view_entries(
            meshes.is_some(),
            camera_mode,
            shadow_enabled,
            !view_volume && matches!(lighting, FrameLighting::Atmosphere(_)),
        );
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic view and target aux"),
            entries: &view_entries,
        });
        let mut light_entries = vec![
            binding::uniform(0, wgpu::ShaderStages::FRAGMENT, DIRECTIONAL_BYTES),
            binding::storage(
                1,
                wgpu::ShaderStages::FRAGMENT,
                true,
                size_of::<LocalSource>() as u64,
            ),
            binding::storage(
                2,
                wgpu::ShaderStages::FRAGMENT,
                true,
                size_of::<ClusterHead>() as u64,
            ),
            binding::storage(
                3,
                wgpu::ShaderStages::FRAGMENT,
                true,
                size_of::<u32>() as u64,
            ),
            binding::uniform(
                ENVIRONMENT_BINDING,
                wgpu::ShaderStages::FRAGMENT,
                ENVIRONMENT_BYTES,
            ),
        ];
        if matches!(lighting, FrameLighting::Atmosphere(_)) {
            if scene.sky_overlay {
                light_entries.extend(
                    crate::atmosphere::overlay::ChartBindings::Uniform
                        .layout(wgpu::ShaderStages::FRAGMENT),
                );
            }
            for (binding, dimension) in [
                (4, wgpu::TextureViewDimension::D2Array),
                (5, wgpu::TextureViewDimension::D2Array),
                (8, wgpu::TextureViewDimension::Cube),
                (9, wgpu::TextureViewDimension::D2),
                (11, wgpu::TextureViewDimension::D2),
            ] {
                light_entries.push(binding::texture(
                    binding,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::TextureSampleType::Float { filterable: true },
                    dimension,
                ));
            }
            for binding in [6, 10] {
                light_entries.push(binding::sampler(
                    binding,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::SamplerBindingType::Filtering,
                ));
            }
            light_entries.push(binding::uniform(
                7,
                wgpu::ShaderStages::FRAGMENT,
                size_of::<crate::records::Atmosphere>() as u64,
            ));
        }
        light_entries.push(binding::uniform(
            12,
            wgpu::ShaderStages::FRAGMENT,
            size_of::<[f32; 4]>() as u64,
        ));
        if cloud.is_some() {
            light_entries.push(binding::texture(
                13,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2,
            ));
            light_entries.push(binding::uniform(
                14,
                wgpu::ShaderStages::FRAGMENT,
                if scene
                    .sun_optical_depth
                    .as_ref()
                    .is_some_and(SunOpticalDepth::gpu_written)
                {
                    size_of::<crate::atmosphere::GpuSunMapping>() as u64
                } else {
                    size_of::<CloudField>() as u64
                },
            ));
        }
        if cloud.as_ref().is_some_and(|cloud| cloud.band.is_some()) {
            light_entries.extend(ground_band::GroundBandPrepared::layout(
                wgpu::ShaderStages::FRAGMENT,
            ));
        }
        if wetness.is_some() {
            light_entries.push(crate::wetness::PreparedWetness::forward_layout());
        }
        let map_light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic directional and clustered sources"),
            entries: &light_entries,
        });
        let lamp_shadows = shadow_selection
            .lamps
            .then(|| raytrace::LampShadows::new(device, queue));
        let ambient = ambient_selection
            .radius
            .map(|radius| raytrace::ambient::Ambient::new(device, radius));
        let soft_sun = soft_sun_enabled.then(|| {
            raytrace::soft::SoftSun::new(device, ambient.as_ref().map(|ambient| &ambient.pipelines))
        });
        let light_layout = if traced_forward {
            if trace_lights {
                light_entries.push(binding::acceleration_structure(
                    15,
                    wgpu::ShaderStages::FRAGMENT,
                ));
            }
            if ambient.is_some() {
                light_entries.extend(raytrace::ambient::Ambient::light_entries());
            }
            if soft_sun.is_some() {
                light_entries.extend(raytrace::soft::SoftPipelines::light_entries());
            }
            if lamp_shadows.is_some() {
                light_entries.extend(raytrace::LampShadows::layout_entries());
            }
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("synthetic traced lights"),
                entries: &light_entries,
            })
        } else {
            map_light_layout.clone()
        };
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic directional source"),
            size: DIRECTIONAL_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&directional, 0, bytemuck::bytes_of(&light_data));
        let mut environment_usage = wgpu::BufferUsages::UNIFORM;
        if matches!(lighting, FrameLighting::Atmosphere(_)) {
            environment_usage |= wgpu::BufferUsages::STORAGE;
            #[cfg(all(test, feature = "creation-counters"))]
            {
                environment_usage |= wgpu::BufferUsages::COPY_SRC;
            }
        }
        // WebGPU initializes this buffer to zero. Only the atmosphere can write it.
        let environment_light = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("environment light"),
            size: ENVIRONMENT_BYTES,
            usage: environment_usage,
            mapped_at_creation: false,
        });
        let exposure_samples = ExposureResolve::allocate_samples(device);
        let mut atmosphere = match lighting {
            FrameLighting::Atmosphere(frame) if scene.sky_overlay => {
                Some(AtmospherePrepared::prepare(
                    device,
                    queue,
                    generation,
                    frame,
                    &scene.view,
                    &environment_light,
                    &exposure_samples,
                    scene.sky_overlay.then_some((&directional, cloud.as_ref())),
                ))
            }
            FrameLighting::Atmosphere(frame) => Some(AtmospherePrepared::new(
                device,
                queue,
                generation,
                frame,
                &scene.view,
                &environment_light,
                &exposure_samples,
            )),
            _ => None,
        };
        if scene.sky_overlay && atmosphere.is_none() {
            return Err(RenderError::GraphPassMismatch);
        }
        if let Some(air) = &mut atmosphere {
            air.prepare_view_volume(
                device,
                queue,
                &scene.view,
                shadow_enabled,
                scene.sun_optical_depth.as_ref().zip(cloud.as_ref()),
            );
        }
        let local_lights = storage_buffer(
            device,
            queue,
            "synthetic local lights",
            &clusters.lights,
            local_light_bytes,
        );
        let cluster_heads = storage_buffer(
            device,
            queue,
            "synthetic cluster heads",
            &clusters.heads,
            cluster_head_bytes,
        );
        let cluster_indices = storage_buffer(
            device,
            queue,
            "synthetic cluster indices",
            &clusters.indices,
            cluster_index_bytes,
        );
        let cluster_compute = gpu_clusters.then(|| {
            let bounds = storage_buffer(
                device,
                queue,
                "synthetic cluster source bounds",
                &clusters.bounds,
                u64::try_from(clusters.bounds.len()).expect("32 sources fit u64"),
            );
            ClusterCompute::new(device, bounds, &cluster_heads, &cluster_indices)
        });
        let light = forward::LightBindings {
            directional: &directional,
            environment_light: &environment_light,
            local: &local_lights,
            heads: &cluster_heads,
            indices: &cluster_indices,
            atmosphere: atmosphere.as_ref(),
            cloud: cloud.as_ref(),
            wetness: wetness.as_ref(),
            aerial: None,
        }
        .bind(
            device,
            &map_light_layout,
            &exposure_samples,
            None,
            None,
            None,
            None,
        );
        let mut entries = vec![binding::storage(
            0,
            wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
            true,
            abi::MATERIAL_BYTES as u64,
        )];
        for binding in 1..=4 {
            entries.push(binding::texture(
                binding,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2Array,
            ));
        }
        entries.push(binding::sampler(
            5,
            wgpu::ShaderStages::FRAGMENT,
            wgpu::SamplerBindingType::Filtering,
        ));
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ABI2 material table and classes"),
            entries: &entries,
        });
        if mesh_arena.buffers().is_none() {
            mesh_arena.upload(device, queue);
        }
        let cube_draw = mesh_arena
            .row(mesh_arena::MeshId::CUBE)
            .and_then(|row| {
                let base = row.draw_base_vertex().ok()?;
                Some((row.first_index..row.first_index + row.index_count, base))
            })
            .ok_or(RenderError::MissingMeshRow)?;
        let arena_buffers = mesh_arena
            .buffers()
            .ok_or(RenderError::MissingMeshBuffers)?;
        let caster_boxes = meshes
            .map(|selection| {
                selection
                    .slots
                    .iter()
                    .map(|&mesh| {
                        mesh_arena
                            .row(mesh)
                            .map(|row| CasterBox::from_bounds(row.bounds))
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or(RenderError::MissingMeshRow)
            })
            .transpose()?;
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ABI2 instance table"),
            entries: &[
                binding::storage(
                    0,
                    wgpu::ShaderStages::VERTEX,
                    true,
                    abi::INSTANCE_BYTES as u64,
                ),
                binding::storage(
                    1,
                    wgpu::ShaderStages::VERTEX,
                    true,
                    history_gpu::ENTRY_BYTES,
                ),
                binding::uniform(2, wgpu::ShaderStages::VERTEX, history_gpu::SERIAL_BYTES),
                binding::storage(
                    3,
                    wgpu::ShaderStages::VERTEX,
                    true,
                    mesh_arena::VERTEX_BYTES,
                ),
            ],
        });
        let history = TransformHistory::new(
            device,
            queue,
            instance_upload.buffer(),
            &arena_buffers.vertices,
            &instance_layout,
            instance_count,
        );
        let shadow = if shadow_enabled {
            let direction = packed_light_direction(&light_data);
            Some(ShadowPrepared::new(
                device,
                queue,
                &scene.view,
                (&scene.instances, caster_boxes.as_deref()),
                direction,
                &instance_layout,
            )?)
        } else {
            None
        };
        let air_shadow = (shadow.is_some() && atmosphere.is_some() && !view_volume).then(|| {
            AirShadowPrepared::new(device, shadow::SAMPLE_BYTES as u64, DIRECTIONAL_BYTES)
        });
        if let Some(shadow) = shadow.as_ref() {
            instance_upload.seed_shadow_clamps(&scene.instances, |slot, instance| {
                instance.is_active()
                    && !shadow.contains_caster(instance, caster_box(caster_boxes.as_deref(), slot))
            });
        }
        let views: Vec<wgpu::TextureView> = classes
            .arrays()
            .iter()
            .map(|array| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("typed default texture class"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: array.class.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &array.class.default_texel(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4),
                        rows_per_image: Some(1),
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let materials = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic material classes"),
            layout: &material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&views[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let bind_group_layouts: [Option<&wgpu::BindGroupLayout>; 4] = [
            Some(&view_layout),
            Some(&light_layout),
            Some(&material_layout),
            Some(&instance_layout),
        ];
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic ABI2"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        let depth_layout = (traced_forward && camera_mode == CullMode::Occlusion).then(|| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("synthetic depth prepass"),
                bind_group_layouts: &[
                    Some(&view_layout),
                    Some(&map_light_layout),
                    Some(&material_layout),
                    Some(&instance_layout),
                ],
                immediate_size: 0,
            })
        });
        #[cfg(feature = "creation-counters")]
        let fail_traced_pipeline = traced_forward && capabilities.fail_traced_pipeline;
        #[cfg(not(feature = "creation-counters"))]
        let fail_traced_pipeline = false;
        let prepare_shader = |wetness| {
            scene_source::prepare(
                device,
                if view_volume {
                    crate::shaders::AtmosphereVariant::ViewVolume
                } else {
                    crate::shaders::AtmosphereVariant::new(
                        atmosphere.is_some(),
                        air_shadow.is_some(),
                    )
                },
                scene.sun_optical_depth.as_ref(),
                wetness,
                scene_source::SunShadows::from(shadow.is_some()),
                shadow_selection,
                (soft_sun.is_some(), ambient.is_some()),
                fail_traced_pipeline,
                atmosphere.as_ref(),
            )
        };
        let (shader_source, shader, fragment_entry) = prepare_shader(
            scene
                .wetness
                .as_ref()
                .map(crate::wetness::WetnessProvider::source),
        );
        let (dry_shader_source, dry_shader) = if wetness.is_some() {
            let (source, module, _) = prepare_shader(None);
            (Some(source), Some(module))
        } else {
            (None, None)
        };
        let vertex_entry = camera_culling::vertex_entry(camera_mode);
        let opaque_targets = [
            Some(wgpu::ColorTargetState {
                format: SCENE_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: MOTION_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: REACTIVE_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ];
        let receiver_targets = [Some(wgpu::ColorTargetState {
            format: raytrace::ambient::RECEIVER_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let create_route_pipeline = |module: &crate::runtime::PipelineInput<wgpu::ShaderModule>,
                                     vertex_entry,
                                     primitive,
                                     draw| {
            let receiver = draw == CameraDraw::AmbientReceiver;
            device.render(&wgpu::RenderPipelineDescriptor {
                label: Some("prepared opaque synthetic role"),
                layout: Some(if draw == CameraDraw::Depth {
                    depth_layout.as_ref().unwrap_or(&layout)
                } else {
                    &layout
                }),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive,
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(!receiver),
                    depth_compare: Some(if receiver {
                        wgpu::CompareFunction::Equal
                    } else {
                        wgpu::CompareFunction::Greater
                    }),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                // The depth prepass shares the module, vertex entry, primitive
                // and depth state, so it rasterizes the same depth.
                fragment: (draw != CameraDraw::Depth).then_some(wgpu::FragmentState {
                    module,
                    entry_point: Some(if receiver {
                        raytrace::ambient::RECEIVER_ENTRY
                    } else {
                        fragment_entry
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: if receiver {
                        &receiver_targets
                    } else {
                        &opaque_targets
                    },
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let create_pipeline = |cull_mode, draw| {
            let primitive = wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            };
            let module = if draw == CameraDraw::Forward {
                &shader
            } else {
                dry_shader.as_ref().unwrap_or(&shader)
            };
            create_route_pipeline(module, vertex_entry, primitive, draw)
        };
        let mut bucket_route = meshes
            .map(|selection| {
                BucketRoute::new(
                    device,
                    queue,
                    selection,
                    &mesh_arena,
                    &instance_upload,
                    &scene.instances,
                    |primitive, draw| {
                        let module = if draw == CameraDraw::Forward {
                            &shader
                        } else {
                            dry_shader.as_ref().unwrap_or(&shader)
                        };
                        create_route_pipeline(module, "compacted_vertex", primitive, draw)
                    },
                    (camera_mode == CullMode::Occlusion, ambient.is_some()),
                )
            })
            .transpose()?;
        if let (Some(route), Some(module)) = (bucket_route.as_mut(), dry_shader.as_ref()) {
            route.prepare_dry(|primitive| {
                create_route_pipeline(module, "compacted_vertex", primitive, CameraDraw::Forward)
            });
        }
        let dry_pipelines = dry_shader.as_ref().map(|module| {
            [Some(wgpu::Face::Back), None].map(|cull_mode| {
                create_route_pipeline(
                    module,
                    vertex_entry,
                    wgpu::PrimitiveState {
                        cull_mode,
                        ..Default::default()
                    },
                    CameraDraw::Forward,
                )
            })
        });
        let visibility = camera_culling::prepare(
            device,
            queue,
            generation,
            camera_mode,
            (instance_count, instance_upload.active_count()),
            bucket_route.as_ref(),
            &mesh_arena,
        )?;
        let front_pipelines = || {
            let pipeline = create_pipeline(Some(wgpu::Face::Back), CameraDraw::Forward);
            let depth = (camera_mode == CullMode::Occlusion)
                .then(|| create_pipeline(Some(wgpu::Face::Back), CameraDraw::Depth));
            let ambient = ambient
                .as_ref()
                .map(|_| create_pipeline(Some(wgpu::Face::Back), CameraDraw::AmbientReceiver));
            (pipeline, depth, ambient)
        };
        let two_sided_pipelines = || {
            let pipeline = create_pipeline(None, CameraDraw::Forward);
            let depth = (camera_mode == CullMode::Occlusion)
                .then(|| create_pipeline(None, CameraDraw::Depth));
            let ambient = ambient
                .as_ref()
                .map(|_| create_pipeline(None, CameraDraw::AmbientReceiver));
            (pipeline, depth, ambient)
        };
        #[cfg(target_os = "macos")]
        let (front, two_sided) =
            prepare_pipeline_pair(device, front_pipelines, two_sided_pipelines)?;
        #[cfg(not(target_os = "macos"))]
        let (front, two_sided) = (front_pipelines(), two_sided_pipelines());
        let (pipeline, depth_front, ambient_front) = front;
        let (pipeline_two_sided, depth_two_sided, ambient_two_sided) = two_sided;
        let depth_prepass = depth_front
            .zip(depth_two_sided)
            .map(|(front, two_sided)| [front, two_sided]);
        let ambient_receiver_pipelines = ambient_front
            .zip(ambient_two_sided)
            .map(|(front, two_sided)| [front, two_sided]);
        let shadow_members = shadow
            .as_ref()
            .and_then(ShadowPrepared::member_layout)
            .zip(bucket_route.as_ref())
            .map(|(layout, route)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("synthetic Sun caster members"),
                    layout,
                    entries: &[route.members_entry(0)],
                })
            });
        // Direct draws every admitted caster in every Sun pass: the oracle.
        let caster_culling = match (&shadow, &bucket_route, &visibility) {
            (Some(shadow), Some(route), Some(_)) => {
                let layout = shadow
                    .member_layout()
                    .ok_or(RenderError::GraphPassMismatch)?;
                let mut culling = CasterCulling::new(
                    device,
                    queue,
                    &camera_culling::mesh_records(&mesh_arena)?,
                    route.shape()?,
                    &CasterInputs {
                        instances: instance_upload.buffer(),
                        serials: history.serial_buffer(),
                        members: route.member_inputs(),
                        views: shadow.caster_views(),
                    },
                );
                culling.set_member_groups(route.groups());
                let groups = culling
                    .lists()
                    .map(|(list, _)| {
                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("synthetic Sun culled casters"),
                            layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                    buffer: list,
                                    offset: 0,
                                    size: wgpu::BufferSize::new(culling.segment_bytes()),
                                }),
                            }],
                        })
                    })
                    .collect();
                Some((culling, groups))
            }
            _ => None,
        };
        let exposure_resolve = if scene.sky_overlay {
            ExposureResolve::prepare(
                device,
                &directional,
                &environment_light,
                history.serial_buffer(),
                exposure_samples,
                (cloud.as_ref(), atmosphere.as_ref()),
            )
        } else {
            ExposureResolve::new(
                device,
                &directional,
                &environment_light,
                history.serial_buffer(),
                exposure_samples,
                cloud.as_ref().zip(atmosphere.as_ref()),
            )
        };
        exposure_resolve.update(
            queue,
            scene.view.exposure_policy(exposure, atmosphere.is_some()),
        );
        let prepare_display = || {
            SdrToneStage::new(
                device,
                PipelineRole::Display.pipeline_key(generation, SYNTHETIC_REVISION),
            )
        };
        let prepare_temporal = || {
            let temporal = temporal::TemporalInputs::new(device);
            let taau = scene.view.reconstruction().then(|| {
                taau::Taau::new(device, scene.view.reconstruction_reference(), view_volume)
            });
            (temporal, taau)
        };
        #[cfg(target_os = "macos")]
        let (display, (temporal, taau)) =
            prepare_pipeline_pair(device, prepare_display, prepare_temporal)?;
        #[cfg(not(target_os = "macos"))]
        let (display, (temporal, taau)) = (prepare_display(), prepare_temporal());
        let stars = if atmosphere.is_some() {
            scene
                .star_catalogue
                .map(|records| {
                    StarPrepared::new(
                        device,
                        records,
                        limits.max_storage_buffer_binding_size,
                        &light_data,
                        view_volume,
                    )
                })
                .transpose()?
        } else {
            None
        };
        let traced_view = (scene.view.traced_geometry() && ray_structure.is_some())
            .then(|| Box::new(raytrace::TracedView::new(device, &view_layout)));
        let mut pipeline_error = None;
        for scope in [Some(validation_scope), memory_scope, internal_scope]
            .into_iter()
            .flatten()
        {
            pipeline_error = scope.pop().await.or(pipeline_error);
        }
        #[cfg(feature = "creation-counters")]
        if traced_forward && capabilities.fail_traced_pipeline_internal {
            let description = String::from("injected native traced pipeline linkage failure");
            pipeline_error = Some(wgpu::Error::Internal {
                source: Box::new(std::io::Error::other(description.clone())),
                description,
            });
        }
        if let Some(error) = pipeline_error {
            return Err(RenderError::Gpu(error));
        }
        Ok(Self {
            native: if scene.view.requests_metalfx() {
                Some(crate::apple_tile::metalfx::Factory::new(
                    capabilities,
                    queue,
                    scene.view.metalfx_probe(),
                ))
            } else if scene.view.native_witness() == NativeWitness::Off {
                None
            } else {
                Some(native_witness::Factory::new(
                    capabilities,
                    queue,
                    scene.view.native_witness(),
                ))
            },
            temporal_inputs: scene.view.temporal_inputs(),
            temporal,
            taau,
            capabilities: capabilities.clone(),
            culling_selection,
            classes,
            owner: Arc::new(()),
            key: PipelineRole::Opaque.pipeline_key(generation, SYNTHETIC_REVISION),
            pipeline,
            pipeline_two_sided,
            dry_pipelines,
            depth_prepass,
            ambient_receiver_pipelines,
            row_updates: scene
                .meshes
                .as_ref()
                .and_then(|meshes| meshes.updates.clone())
                .map(|updates| mesh_rows::LiveRows::new(updates, &mut mesh_arena)),
            mesh_arena,
            ray_structure,
            traced_view,
            ray_structure_selection,
            shadow_selection,
            lamp_shadows,
            soft_sun,
            ambient,
            ambient_selection,
            bucket_route,
            caster_boxes,
            shadow_members,
            caster_culling,
            cube_draw,
            display,
            interface_inputs: None,
            view_layout,
            view_buffer,
            view_stride,
            view_sample: scene.view,
            jitter_phase: 0,
            pending_view: None,
            frame_revision: 0,
            cloud,
            wetness,
            directional_buffer: directional,
            environment_light,
            local_light_buffer: local_lights,
            cluster_head_buffer: cluster_heads,
            cluster_index_buffer: cluster_indices,
            clusters,
            cluster_compute,
            light,
            map_light_layout,
            materials,
            history,
            visibility,
            stars,
            secondary: Vec::new(),
            secondary_handles: Vec::new(),
            secondary_sources: Vec::new(),
            exposure: exposure_resolve,
            history_source: None,
            instance_upload,
            material_bytes,
            instance_bytes,
            instance_count,
            exposure_bits: exposure.to_bits(),
            local_light_bytes,
            cluster_head_bytes,
            cluster_index_bytes,
            dropped_light_references,
            presentation_light_clamps,
            background,
            shadow,
            air_shadow,
            atmosphere,
            surface: SurfaceParts {
                shader_source,
                dry_shader_source,
                pipeline_layout: layout,
                light_layout,
                material_layout,
                fragment_entry,
            },
            contribution_passes: ContributionPasses::default(),
            effect_graph: crate::contribution::EffectGraph::default(),
            effect_time: crate::contribution::EffectTime {
                epoch: 0,
                timeline: 0,
                tick: 0,
                tick_duration: std::time::Duration::from_secs_f64(1.0 / 60.0),
                alpha: 1.0,
                display_phase: 1.0,
                capture: false,
            },
        })
    }
}

// The scoped worker owns its wgpu error scopes. The parent still records errors
// in the existing preparation transaction, and neither side can outlive it.
#[cfg(target_os = "macos")]
fn prepare_pipeline_pair<A: Send, B: Send>(
    device: &crate::runtime::DeviceSite,
    front: impl FnOnce() -> A + Send,
    two_sided: impl FnOnce() -> B + Send,
) -> Result<(A, B), RenderError> {
    let Some((phase, label)) = device.pipeline_worker_scope() else {
        return Ok((front(), two_sided()));
    };
    let detached = device.ledger().detach();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            let _region = detached.region(phase, label);
            run_pipeline_worker(device, front)
        });
        let second = scope.spawn(|| {
            let _region = detached.region(phase, label);
            run_pipeline_worker(device, two_sided)
        });
        let first = first.join().expect("pipeline worker completed")?;
        let second = second.join().expect("pipeline worker completed")?;
        Ok((first, second))
    })
}

#[cfg(target_os = "macos")]
fn run_pipeline_worker<T>(
    device: &crate::runtime::DeviceSite,
    create: impl FnOnce() -> T,
) -> Result<T, RenderError> {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let result = create();
    let mut error = None;
    for scope in [memory, internal, validation] {
        error = pollster::block_on(scope.pop()).or(error);
    }
    error.map_or(Ok(result), |error| Err(RenderError::Gpu(error)))
}
