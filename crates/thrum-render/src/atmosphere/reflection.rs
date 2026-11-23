//! Disk-free sky reflection products. All passes record into the existing
//! frame graph; generation/profile validity is owned by AtmospherePrepared.

use crate::error::RenderError;
use crate::pass_key::AtmosphereStage;
use crate::shaders::wgsl;

pub(super) const CUBE_SIZE: u32 = 64;
pub(super) const MIP_COUNT: u32 = 7;
pub(super) const DFG_SIZE: u32 = 65;
// Mip zero's minimum-roughness lobe is narrower than a cube texel.
// Allocate the quadrature to the wider kernels that expose discrete taps
// against the high-contrast twilight horizon.
const PREFILTER_COUNTS: [u32; MIP_COUNT as usize] = [8, 512, 1024, 512, 512, 512, 512];
const PREFILTER_CAPACITY: u32 = 1024;
const CUBE_TEXELS_PER_GROUP: u32 = 16;
const DFG_WORKGROUP_EDGE: u32 = 8;
const PREFILTER_WORKGROUP: u32 = 64;
pub(super) const DFG_SAMPLES: u32 = 1024;
pub(super) const SAMPLE_BYTES: u64 = prefilter_sample_count() as u64 * 16;

const fn prefilter_sample_count() -> u32 {
    let mut total = 0;
    let mut mip = 0;
    while mip < PREFILTER_COUNTS.len() {
        let count = PREFILTER_COUNTS[mip];
        assert!(count > 0 && count <= PREFILTER_CAPACITY);
        total += count;
        mip += 1;
    }
    total
}

#[derive(Clone)]
pub(super) struct ReflectionPrepared {
    pub(super) cube: wgpu::Texture,
    pub(super) dfg: wgpu::Texture,
    cube_view: wgpu::TextureView,
    dfg_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    cube_pipeline: wgpu::ComputePipeline,
    cube_groups: [wgpu::BindGroup; MIP_COUNT as usize],
    dfg_pipeline: wgpu::ComputePipeline,
    dfg_group: wgpu::BindGroup,
    samples_pipeline: wgpu::ComputePipeline,
    samples_group: wgpu::BindGroup,
}

impl ReflectionPrepared {
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        uniform: &wgpu::Buffer,
        charts: &[wgpu::BindGroupEntry<'_>],
        sky_sampler: &wgpu::Sampler,
    ) -> Self {
        let cube = texture(device, "atmosphere specular cube", CUBE_SIZE, 6, MIP_COUNT);
        let dfg = texture(device, "atmosphere DFG", DFG_SIZE, 1, 1);
        let samples = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere prepared GGX samples"),
            size: SAMPLE_BYTES,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let cube_view = cube.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let dfg_view = dfg.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atmosphere specular trilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_max_clamp: (MIP_COUNT - 1) as f32,
            ..Default::default()
        });
        let cube_pipeline = pipeline(
            device,
            "atmosphere sky specular prefilter",
            "reflection_main",
            crate::shaders::Shader::AtmosphereReflection,
        );
        let cube_layout = cube_pipeline.get_bind_group_layout(0);
        let cube_groups = std::array::from_fn(|mip| {
            let output = cube.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_mip_level: u32::try_from(mip).expect("seven prepared mips"),
                mip_level_count: Some(1),
                ..Default::default()
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("atmosphere specular mip"),
                layout: &cube_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sky_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&output),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: samples.as_entire_binding(),
                    },
                ]
                .into_iter()
                .chain(charts.iter().cloned())
                .collect::<Vec<_>>(),
            })
        });
        let dfg_pipeline = pipeline(
            device,
            "atmosphere DFG integration",
            "dfg_main",
            crate::shaders::Shader::AtmosphereDfg,
        );
        let dfg_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atmosphere DFG output"),
            layout: &dfg_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dfg_view),
            }],
        });
        let samples_pipeline = pipeline(
            device,
            "atmosphere GGX sample preparation",
            "prefilter_samples_main",
            crate::shaders::Shader::AtmosphereDfg,
        );
        let samples_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atmosphere GGX sample output"),
            layout: &samples_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                resource: samples.as_entire_binding(),
            }],
        });
        Self {
            cube,
            dfg,
            cube_view,
            dfg_view,
            sampler,
            cube_pipeline,
            cube_groups,
            dfg_pipeline,
            dfg_group,
            samples_pipeline,
            samples_group,
        }
    }

    pub(super) fn sampled(&self) -> (&wgpu::TextureView, &wgpu::TextureView, &wgpu::Sampler) {
        (&self.cube_view, &self.dfg_view, &self.sampler)
    }

    pub(super) fn encode(
        &self,
        stage: AtmosphereStage,
        pass: &mut wgpu::ComputePass<'_>,
    ) -> Result<(), RenderError> {
        if stage == AtmosphereStage::Dfg {
            pass.set_pipeline(&self.dfg_pipeline);
            pass.set_bind_group(0, &self.dfg_group, &[]);
            let groups = DFG_SIZE.div_ceil(DFG_WORKGROUP_EDGE);
            pass.dispatch_workgroups(groups, groups, 1);
            pass.set_pipeline(&self.samples_pipeline);
            pass.set_bind_group(0, &self.samples_group, &[]);
            pass.dispatch_workgroups(
                PREFILTER_CAPACITY.div_ceil(PREFILTER_WORKGROUP),
                MIP_COUNT,
                1,
            );
        } else {
            let AtmosphereStage::Reflection(mip) = stage else {
                return Err(RenderError::GraphPassMismatch);
            };
            let group = self
                .cube_groups
                .get(mip as usize)
                .ok_or(RenderError::GraphPassMismatch)?;
            pass.set_pipeline(&self.cube_pipeline);
            pass.set_bind_group(0, group, &[]);
            let width = CUBE_SIZE >> mip;
            pass.dispatch_workgroups(width.div_ceil(CUBE_TEXELS_PER_GROUP), width, 6);
        }
        Ok(())
    }
}

fn shader_constants() -> String {
    use std::fmt::Write as _;
    let mut source = format!(
        "const REFLECTION_SIZE:u32={CUBE_SIZE}u;\nconst REFLECTION_MIPS:u32={MIP_COUNT}u;\nconst DFG_SAMPLES:u32={DFG_SAMPLES}u;\nconst CUBE_TEXELS_PER_GROUP:u32={CUBE_TEXELS_PER_GROUP}u;\nconst PREFILTER_SPANS=array<vec2<u32>,{MIP_COUNT}>(\n"
    );
    let mut offset = 0;
    for count in PREFILTER_COUNTS {
        writeln!(source, "vec2<u32>({offset}u,{count}u),").expect("write WGSL constants");
        offset += count;
    }
    source.push_str(");\n");
    source
}

fn texture(device: &wgpu::Device, label: &str, size: u32, layers: u32, mips: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: layers,
        },
        mip_level_count: mips,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn pipeline(
    device: &crate::runtime::DeviceSite,
    label: &str,
    entry: &str,
    source: crate::shaders::Shader,
) -> wgpu::ComputePipeline {
    let module = source.compile(device, label);
    device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

pub(crate) fn cube_source() -> String {
    let constants = shader_constants();
    format!(
        "{constants}\n{}\n{}\n{}",
        concat!(
            wgsl!("atmosphere/storage.wgsl"),
            "\n",
            concat!(
                wgsl!("atmosphere/geometry.wgsl"),
                "\n",
                wgsl!("atmosphere/common.wgsl")
            )
        ),
        wgsl!("atmosphere/brdf.wgsl"),
        wgsl!("atmosphere/reflection.wgsl")
    )
}

pub(crate) fn dfg_source() -> String {
    format!(
        "const DFG_WORKGROUP_EDGE: u32 = {DFG_WORKGROUP_EDGE}u;
const PREFILTER_WORKGROUP: u32 = {PREFILTER_WORKGROUP}u;
{}
{}
{}",
        shader_constants(),
        wgsl!("atmosphere/brdf.wgsl"),
        wgsl!("atmosphere/dfg.wgsl")
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(in crate::atmosphere) mod tests;
