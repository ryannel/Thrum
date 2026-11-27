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

#[test]
fn elevated_cloud_uses_the_near_view_volume_before_the_surface() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    for remote_ridge in [false, true] {
        let ground =
            ground_support::fixture_at_height(ground_support::fog(false), remote_ridge, 890.0);
        for coverage in [0.0, 1.0] {
            let state = ground_support::state(coverage, 0.0);
            let (_host, input, _) = ground_support::render_ground(
                &gpu,
                &ground,
                &state,
                35.0,
                930.0,
                Some(Box::new(DepthSteps::new([120; 4]))),
            );
            let medium = input
                .ground_condensate_products()
                .expect("submitted medium");
            let products = input.sky_products().expect("submitted Sky products");
            let depth = ground_support::read(&gpu, &products.fresh_front, Mip::layer(0, 0));
            assert!((depth[72][3] * 1000.0 - 120.0).abs() < 1.0);
            let min_transmittance = (0..4)
                .flat_map(|slot| {
                    ground_support::read(&gpu, &products.fresh_transport, Mip::layer(0, slot))
                })
                .map(|value| value[3])
                .fold(1.0_f32, f32::min);
            assert_eq!(
                min_transmittance, 1.0,
                "near cloud belongs to the view volume"
            );
            // The first cell layer is wholly in front of the 120 m depth writer.
            let extinction =
                ground_support::read(&gpu, &medium.medium.coefficients, Mip::layer(0, 0));
            let max_extinction = extinction
                .iter()
                .map(|cell| cell[0])
                .fold(0.0_f32, f32::max);
            if coverage > 0.0 {
                assert!(
                    max_extinction > 0.0,
                    "elevated cloud contributes to the near field"
                );
            } else {
                assert_eq!(max_extinction, 0.0, "clear control has no cloud or fog");
            }
            eprintln!(
                "remote ridge {remote_ridge}, coverage {coverage}, max medium extinction {max_extinction}, min far transmittance {min_transmittance}"
            );
        }
    }
}

#[test]
fn raising_a_ridge_behind_the_camera_preserves_far_cloud_transmittance() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    let state = ground_support::state(1.0, 0.0);
    let model = thrum_cloud_model::Profile {
        erosion: 0.61,
        ..ground_support::PROFILE
    };
    let kind = thrum_cloud_model::Type {
        deck: 0.5,
        ..ground_support::KIND
    };
    // The ridge at x = -200 m crosses the 900 m base. The camera looks
    // in +x, so the hill cannot clip the cloud or the 3,500 m depth stop.
    for (ground_height, view) in [(0.0, [40.0, 40.5]), (890.0, [975.0, 975.0])] {
        let read_stops = |ridge| {
            let ground = ground_support::surface_fixture(ground_support::fog(false), |x| {
                if x == 3 { ridge } else { ground_height }
            });
            let (_host, input, _) = ground_support::render_layer(
                &gpu,
                &ground,
                &state,
                35.0,
                view,
                model,
                kind,
                Some(Box::new(DepthSteps::new([3500; 4]))),
            );
            let sky = input.sky_products().expect("submitted far trace");
            let front = ground_support::read(&gpu, &sky.fresh_front, Mip::layer(0, 0));
            assert!(
                (front[72][3] - 3.5).abs() < 0.01,
                "opaque stop in kilometres"
            );
            ground_support::read(&gpu, &sky.fresh_transport, Mip::level(0))
        };
        let below = read_stops(899.0);
        let above = read_stops(901.0);
        assert_eq!(below.len(), above.len());
        let min_transmittance = below.iter().map(|cell| cell[3]).fold(1.0_f32, f32::min);
        assert!(
            min_transmittance < 0.99,
            "the far trace must see cloud: {min_transmittance}"
        );
        let max_delta = below
            .iter()
            .zip(&above)
            .map(|(a, b)| (a[3] - b[3]).abs())
            .fold(0.0_f32, f32::max);
        eprintln!(
            "eye {} m, min far transmittance {min_transmittance}, maximum ridge-induced transmittance change {max_delta}",
            view[0]
        );
        assert_eq!(
            max_delta, 0.0,
            "ridge height must leave extinction unchanged"
        );
    }
}

struct DepthSteps {
    metres: [u32; 4],
    pipeline: Option<wgpu::RenderPipeline>,
}

impl DepthSteps {
    fn new(metres: [u32; 4]) -> Self {
        Self {
            metres,
            pipeline: None,
        }
    }
}

impl Default for DepthSteps {
    fn default() -> Self {
        Self::new([420, 700, 920, 420])
    }
}

impl SceneContribution for DepthSteps {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let device = surface.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("depth steps"),
            source: wgpu::ShaderSource::Wgsl(
                format!(r"
                @vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
                    let points = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                    return vec4(points[i], 0.0, 1.0);
                }}
                @fragment fn fragment(@builtin(position) p: vec4<f32>) -> @builtin(frag_depth) f32 {{
                    let pixel = vec2<u32>(p.xy);
                    let slot = pixel.x % 2u + 2u * (pixel.y % 2u);
                    let ranges = array({}.0, {}.0, {}.0, {}.0);
                    let distance = ranges[slot];
                    return (1.0 / distance - 1.0 / 4000.0) / (1.0 / 0.1 - 1.0 / 4000.0);
                }}", self.metres[0], self.metres[1], self.metres[2], self.metres[3])
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
        self.pipeline = Some(device.render(&wgpu::RenderPipelineDescriptor {
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
        pass.set_pipeline(self.pipeline.as_ref().expect("prepared depth steps"));
        pass.draw(0..3, 0..1);
    }
}
