//! One real half-extent Metal ray behind three distances in one depth block.
mod ground_support;
#[expect(dead_code, reason = "the other cloud tests share this fixture")]
mod scene_fixture;

use thrum_render::{
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
    runtime::FrameError,
};
use thrum_test_support::readback::Mip;

#[test]
fn one_trace_keeps_the_middle_stop_on_both_sides_of_the_bank() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    let state = ground_support::state(0.0, 1.0);
    for (centre, expected_middle_clear) in [(570.0, false), (800.0, true)] {
        let ground = ground_support::fixture(
            thrum_cloud_model::Fog {
                centre: [centre, 0.0],
                radius: [42.0, 120.0],
                liquid_water: 0.000_120,
                ..ground_support::fog(true)
            },
            false,
        );
        let (_host, input, _) = ground_support::render_ground(
            &gpu,
            &ground,
            &state,
            45.0,
            12.0,
            Some(Box::<DepthSteps>::default()),
        );
        let products = input.sky_products().expect("submitted Sky products");
        assert_eq!(products.trace_stride, 1);
        let texel = 8 + 4 * 16;
        let transmittance =
            |slot| ground_support::read(&gpu, &products.transport, Mip::layer(0, slot))[texel][3];
        let stops = [0, 1, 2, 3].map(transmittance);
        assert_eq!(stops[0], 1.0, "nearest depth has clear air only");
        assert_eq!(stops[3], 1.0, "duplicate nearest depth");
        assert!(
            stops[2] < 0.70,
            "farthest stop includes the whole bank: {stops:?}"
        );
        if expected_middle_clear {
            assert_eq!(stops[1], 1.0, "bank behind the middle pixel: {stops:?}");
        } else {
            assert_eq!(stops[1], stops[2], "both pixels behind the bank: {stops:?}");
        }
        let stop_metres: [f32; 4] = [0, 1, 2, 3].map(|slot| {
            ground_support::read(&gpu, &products.front, Mip::layer(0, slot))[texel][3] * 1000.0
        });
        for (actual, expected) in stop_metres.into_iter().zip([420.0, 700.0, 920.0, 420.0]) {
            assert!((actual - expected).abs() < 2.0, "stops {stop_metres:?}");
        }
        eprintln!("bank at {centre:.0} m, stops {stop_metres:?} m, transmission {stops:?}");
    }
}

#[derive(Default)]
struct DepthSteps(Option<wgpu::RenderPipeline>);

impl SceneContribution for DepthSteps {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let device = surface.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("depth steps"),
            source: wgpu::ShaderSource::Wgsl(
                r"
                @vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
                    let points = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                    return vec4(points[i], 0.0, 1.0);
                }
                @fragment fn fragment(@builtin(position) p: vec4<f32>) -> @builtin(frag_depth) f32 {
                    let pixel = vec2<u32>(p.xy);
                    let slot = pixel.x % 2u + 2u * (pixel.y % 2u);
                    let ranges = array(420.0, 700.0, 920.0, 420.0);
                    let distance = ranges[slot];
                    return (1.0 / distance - 1.0 / 4000.0) / (1.0 / 0.1 - 1.0 / 4000.0);
                }"
                .into(),
            ),
        });
        let targets = surface.color_formats.map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            })
        });
        self.0 = Some(device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("depth steps"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        }));
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(self.0.as_ref().expect("prepared depth steps"));
        pass.draw(0..3, 0..1);
    }
}
