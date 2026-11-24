//! Labelled analytic stand-in for the weather's medium and ground band.
use thrum_render::{
    atmosphere::{GroundBand, GroundBandMapping, SunOpticalDepth, ViewVolumeMediumTarget},
    binding,
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence, EffectGraph,
        EffectPass, SceneContribution, SceneContributions, SceneSurface, ViewVolumeResources,
    },
    error::RenderError,
    graph::{AccessKind, ResourceDescriptor, ResourceScope, TextureKind},
    runtime::FrameError,
};

pub(crate) const SELECTION: &str = "test-bank";
const BAND_EXTENT: [u32; 3] = [128, 112, 19];
const PASS: &str = "test bank medium coefficients and ground band";

pub(super) fn sun_path() -> Result<SunOpticalDepth, RenderError> {
    let region = [-142.0, -119.0, 114.0, 105.0];
    let band = GroundBand::gpu_target(
        BAND_EXTENT,
        GroundBandMapping {
            region,
            top_m: 20.0,
            border_m: 8.0,
            padding: [0.0; 2],
        },
    )?;
    // This stand-in has no upper cloud. The bank's top and sides taper to the
    // transparent 2D field and keep the same one-provider receiver operation.
    Ok(SunOpticalDepth::new([1, 1], &[0.0], 1_100.0, region, 0.001, 1)?.with_ground_band(band))
}

pub(crate) fn alongside(contribution: Box<dyn SceneContribution>) -> Box<dyn SceneContribution> {
    Box::new(SceneContributions::new(vec![
        contribution,
        Box::new(TestBank::default()),
    ]))
}

#[derive(Default)]
pub(crate) struct TestBank(Option<Prepared>);

struct Prepared {
    medium: ViewVolumeMediumTarget,
    band: wgpu::Texture,
    mapping: wgpu::Buffer,
    pipelines: [wgpu::ComputePipeline; 2],
    groups: [wgpu::BindGroup; 2],
}

fn access(resource: usize, kind: AccessKind, end: u64) -> EffectAccess {
    EffectAccess {
        resource,
        range: 0..end,
        kind,
        view: None,
        history: None,
    }
}

fn uniform_resource(buffer: &wgpu::Buffer) -> (ResourceDescriptor, ResourceScope) {
    (
        ResourceDescriptor::Buffer {
            bytes: buffer.size(),
            usage: buffer.usage(),
        },
        ResourceScope::Imported { initialized: true },
    )
}

impl SceneContribution for TestBank {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let (Some(medium), Some((band, mapping))) = (
            surface.view_volume_medium_target(),
            surface.ground_band_target(),
        ) else {
            return Err(FrameError::InvalidState(
                "test bank needs medium and ground band targets",
            ));
        };
        let device = surface.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    medium.shader_source(),
                    include_str!("test_bank.wgsl")
                )
                .into(),
            ),
        });
        let medium_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test bank sample points"),
            entries: &ViewVolumeMediumTarget::layout_entries(),
        });
        let stages = wgpu::ShaderStages::COMPUTE;
        let band_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test bank ground band"),
            entries: &[
                binding::uniform(0, stages, mapping.size()),
                binding::storage_texture(
                    1,
                    stages,
                    wgpu::StorageTextureAccess::WriteOnly,
                    band.format(),
                    wgpu::TextureViewDimension::D3,
                ),
            ],
        });
        let band_view = band.create_view(&wgpu::TextureViewDescriptor::default());
        let groups = [
            medium.bind(device, &medium_layout),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("test bank ground band"),
                layout: &band_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: mapping.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&band_view),
                    },
                ],
            }),
        ];
        let pipelines =
            [("coefficients", None), ("ground", Some(&band_layout))].map(|(entry, band)| {
                let groups = [Some(&medium_layout), band];
                let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(PASS),
                    bind_group_layouts: &groups[..if band.is_some() { 2 } else { 1 }],
                    immediate_size: 0,
                });
                device.compute(&wgpu::ComputePipelineDescriptor {
                    label: Some(PASS),
                    layout: Some(&layout),
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                })
            });
        self.0 = Some(Prepared {
            medium,
            band,
            mapping,
            pipelines,
            groups,
        });
        Ok(())
    }

    fn effect_graph(&self) -> EffectGraph {
        let Some(gpu) = &self.0 else {
            return EffectGraph::default();
        };
        let write = AccessKind::StorageWrite;
        let uniform = AccessKind::UniformRead;
        EffectGraph {
            resources: vec![
                (
                    ResourceDescriptor::of_texture(&gpu.medium.coefficients),
                    ResourceScope::Imported { initialized: false },
                ),
                (
                    ResourceDescriptor::Texture {
                        dimension: TextureKind::D3 {
                            depth: gpu.band.depth_or_array_layers(),
                        },
                        width: gpu.band.width(),
                        height: gpu.band.height(),
                        layers: 1,
                        mips: 1,
                        samples: 1,
                        format: gpu.band.format(),
                        usage: gpu.band.usage(),
                    },
                    ResourceScope::Imported { initialized: false },
                ),
                uniform_resource(&gpu.medium.observer),
                uniform_resource(&gpu.medium.history),
                uniform_resource(&gpu.mapping),
            ],
            passes: vec![EffectPass {
                name: PASS.into(),
                cadence: EffectCadence::EveryFrame,
                async_eligible: false,
                accesses: vec![
                    access(
                        0,
                        write,
                        u64::from(gpu.medium.coefficients.depth_or_array_layers()),
                    ),
                    access(1, write, 1),
                    access(2, uniform, gpu.medium.observer.size()),
                    access(3, uniform, gpu.medium.history.size()),
                    access(4, uniform, gpu.mapping.size()),
                ],
            }],
            opaque_reads: vec![access(1, AccessKind::SampledRead, 1)],
            view_volume_media: vec![ViewVolumeResources {
                coefficients: 0,
                ground_band: Some(1),
            }],
            ..EffectGraph::default()
        }
    }

    fn encode_effect(&mut self, _: usize, pass: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {
        let Some(gpu) = &self.0 else {
            return;
        };
        pass.set_pipeline(&gpu.pipelines[0]);
        pass.set_bind_group(0, &gpu.groups[0], &[]);
        let extent = gpu.medium.coefficients.size();
        pass.dispatch_workgroups(
            extent.width.div_ceil(8),
            extent.height.div_ceil(8),
            extent.depth_or_array_layers,
        );
        pass.set_pipeline(&gpu.pipelines[1]);
        pass.set_bind_group(1, &gpu.groups[1], &[]);
        let extent = gpu.band.size();
        pass.dispatch_workgroups(
            extent.width.div_ceil(8),
            extent.height.div_ceil(8),
            extent.depth_or_array_layers,
        );
    }
}
