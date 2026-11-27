//! A registered sky overlay written by a draw uses the usual atmosphere refresh.
use super::*;

const PASS: &str = "draw sky overlay";
const SHADER: &str = r"
@group(0) @binding(0) var overlay: texture_storage_2d_array<rgba16float, write>;

struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) @interpolate(flat) cloudy: u32,
};

@vertex fn vertex(@builtin(vertex_index) i: u32, @builtin(instance_index) cloudy: u32) -> Vertex {
    let corner = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    return Vertex(vec4f(corner * 2.0 - 1.0, 0.0, 1.0), cloudy);
}

@fragment fn paint(v: Vertex) -> @location(0) vec4f {
    let value = select(vec4f(0, 0, 0, 1), vec4f(64, 48, 32, 0.5), v.cloudy != 0u);
    for (var face = 0; face < 6; face++) {
        textureStore(overlay, vec2i(v.position.xy), face, value);
    }
    return vec4f(1);
}
";

#[derive(Default)]
struct OverlayDraw {
    graph: EffectGraph,
    prepared: Option<(EffectDrawTarget, wgpu::RenderPipeline, wgpu::BindGroup)>,
}

impl SceneContribution for OverlayDraw {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let overlay = surface.sky_overlay_target().unwrap();
        let colour = surface.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(PASS),
            size: wgpu::Extent3d {
                width: overlay.width(),
                height: overlay.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target = EffectDrawTarget::new(PASS, &colour, 0, 0, wgpu::Color::BLACK, false, surface)
            .map_err(FrameError::EffectGraph)?;
        let shader = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(PASS),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let pipe = pipeline(surface, &shader, "paint", &[Some(FORMAT.into())], None);
        let view = overlay.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let group = surface
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(PASS),
                layout: &pipe.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
        self.prepared = Some((target, pipe, group));
        self.graph = EffectGraph {
            resources: [overlay, colour]
                .iter()
                .enumerate()
                .map(|(index, texture)| {
                    (
                        ResourceDescriptor::of_texture(texture),
                        ResourceScope::Persistent {
                            owner: index as u64,
                            epoch: 0,
                            initialized: true,
                        },
                    )
                })
                .collect(),
            passes: vec![EffectPass {
                name: PASS.into(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses: vec![
                    EffectAccess {
                        resource: 0,
                        range: 0..6,
                        kind: AccessKind::StorageWrite,
                        view: None,
                        history: None,
                    },
                    EffectAccess {
                        resource: 1,
                        range: 0..1,
                        kind: AccessKind::AttachmentClear,
                        view: None,
                        history: None,
                    },
                ],
            }],
            sky_overlays: vec![0],
            ..Default::default()
        };
        Ok(())
    }

    fn effect_graph(&self) -> EffectGraph {
        self.graph.clone()
    }

    fn effect_draw_target(&self, _index: usize) -> Option<&EffectDrawTarget> {
        self.prepared.as_ref().map(|(target, _, _)| target)
    }

    fn effect_runs(&self, _index: usize, frame: &ContributionFrame) -> bool {
        frame.time.tick != 1
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn draw_effect(
        &mut self,
        _: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        let (_, pipe, group) = self.prepared.as_ref().unwrap();
        pass.set_pipeline(pipe);
        pass.set_bind_group(0, group, &[]);
        let instance = u32::from(frame.time.tick != 0);
        pass.draw(0..3, instance..instance + 1);
    }
}

#[test]
fn simulate_draw_gpu_refreshes_registered_sky_overlay_only_when_written() {
    let Some(gpu) = crate::test_device::open(
        PASS,
        Some(wgpu::Backend::Metal),
        DeviceProfile::Portable,
        &DeviceRestriction::default(),
    ) else {
        return;
    };
    pollster::block_on(async {
        let lighting = FrameLighting::Atmosphere(
            crate::atmosphere::AtmosphereFrame::new(
                [0.75_f32.sqrt(), 0.5, 0.0],
                90_000.0,
                [1.0; 3],
                40.0,
                1,
            )
            .unwrap(),
        );
        let mut host = Box::pin(host_with_lighting(&gpu, lighting, true)).await;
        let validation = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        host.set_scene_contribution(Box::<OverlayDraw>::default())
            .unwrap();
        enable_capture(&mut host);
        let drawable = output(&gpu.device);
        let lighting_passes = [
            "atmosphere chart overlay lighting",
            "atmosphere diffuse irradiance",
        ]
        .map(str::to_owned)
        .into_iter()
        .chain((0..7).map(|mip| format!("atmosphere reflection mip {mip}")))
        .collect::<Vec<_>>();
        for tick in 0..=2 {
            let (image, sample) = submit(&mut host, &drawable, tick);
            let timing = sample.gpu_pass_ns.unwrap();
            let ran = |name: &str| matches!(timing.execution(name), Some(PassExecution::Timed(_)));
            assert_eq!(ran("effect 0: draw sky overlay"), tick != 1);
            for name in &lighting_passes {
                assert_eq!(ran(name), tick != 1, "tick {tick}: {name}");
            }
            host.recycle_submitted_image(image);
        }
        assert!(validation.pop().await.is_none());
    });
}
