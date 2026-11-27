//! The real V1 after-opaque trace sees a foreground occluder uncover a farther surface.

mod clouds_sky_contract;
#[expect(dead_code, reason = "the placement tests share this fixture")]
mod fixture;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};
use thrum_render::{
    contribution::{
        ContributionFrame, ContributionPasses, EffectGraph, SceneContribution, SceneContributions,
        SceneSurface, SecondaryResize, SecondaryResizeJob, SecondaryView, SecondaryViewRequest,
        SecondaryViewSlot, ViewAir,
    },
    runtime::{CaptureLimits, FrameConfig, FrameError, SurfaceOutput},
    synthetic::CaptureSource,
};
use thrum_test_support::{
    Opened, Restriction,
    creation_trace::{Creations, with_default},
    readback::{self, Mip},
};
use v1_game::{CloudSelection, GameHost, GameSetup, GroundWeather, SceneCamera};

const EXTENT: [u32; 2] = [32, 18];
const GROWN: [u32; 2] = [65, 37];

fn output_texture(device: &wgpu::Device, [width, height]: [u32; 2]) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cloud occluder output"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn stop_channel(bytes: &[u8], [width, height]: [u32; 2], x: u32, y: u32, channel: u32) -> f32 {
    let slot = x % 2 + 2 * (y % 2);
    let index = (((slot * height.div_ceil(2) + y / 2) * width.div_ceil(2) + x / 2) * 8
        + channel * 2) as usize;
    half::f16::from_le_bytes([bytes[index], bytes[index + 1]]).to_f32()
}

fn submitted_linear(
    host: &mut fixture::Host,
    device: &wgpu::Device,
) -> thrum_render::runtime::SubmittedImage {
    thrum_test_support::wait::until("submitted pre-tone image", Duration::from_secs(5), || {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal complete");
        host.try_submitted_image().expect("pre-tone readback")
    })
}

fn composed_channel(
    image: &thrum_render::runtime::SubmittedImage,
    x: u32,
    y: u32,
    channel: u32,
) -> f32 {
    assert_eq!(image.pixel_source, CaptureSource::SceneLinear);
    let offset = ((y * image.extent[0] + x) * 8 + channel * 2) as usize;
    half::f16::from_le_bytes([image.rgba[offset], image.rgba[offset + 1]]).to_f32()
}

fn wait_for_retirement(host: &mut fixture::Host, device: &wgpu::Device) {
    thrum_test_support::wait::until("retire submitted frames", Duration::from_secs(5), || {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal idle");
        host.poll_completed().expect("retire timing");
        (host.pending_frames() == 0).then_some(())
    });
}

#[test]
fn metal_uncover_cut_and_resize_use_current_surface_stops() {
    prove_surface_stops(false);
    prove_surface_stops(true);
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the test pixel coordinates are at most 32"
)]
fn prove_surface_stops(worker: bool) {
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
    let target = output_texture(&device, EXTENT);
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
    let occluder = Box::new(Occluder {
        position: Arc::clone(&position),
        pipeline: None,
    });
    let replaced = Arc::new(AtomicBool::new(false));
    if worker {
        host.set_game_contribution(Box::new(SceneContributions::new(vec![
            occluder,
            Box::new(SecondaryProbe {
                view: None,
                replaced: Arc::clone(&replaced),
            }),
        ])))
        .expect("opaque mover and secondary slot beside Ground");
    } else {
        host.set_game_contribution(occluder).expect("opaque mover");
    }
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
         frame,
         view: &wgpu::TextureView,
         extent| {
            assert!(
                host.drive(
                    Duration::from_millis(frame),
                    Some(&SurfaceOutput { view, extent })
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
    host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
        .expect("submitted transport tap");
    let occluded = submit(&mut host, 0, &view, EXTENT);
    let first_image = submitted_linear(&mut host, &device);
    host.recycle_submitted_image(first_image);
    assert!(!occluded.history_valid);
    let read =
        |texture: &wgpu::Texture| readback::texture(&device, &queue, texture, Mip::level(0), ..);
    let previous = read(&occluded.front);
    position.store(1, Ordering::Relaxed);
    let uncovered = submit(&mut host, 25, &view, EXTENT);
    assert!(uncovered.history_valid);
    assert_eq!(uncovered.refresh_stride, 2);
    assert_eq!(uncovered.trace_stride, 1);
    let resolved = read(&uncovered.front);
    let fresh = read(&uncovered.fresh_front);
    let fresh_transport = read(&uncovered.fresh_transport);
    let image = submitted_linear(&mut host, &device);
    let half = |bytes: &[u8], pixel, channel| {
        stop_channel(bytes, EXTENT, pixel % EXTENT[0], pixel / EXTENT[0], channel)
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
            assert!(
                (composed_channel(&image, x, y, 3) - half(&fresh_transport, pixel, 3)).abs()
                    < 0.002,
                "uncovered pixel {x},{y} must compose its own fresh stop"
            );
            let opacity = half(&fresh, pixel, 1);
            assert_eq!(half(&resolved, pixel, 2), opacity);
            mist_covered += u32::from(opacity > 0.02);
        }
    }
    assert!(mist_covered > 0, "the uncovered surface must show far mist");
    host.recycle_submitted_image(image);
    host.set_scene_camera(camera).expect("request cut");
    let cut = submit(&mut host, 50, &view, EXTENT);
    assert!(!cut.history_valid);
    assert_eq!(
        read(&cut.transport),
        read(&cut.fresh_transport),
        "cut composes the fresh stops"
    );

    let scale = host
        .quality_knobs()
        .into_iter()
        .find(|knob| knob.name.ends_with("cloud layer scale"))
        .expect("Ground scale");
    assert_eq!(scale.range, 100..=100);
    assert!(host.set_game_quality_knob(&scale.name, 50).is_err());

    wait_for_retirement(&mut host, &device);
    if worker {
        publish_worker_extent(&mut host);
    } else {
        assert!(host.resize_scene_surface(GROWN).expect("immediate grow"));
    }
    assert_eq!(host.contribution_error(), None);
    let grown_view =
        output_texture(&device, GROWN).create_view(&wgpu::TextureViewDescriptor::default());
    let grown = submit(&mut host, 75, &grown_view, GROWN);
    assert_grown_stops(&grown, eye, read);
    if worker {
        assert!(
            replaced.load(Ordering::Relaxed),
            "first frame reads the resized secondary slot"
        );
    }
}

#[expect(
    clippy::print_stderr,
    reason = "save publication counters in the Metal proof"
)]
fn publish_worker_extent(host: &mut fixture::Host) {
    let render = std::thread::current().id();
    let creations = Creations::default();
    let dispatch = creations.dispatch();
    let worker_dispatch = dispatch.clone();
    let worker = with_default(&dispatch, || {
        let inputs = host
            .extent_prepare_inputs(GROWN)
            .expect("new extent inputs");
        let (worker, candidate) = std::thread::spawn(move || {
            with_default(&worker_dispatch, || {
                (
                    std::thread::current().id(),
                    inputs
                        .build(&AtomicBool::new(false))
                        .expect("build new extent")
                        .expect("prepared extent"),
                )
            })
        })
        .join()
        .expect("worker completed");
        assert!(host.publish_extent(candidate));
        worker
    });
    assert!(
        creations.on(render).is_empty(),
        "render thread creations: {:?}",
        creations.on(render)
    );
    let events = creations.on(worker);
    for kind in [
        "Device::create_texture",
        "Texture::create_view",
        "Device::create_bind_group",
    ] {
        assert!(
            events.contains(&kind),
            "worker did not create {kind}: {events:?}"
        );
    }
    eprintln!(
        "Mac extent 32×18 -> 65×37: render created zero objects, worker recorded {} creations",
        events.len()
    );
}

struct SecondaryProbe {
    view: Option<SecondaryView>,
    replaced: Arc<AtomicBool>,
}

impl SceneContribution for SecondaryProbe {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![SecondaryViewSlot::new(1.0).expect("full-sized secondary slot")]
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.view = Some(surface.secondary_views[0].clone());
        Ok(())
    }

    fn secondary_resize_job(&self) -> Result<Option<SecondaryResizeJob>, FrameError> {
        Ok(Some(Box::new(|views| {
            let view = views[0].clone();
            Ok(SecondaryResize::new(
                EffectGraph::default(),
                move |owner: &mut Self| {
                    owner.view = Some(view);
                },
            ))
        })))
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        if frame.render_extent != GROWN {
            return;
        }
        let view = self.view.as_ref().expect("secondary binding");
        assert_eq!(view.maximum_extent(), GROWN);
        let [x, y, z] = frame.camera;
        let camera = thrum_render::synthetic::SyntheticView::perspective(
            [x, y + 10.0, z],
            [x, y + 10.0, z - 10.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            1000.0,
        )
        .expect("secondary camera");
        view.request(
            &SecondaryViewRequest::new(&camera, GROWN, None, ViewAir::Camera)
                .expect("view request"),
        )
        .expect("the first grown frame uses replacement slot bindings");
        self.replaced.store(true, Ordering::Relaxed);
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the test pixel coordinates are at most 65"
)]
fn assert_grown_stops(
    grown: &thrum_clouds::SkyProducts,
    eye: [f32; 3],
    read: impl Fn(&wgpu::Texture) -> Vec<u8>,
) {
    assert_eq!(grown.active_extent, GROWN.map(|side| side.div_ceil(2)));
    assert!(!grown.history_valid);
    for texture in [
        &grown.fresh_front,
        &grown.front,
        &grown.fresh_transport,
        &grown.transport,
    ] {
        assert_eq!([texture.width(), texture.height()], grown.active_extent);
        assert_eq!(texture.depth_or_array_layers(), 4);
    }
    let metadata = read(&grown.front);
    let fresh = read(&grown.fresh_front);
    let inverse = glam::Mat4::from_cols_array_2d(&grown.raster_view_projection).inverse();
    for y in 0..GROWN[1] {
        for x in 0..GROWN[0] {
            let plane = if (16..24).contains(&x) { 340.0 } else { 920.0 };
            let depth = (1.0 / plane - 1.0 / 4000.0) / (1.0 / 0.1 - 1.0 / 4000.0);
            let uv = (glam::Vec2::new(x as f32, y as f32) + 0.5)
                / glam::Vec2::from_array(GROWN.map(|side| side as f32));
            let hit = inverse
                * glam::Vec4::new(uv.x.mul_add(2.0, -1.0), uv.y.mul_add(-2.0, 1.0), depth, 1.0);
            let expected = (hit.truncate() / hit.w - glam::Vec3::from_array(eye)).length() * 0.001;
            let distance = stop_channel(&metadata, GROWN, x, y, 3);
            assert_eq!(
                distance.to_bits(),
                stop_channel(&fresh, GROWN, x, y, 3).to_bits()
            );
            assert!(
                (distance - expected).abs() < 0.002,
                "grown stop {x},{y}: {distance} vs {expected}"
            );
        }
    }
}

#[test]
fn metal_directed_cloud_keeps_its_far_stop_with_clear_ground_rows() {
    let Some(gpu) = thrum_test_support::open(
        "placed cloud behind mixed depth",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::with_placements(true, &[[20.0, -1100.0]], [0.0; 2]);
    let mut profile = fixture::Fixture::profile();
    for kind in &mut profile.types {
        kind.base = 125.0;
    }
    fixture.sky = fixture.sky.with_profile(profile);
    let mut host = pollster::block_on(
        fixture.host_with_clouds(&gpu, CloudSelection::Ground(GroundWeather::Clear)),
    );
    let position = Arc::new(AtomicU32::new(2));
    host.set_game_contribution(Box::new(Occluder {
        position: Arc::clone(&position),
        pipeline: None,
    }))
    .expect("mixed depth control");
    let camera = SceneCamera::perspective(
        [80.0, 320.0, 0.0],
        [80.0, 320.0, -1100.0],
        0.25,
        0.1,
        4000.0,
    )
    .expect("look through the placed slab");
    host.set_scene_camera(camera).expect("fixed camera");
    let mut frames = fixture::Frames::new(&gpu);
    frames.placed_submitted(&mut host, |report| {
        report.volume_instances == 1 && !report.history_invalid
    });
    let read_centre = |host: &fixture::Host| {
        let sky = host.cloud_sky_products().expect("submitted stops");
        let read = |texture| readback::texture(&gpu.device, &gpu.queue, texture, Mip::level(0), ..);
        let fresh = read(&sky.fresh_transport);
        let resolved = read(&sky.transport);
        let metadata = read(&sky.front);
        let x = fixture::EXTENT[0] / 2;
        let y = fixture::EXTENT[1] / 2;
        std::array::from_fn::<_, 4, _>(|slot| {
            let channel = |bytes| {
                stop_channel(
                    bytes,
                    fixture::EXTENT,
                    x + u32::try_from(slot % 2).unwrap(),
                    y + u32::try_from(slot / 2).unwrap(),
                    3,
                )
            };
            let transmission = channel(&resolved);
            assert_eq!(transmission.to_bits(), channel(&fresh).to_bits());
            (transmission, channel(&metadata))
        })
    };
    wait_for_retirement(&mut host, &gpu.device);
    host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
        .expect("submitted surface transport tap");
    frames.submit(&mut host);
    let level = read_centre(&host);
    let level_image = submitted_linear(&mut host, &gpu.device);
    assert!(
        level[2].0 < 0.2,
        "all-far control must see the slab: {level:?}"
    );
    position.store(3, Ordering::Relaxed);
    host.set_scene_camera(camera)
        .expect("same camera, fresh stops");
    frames.submit(&mut host);
    let mixed = read_centre(&host);
    let mixed_image = submitted_linear(&mut host, &gpu.device);
    for (slot, ((transmission, distance), expected)) in
        mixed.into_iter().zip([0.42, 0.70, 2.0, 0.42]).enumerate()
    {
        let slot = u32::try_from(slot).unwrap();
        let x = fixture::EXTENT[0] / 2 + slot % 2;
        let y = fixture::EXTENT[1] / 2 + slot / 2;
        assert!((composed_channel(&mixed_image, x, y, 3) - transmission).abs() < 0.002);
        assert!((composed_channel(&level_image, x, y, 3) - level[slot as usize].0).abs() < 0.002);
        assert!(
            (distance - expected).abs() < 0.003,
            "mixed stop depths: {mixed:?}"
        );
        if expected < 1.0 {
            assert_eq!(
                transmission.to_bits(),
                1.0_f32.to_bits(),
                "near stops: {mixed:?}"
            );
        }
    }
    assert!(
        (mixed[2].0 - level[2].0).abs() < 0.02,
        "the far pixel has the same view past its neighbours: {level:?} -> {mixed:?}"
    );
    eprintln!("Clear Ground at zero coverage, (transmittance, stop km): {level:?} -> {mixed:?}");
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
                struct OccluderOutput {
                    @builtin(frag_depth) depth: f32,
                    @location(0) colour: vec4<f32>,
                };
                @fragment fn fragment(v: Varying) -> OccluderOutput {
                    // A known scene colour lets the tests recover both components of
                    // blended cloud transport independently of the sky behind the layer.
                    let colour = vec4(0.125, 0.25, 0.5, 1.0);
                    let left = 8u + 8u * v.mover;
                    let x = u32(v.position.x);
                    var distance = select(920.0, 340.0, x >= left && x < left + 8u);
                    if v.mover >= 2u {
                        let slot = x % 2u + 2u * (u32(v.position.y) % 2u);
                        let mixed = array(420.0, 700.0, 2000.0, 420.0);
                        distance = select(2000.0, mixed[slot], v.mover == 3u);
                    }
                    if v.mover >= 16u {
                        if x < v.mover { return OccluderOutput(0.0, colour); }
                        distance = 3.0;
                    }
                    let z = (1.0 / distance - 1.0 / 4000.0) / (1.0 / 0.1 - 1.0 / 4000.0);
                    return OccluderOutput(z, colour);
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
                targets: &[0, 1, 2].map(|i| {
                    Some(wgpu::ColorTargetState {
                        format: surface.color_formats[i],
                        blend: None,
                        write_mask: if i == 0 {
                            wgpu::ColorWrites::ALL
                        } else {
                            wgpu::ColorWrites::empty()
                        },
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
