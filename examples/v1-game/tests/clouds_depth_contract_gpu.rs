//! The real V1 after-opaque trace sees a foreground occluder uncover a farther surface.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};
use thrum_render::{
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
    runtime::{FrameConfig, FrameError, SurfaceOutput},
};
use thrum_test_support::{
    Opened, Restriction,
    readback::{self, Mip},
};
use v1_game::{CloudSelection, GameHost, GameSetup, GroundWeather, SceneCamera};

const EXTENT: [u32; 2] = [32, 18];

#[test]
#[expect(
    clippy::cast_precision_loss,
    reason = "the test pixel coordinates are at most 32"
)]
fn metal_uncovered_pixels_use_their_current_surface_stops_and_a_cut_refills_them() {
    let Some(Opened {
        device,
        queue,
        capabilities,
        ..
    }) = thrum_test_support::open(
        "V1 moving cloud occluder",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &Restriction::default(),
    )
    else {
        return;
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cloud occluder output"),
        size: wgpu::Extent3d {
            width: EXTENT[0],
            height: EXTENT[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut host = pollster::block_on(
        GameSetup {
            clouds: CloudSelection::Ground(GroundWeather::HandoverBank(450)),
            cloud_weather: v1_game::clouds::WeatherPreset::Clear.parameters(),
            ..GameSetup::default()
        }
        .prepare(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(thrum_render::present::SurfaceProfile {
                    format: target.format(),
                    extent: EXTENT,
                }),
            },
        ),
    )
    .expect("prepared V1");
    let eye = [-485.0, 100.0, 810.0];
    let camera = SceneCamera::perspective(eye, [-450.0, 100.0, 0.0], 0.35, 0.1, 4000.0)
        .expect("bank camera");
    let position = Arc::new(AtomicU32::new(0));
    host.set_game_contribution(Box::new(Occluder {
        position: Arc::clone(&position),
        pipeline: None,
    }))
    .expect("opaque mover");
    host.set_scene_camera(camera).expect("starting camera");
    let refresh = host
        .quality_knobs()
        .into_iter()
        .find(|knob| knob.name.ends_with("cloud refresh exponent"))
        .expect("cloud refresh control")
        .name;
    host.set_game_quality_knob(&refresh, 1)
        .expect("requested sparse profile");
    let submit =
        |host: &mut thrum_render::runtime::FrameHost<v1_game::GameWorld, v1_game::GameScene>,
         frame| {
            assert!(
                host.drive(
                    Duration::from_millis(frame),
                    Some(&SurfaceOutput {
                        view: &view,
                        extent: EXTENT
                    })
                )
                .expect("drive V1")
                .submitted
            );
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("Metal complete");
            while host.poll_completed().expect("retire V1 frame").is_some() {}
            host.cloud_sky_products().expect("submitted cloud layer")
        };
    let occluded = submit(&mut host, 0);
    assert!(!occluded.history_valid);
    let read = |texture| readback::texture(&device, &queue, texture, Mip::level(0), ..);
    let previous = read(&occluded.front);
    position.store(1, Ordering::Relaxed);
    let uncovered = submit(&mut host, 25);
    assert!(uncovered.history_valid);
    assert_eq!(uncovered.refresh_stride, 2);
    assert_eq!(uncovered.trace_stride, 1);
    let resolved = read(&uncovered.front);
    let fresh = read(&uncovered.fresh_front);
    let half = |bytes: &[u8], pixel: u32, channel: u32| {
        let x = pixel % EXTENT[0];
        let y = pixel / EXTENT[0];
        let slot = x % 2 + 2 * (y % 2);
        let index = (((slot * (EXTENT[1] / 2) + y / 2) * (EXTENT[0] / 2) + x / 2) * 8 + channel * 2)
            as usize;
        half::f16::from_le_bytes([bytes[index], bytes[index + 1]]).to_f32()
    };
    let inverse = glam::Mat4::from_cols_array_2d(&uncovered.raster_view_projection).inverse();
    let depth = (1.0 / 920.0 - 1.0 / 4000.0) / (1.0 / 0.1 - 1.0 / 4000.0);
    let mut mist_covered = 0;
    for y in 0..EXTENT[1] {
        for x in 8..16 {
            let pixel = y * EXTENT[0] + x;
            assert!(half(&previous, pixel, 3) < 0.36);
            let distance = half(&fresh, pixel, 3);
            let uv = (glam::Vec2::new(x as f32, y as f32) + 0.5)
                / glam::Vec2::from_array(EXTENT.map(|side| side as f32));
            let hit = inverse
                * glam::Vec4::new(uv.x.mul_add(2.0, -1.0), uv.y.mul_add(-2.0, 1.0), depth, 1.0);
            let expected = (hit.truncate() / hit.w - glam::Vec3::from_array(eye)).length() * 0.001;
            assert!(
                (distance - expected).abs() < 0.002,
                "fresh stop at {x},{y}: {distance} km, expected {expected}"
            );

            assert_eq!(
                half(&resolved, pixel, 3),
                distance,
                "carried or borrowed stop at {x},{y}"
            );
            let opacity = half(&fresh, pixel, 1);
            assert_eq!(half(&resolved, pixel, 2), opacity);
            mist_covered += u32::from(opacity > 0.02);
        }
    }
    assert!(mist_covered > 0, "the uncovered surface must show far mist");
    host.set_scene_camera(camera).expect("request cut");
    let cut = submit(&mut host, 50);
    assert!(!cut.history_valid);
    assert_eq!(
        read(&cut.transport),
        read(&cut.fresh_transport),
        "cut composes the fresh stops"
    );
}

struct Occluder {
    position: Arc<AtomicU32>,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl SceneContribution for Occluder {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let device = surface.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("moving cloud occluder"),
            source: wgpu::ShaderSource::Wgsl(r"
                struct Varying {
                    @builtin(position) position: vec4<f32>,
                    @location(0) @interpolate(flat) mover: u32,
                };
                @vertex fn vertex(@builtin(vertex_index) i: u32, @builtin(instance_index) mover: u32) -> Varying {
                    let points = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                    return Varying(vec4(points[i], 0.0, 1.0), mover);
                }
                @fragment fn fragment(v: Varying) -> @builtin(frag_depth) f32 {
                    let left = 8u + 8u * v.mover;
                    let x = u32(v.position.x);
                    let distance = select(920.0, 340.0, x >= left && x < left + 8u);
                    return (1.0 / distance - 1.0 / 4000.0) / (1.0 / 0.1 - 1.0 / 4000.0);
                }".into()),
        });
        self.pipeline = Some(device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("moving cloud occluder"),
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
                targets: &surface.color_formats.map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::empty(),
                    })
                }),
            }),
            multiview_mask: None,
            cache: None,
        }));
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let position = self.position.load(Ordering::Relaxed);
        pass.set_pipeline(self.pipeline.as_ref().expect("prepared mover"));
        pass.draw(0..3, position..position + 1);
    }
}
