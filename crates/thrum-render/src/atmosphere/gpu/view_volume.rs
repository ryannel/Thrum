use crate::{
    atmosphere::{CloudPrepared, EnvironmentEpoch, SunOpticalDepth},
    binding,
    graph::{Access, AccessKind, FrameGraph, GraphError, ResourceDescriptor, ResourceScope},
    records::VolumeHistory,
    shaders::{CloudVariant, Shader, wgsl},
    synthetic::SyntheticView,
};
use glam::{Mat4, Vec4};

const INTERVALS: u32 = 80;
pub(crate) const NEAR_INTERVALS: u32 = 40;
const NEAR_DISTANCE: f32 = 500.0;
const HISTORY_GAIN: f32 = 262_144.0;
const WIDTH: u32 = 160;
const HEIGHT: u32 = 90;

pub(super) fn extent(depth_factor: u8) -> [u32; 3] {
    [WIDTH, HEIGHT, INTERVALS * u32::from(depth_factor) + 2]
}

pub(crate) fn grid_source() -> String {
    grid_source_with_near(NEAR_INTERVALS)
}

pub(super) fn grid_source_with_near(near: u32) -> String {
    let layers = near * 2;
    format!(
        "const VOLUME_INTERVALS = {layers}u;\nconst VOLUME_NEAR_LAYERS = {near}u;\n\
         const VOLUME_NEAR_DISTANCE = {NEAR_DISTANCE:?};\nconst VOLUME_HISTORY_GAIN = {HISTORY_GAIN:?};\n{}",
        wgsl!("atmosphere/volume_grid.wgsl")
    )
}

pub(crate) fn aerial_source() -> String {
    format!(
        "{}{}{}{}{}",
        wgsl!("records/observer.wgsl"),
        wgsl!(@bytes "synthetic_aerial.wgsl"),
        wgsl!(@bytes "atmosphere/volume_sample.wgsl"),
        wgsl!(@bytes "synthetic_aerial_decode.wgsl"),
        grid_source(),
    )
}

const MOTION_BYTES: u64 = size_of::<VolumeHistory>() as u64;

#[derive(Clone)]
struct HistoryFrame {
    view: SyntheticView,
    environment_epoch: EnvironmentEpoch,
    provider_epochs: [Option<u64>; 2],
}

#[derive(Clone)]
pub(crate) struct ViewVolume {
    pipeline: wgpu::ComputePipeline,
    scene_layout: wgpu::BindGroupLayout,
    groups: [wgpu::BindGroup; 2],
    compare: Option<wgpu::Sampler>,
    histories: [[wgpu::Texture; 2]; 2],
    motion: wgpu::Buffer,
    medium: Option<wgpu::Texture>,
    intervals: u32,
    submitted: Option<HistoryFrame>,
    pending: Option<HistoryFrame>,
    phase: u16,
}

pub(super) fn texture_entry(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

pub(super) fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

pub(super) fn read_texture(
    binding: u32,
    dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    binding::texture(
        binding,
        wgpu::ShaderStages::COMPUTE,
        wgpu::TextureSampleType::Float { filterable: true },
        dimension,
    )
}

fn write_volume(binding: u32) -> wgpu::BindGroupLayoutEntry {
    binding::storage_texture(
        binding,
        wgpu::ShaderStages::COMPUTE,
        wgpu::StorageTextureAccess::WriteOnly,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureViewDimension::D2Array,
    )
}

pub(crate) fn source(cascades: bool, cloud: CloudVariant, medium: bool, near: u32) -> String {
    let ground = cloud.ground();
    let cascade = if cascades {
        concat!(
            wgsl!("records/shadow_cascades.wgsl"),
            wgsl!("atmosphere/volume_cascade.wgsl")
        )
    } else {
        "fn volume_terrain_visibility(point: vec3<f32>, depth: f32) -> f32 { return 1.0; }
         fn volume_cascade_fade(depth: f32) -> f32 { return 1.0; }"
    };
    let cloud = if cloud.present() {
        let source = cloud.sampling_source();
        let path = if cloud.gpu() {
            "if cloud_field.plane_drift.w == 0.0 { return vec3(0.0); }
                 return cloud_field.path_border.xyz;"
        } else {
            "let sun = aerial_frame.sun_lux.xyz;
                 if cloud_field.validity.x == 0.0 || sun.y < cloud_field.plane_drift.w { return vec3(0.0); }
                 return sun;"
        };
        format!(
            "@group(1) @binding(3) var<uniform> aerial_frame: Atmosphere;\n\
             @group(1) @binding(4) var aerial_sampler: sampler;\n{source}\n\
             fn volume_cloud_path() -> vec3<f32> {{ {path} }}\n{}",
            wgsl!("atmosphere/volume_cloud_segment.wgsl")
        )
    } else {
        "fn volume_cloud_projection(direction: vec3<f32>) -> f32 { return -1.0; }\n\
         fn volume_cloud_visibility(eye: vec3<f32>, direction: vec3<f32>,\n\
             projection: f32, start: f32, end: f32) -> f32 { return 1.0; }"
            .to_owned()
    };
    let (integration, medium_code) = if medium {
        (
            medium_integration(ground),
            wgsl!("atmosphere/volume_medium.wgsl"),
        )
    } else {
        (wgsl!("atmosphere/view_volume.wgsl").to_owned(), "")
    };
    format!(
        "{}\n{}{}\n{cascade}\n{cloud}\n{medium_code}",
        super::aerosol::aerial_source(&integration),
        wgsl!("atmosphere/volume_history.wgsl"),
        grid_source_with_near(near),
    )
}

fn medium_integration(ground: bool) -> String {
    let sun_visibility = if ground {
        "terrain * mix(1.0, ground_band_sample(world).y, volume_cascade_fade(sample_distance * forward_cos))"
    } else {
        "terrain"
    };
    wgsl!("atmosphere/view_volume.wgsl")
        .replace("var baseline = empty_ray();", "var baseline = empty_ray();
    var joint_medium = empty_ray();
    var relative = 1.0;
    var has_medium = false;")
        .replace("let visibility = volume_terrain_visibility(world, sample_distance * forward_cos)", "let terrain = volume_terrain_visibility(world, sample_distance * forward_cos);
            let visibility = terrain")
        .replace("            baseline.lo = extend_ray", &format!(
            "            let medium = medium_cell(xy, layer, eye + direction * middle);
            if !has_medium && any(medium != vec3(0.0)) {{
                has_medium = true;
                joint_medium.lo = Segment(baseline.lo.radiance + correction.lo.radiance, baseline.lo.transmission);
                joint_medium.hi = Segment(baseline.hi.radiance + correction.hi.radiance, baseline.hi.transmission);
            }}
            if has_medium {{
                let shadow = {sun_visibility};
                joint_medium.lo = extend_ray(joint_medium.lo, joint_step(single_lo + multi_lo + source[0] / VOLUME_HISTORY_GAIN, sigma_lo, direct[0], medium, shadow, delta));
                joint_medium.hi = extend_ray(joint_medium.hi, joint_step(single_hi + multi_hi + source[1] / VOLUME_HISTORY_GAIN, sigma_hi, direct[1], medium, shadow, delta));
            }}
            relative *= exp(-medium.x * delta);
            baseline.lo = extend_ray"))
        .replace("0.5 * sigma_hi.x * delta", "0.5 * (sigma_hi.x + medium.x) * delta")
        .replace("let joint = project_spectrum", "var joint = project_spectrum")
        .replace("        textureStore(scatter_out, xy, i32(layer + 1u)", "        if has_medium { joint = project_spectrum(joint_medium.lo.radiance, joint_medium.hi.radiance); }
        textureStore(scatter_out, xy, i32(layer + 1u)")
        .replace("vec4(throughput, night));
    }", "vec4(throughput * relative, night));
    }")
        .replace("let joint_rgb = project_spectrum", "var joint_rgb = project_spectrum")
        .replace("    let terminal = vec4((joint_rgb - baseline_rgb) * SKY_LUT_GAIN, 1.0);", "    if has_medium { joint_rgb = project_spectrum(joint_medium.lo.radiance, joint_medium.hi.radiance); }
    let terminal = vec4((joint_rgb - relative * baseline_rgb) * SKY_LUT_GAIN, relative);")
}

impl ViewVolume {
    pub(in crate::atmosphere) fn medium_read(
        &self,
        graph: &mut FrameGraph,
        produced: Option<Access>,
    ) -> Result<Option<Access>, crate::error::RenderError> {
        match (&self.medium, produced) {
            (Some(_), Some(access)) => Ok(Some(access)),
            (None, None) => Ok(None),
            (None, Some(_)) => Err(crate::error::EffectGraphError::InvalidViewVolumeMedium.into()),
            (Some(medium), None) => {
                let resource = graph.resource(
                    ResourceDescriptor::of_texture(medium),
                    ResourceScope::Imported { initialized: true },
                )?;
                Ok(Some(Access::new(
                    resource,
                    0..u64::from(medium.depth_or_array_layers()),
                    AccessKind::SampledRead,
                )))
            }
        }
    }

    pub(super) fn medium_target(
        &self,
        observer: &wgpu::Buffer,
    ) -> Option<super::medium::ViewVolumeMediumTarget> {
        let medium = self.medium.as_ref()?;
        Some(super::medium::ViewVolumeMediumTarget {
            coefficients: medium.clone(),
            observer: observer.clone(),
            history: self.motion.clone(),
        })
    }

    pub(super) fn storage(
        &self,
        air: &super::AtmospherePrepared,
    ) -> crate::atmosphere::ViewVolumeStorage {
        let bytes = |texture: &wgpu::Texture| {
            let texels = texture.size();
            u64::from(texels.width)
                * u64::from(texels.height)
                * u64::from(texels.depth_or_array_layers)
                * u64::from(
                    texture
                        .format()
                        .block_copy_size(None)
                        .expect("uncompressed view volume"),
                )
        };
        let mut resources = [
            (
                "view volume in-scatter and terminal pair",
                bytes(&air.aerial_textures[0]),
            ),
            (
                "view volume transmittance and night in-scatter",
                bytes(&air.aerial_textures[1]),
            ),
            (
                "view volume low bands history A",
                bytes(&self.histories[0][0]),
            ),
            (
                "view volume high bands history A",
                bytes(&self.histories[0][1]),
            ),
            (
                "view volume low bands history B",
                bytes(&self.histories[1][0]),
            ),
            (
                "view volume high bands history B",
                bytes(&self.histories[1][1]),
            ),
            ("view volume history camera", self.motion.size()),
            (
                "atmosphere aerosol",
                air.aerosol.as_ref().expect("volume optics").size(),
            ),
        ]
        .to_vec();
        if let Some(medium) = &self.medium {
            resources.push(("view volume medium coefficients", bytes(medium)));
        }
        crate::atmosphere::ViewVolumeStorage {
            grid: [WIDTH, HEIGHT, self.intervals],
            resources,
        }
    }

    #[cfg(feature = "creation-counters")]
    pub(crate) fn submitted_phase(&self) -> u16 {
        self.phase
    }

    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        air: &super::AtmospherePrepared,
        cascades: bool,
        cloud: Option<(&SunOpticalDepth, &CloudPrepared)>,
    ) -> Self {
        use wgpu::{ShaderStages as Stages, TextureViewDimension as Dim};
        let motion = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view volume history camera"),
            size: MOTION_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let intervals = air.aerial_textures[0].depth_or_array_layers() - 2;
        let medium = air
            .frame
            .medium_epoch
            .map(|_| super::medium::coefficients(device, WIDTH, HEIGHT, intervals / 2));
        let histories = std::array::from_fn(|_| {
            std::array::from_fn(|band| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("view volume history source density"),
                    size: wgpu::Extent3d {
                        width: WIDTH,
                        height: HEIGHT,
                        depth_or_array_layers: intervals
                            + if band == 0 && medium.is_some() {
                                intervals / 2
                            } else {
                                0
                            },
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    view_formats: &[],
                })
            })
        });
        let history_views = histories.each_ref().map(|side| {
            side.each_ref().map(|texture| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(Dim::D2Array),
                    ..Default::default()
                })
            })
        });
        let mut entries = vec![
            binding::uniform(0, Stages::COMPUTE, super::UNIFORM_BYTES),
            read_texture(1, Dim::D2),
            read_texture(2, Dim::D2),
            write_volume(3),
            write_volume(4),
            binding::sampler(5, Stages::COMPUTE, wgpu::SamplerBindingType::Filtering),
            super::aerosol::layout(),
            write_volume(9),
            write_volume(10),
            read_texture(11, Dim::D2Array),
            read_texture(12, Dim::D2Array),
            binding::uniform(15, Stages::COMPUTE, MOTION_BYTES),
        ];
        entries.extend(super::night::layout_entries());
        if medium.is_some() {
            entries.push(read_texture(16, Dim::D2Array));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view volume air and history"),
            entries: &entries,
        });
        let mut scene_entries = Vec::new();
        let compare = cascades.then(|| {
            scene_entries.extend([
                binding::uniform(
                    0,
                    Stages::COMPUTE,
                    size_of::<crate::records::ShadowCascades>() as u64,
                ),
                binding::texture(
                    1,
                    Stages::COMPUTE,
                    wgpu::TextureSampleType::Depth,
                    Dim::D2Array,
                ),
                binding::sampler(2, Stages::COMPUTE, wgpu::SamplerBindingType::Comparison),
            ]);
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("view volume Sun comparison"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                compare: Some(wgpu::CompareFunction::GreaterEqual),
                ..Default::default()
            })
        });
        if let Some((_, cloud)) = cloud {
            scene_entries.extend([
                binding::uniform(3, Stages::COMPUTE, super::UNIFORM_BYTES),
                binding::sampler(4, Stages::COMPUTE, wgpu::SamplerBindingType::Filtering),
                read_texture(13, Dim::D2),
                binding::uniform(14, Stages::COMPUTE, cloud.uniform.size()),
            ]);
        }
        if cloud.is_some_and(|(_, cloud)| cloud.band.is_some()) {
            scene_entries.extend(crate::atmosphere::ground_band::GroundBandPrepared::layout(
                Stages::COMPUTE,
            ));
        }
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view volume shadow providers"),
            entries: &scene_entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("view volume integration"),
            bind_group_layouts: &[Some(&layout), Some(&scene_layout)],
            immediate_size: 0,
        });
        let module = Shader::ViewVolume(
            cascades,
            CloudVariant::from_provider(cloud.map(|(field, _)| field)),
            medium.is_some(),
            intervals / 2,
        )
        .compile(device, "view volume integration");
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("view volume integration"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("view_volume_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let multiple = air
            .textures
            .multiple_scattering
            .create_view(&wgpu::TextureViewDescriptor::default());
        let groups = std::array::from_fn(|previous| {
            let next = 1 - previous;
            let mut entries = [
                buffer_entry(0, &air.uniform),
                texture_entry(1, &air.transmittance_view),
                texture_entry(2, &multiple),
                texture_entry(3, &air.aerial_views[0]),
                texture_entry(4, &air.aerial_views[1]),
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&air.aerial_sampler),
                },
                super::aerosol::entry(air.aerosol.as_ref().expect("view volume declares aerosol")),
                texture_entry(9, &history_views[next][0]),
                texture_entry(10, &history_views[next][1]),
                texture_entry(11, &history_views[previous][0]),
                texture_entry(12, &history_views[previous][1]),
                buffer_entry(15, &motion),
            ]
            .into_iter()
            .chain(air.night.bindings())
            .collect::<Vec<_>>();
            let medium_view = medium.as_ref().map(super::medium::array_view);
            if let Some(view) = &medium_view {
                entries.push(texture_entry(16, view));
            }
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("view volume integration inputs"),
                layout: &layout,
                entries: &entries,
            })
        });
        Self {
            pipeline,
            scene_layout,
            histories,
            compare,
            groups,
            motion,
            medium,
            intervals,
            submitted: None,
            pending: None,
            phase: 0,
        }
    }

    pub(crate) fn prepare_scene_group(
        &self,
        device: &wgpu::Device,
        shadow: Option<(&wgpu::Buffer, &wgpu::TextureView)>,
        cloud: Option<&CloudPrepared>,
        observer: (&wgpu::Buffer, &wgpu::Sampler),
    ) -> wgpu::BindGroup {
        let mut entries = Vec::new();
        if let Some(((samples, maps), compare)) = shadow.zip(self.compare.as_ref()) {
            entries.extend([
                buffer_entry(0, samples),
                texture_entry(1, maps),
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(compare),
                },
            ]);
        }
        if let Some(cloud) = cloud {
            entries.extend([
                buffer_entry(3, observer.0),
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(observer.1),
                },
                texture_entry(13, &cloud.view),
                buffer_entry(14, &cloud.uniform),
            ]);
        }
        if let Some(band) = cloud.and_then(|cloud| cloud.band.as_ref()) {
            entries.extend(band.entries());
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view volume shadow provider inputs"),
            layout: &self.scene_layout,
            entries: &entries,
        })
    }

    pub(super) fn update(
        &mut self,
        queue: &wgpu::Queue,
        environment_epoch: EnvironmentEpoch,
        provider_epochs: [Option<u64>; 2],
        view: &SyntheticView,
    ) {
        let previous = self.submitted.as_ref().filter(|frame| {
            frame.environment_epoch == environment_epoch
                && frame.provider_epochs == provider_epochs
                && frame.view.history_epoch() == view.history_epoch()
                && !view.history_reset()
        });
        let previous_view = previous.map_or(view, |frame| &frame.view);
        let eye = glam::Vec3::from_array(view.camera_position());
        let world = Mat4::from_cols_array_2d(&view.inverse_view_proj_columns())
            * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let forward = (world.truncate() / world.w - eye).normalize_or_zero();
        let phase = if previous.is_some() { self.phase } else { 0 };
        let record = VolumeHistory {
            previous_view_projection: previous_view.view_proj_columns(),
            previous_eye_valid: glam::Vec3::from_array(previous_view.camera_position())
                .extend(f32::from(previous.is_some()))
                .to_array(),
            current_view_forward_phase: forward.extend(f32::from(phase)).to_array(),
        };
        queue.write_buffer(&self.motion, 0, bytemuck::bytes_of(&record));
        self.pending = Some(HistoryFrame {
            view: *view,
            environment_epoch,
            provider_epochs,
        });
    }

    pub(super) fn submitted(&mut self) {
        if let Some(frame) = self.pending.take() {
            self.submitted = Some(frame);
            self.phase = self.phase.wrapping_add(1);
        }
    }

    pub(in crate::atmosphere) fn graph_reads_and_writes(
        &self,
        graph: &mut FrameGraph,
    ) -> Result<Vec<Access>, GraphError> {
        let motion = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: MOTION_BYTES,
                usage: self.motion.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let mut accesses = vec![Access::new(
            motion,
            0..MOTION_BYTES,
            AccessKind::UniformRead,
        )];
        for (side, kind) in [AccessKind::SampledRead, AccessKind::StorageWrite]
            .into_iter()
            .enumerate()
        {
            for (band, texture) in self.histories[side].iter().enumerate() {
                let desc = ResourceDescriptor::of_texture(texture);
                let resource = graph.resource(
                    desc,
                    ResourceScope::History {
                        owner: 0x564f_4c00
                            + u64::try_from(side * 2 + band).expect("four history images"),
                        epoch: 0,
                        initialized: side == 0,
                    },
                )?;
                accesses.push(Access::new(
                    resource,
                    0..u64::from(texture.depth_or_array_layers()),
                    kind,
                ));
            }
        }
        Ok(accesses)
    }

    pub(crate) fn dispatch(&self, compute: &mut wgpu::ComputePass<'_>, scene: &wgpu::BindGroup) {
        let previous = usize::from(self.phase & 1);
        let edge = crate::shader_constants::AERIAL_WORKGROUP_EDGE;
        compute.set_pipeline(&self.pipeline);
        compute.set_bind_group(0, &self.groups[previous], &[]);
        compute.set_bind_group(1, scene, &[]);
        compute.dispatch_workgroups(WIDTH.div_ceil(edge), HEIGHT.div_ceil(edge), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_history_layout_and_portable_sources_validate() {
        for cascades in [false, true] {
            let module = wgpu::naga::front::wgsl::parse_str(&source(
                cascades,
                CloudVariant::Absent,
                false,
                NEAR_INTERVALS,
            ))
            .expect("view volume WGSL");
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .expect("portable view volume");
            let ty = module
                .types
                .iter()
                .find_map(|(_, ty)| (ty.name.as_deref() == Some("VolumeHistory")).then_some(ty))
                .expect("history record");
            let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
                panic!("history layout");
            };
            assert_eq!(u64::from(*span), MOTION_BYTES);
            assert_eq!(
                members
                    .iter()
                    .map(|member| member.offset as usize)
                    .collect::<Vec<_>>(),
                [
                    std::mem::offset_of!(VolumeHistory, previous_view_projection),
                    std::mem::offset_of!(VolumeHistory, previous_eye_valid),
                    std::mem::offset_of!(VolumeHistory, current_view_forward_phase),
                ]
            );
        }
    }
}
