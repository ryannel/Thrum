//! Visible cloud contribution on the real game's submitted scene path.
use crate::support;
use std::time::{Duration, Instant};
use thrum_render::runtime::FrameConfig;
use v1_game::{CloudSelection, GameHost, GameScene, GameSetup, GameWorld};

type Host = thrum_render::runtime::FrameHost<GameWorld, GameScene>;

type Pixel = [f32; 4];
const EXTENT: [u32; 2] = [320, 180];

#[test]
#[cfg(target_os = "macos")]
fn absent_cloud_selection_keeps_the_compiled_graph_storage_and_pixels() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "absent_cloud_selection_keeps_the_compiled_graph_storage_and_pixels",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", false);
        let config = FrameConfig {
            render_extent: [160, 90],
            surface: None,
        };
        let mut original = GameSetup::default()
            .prepare(&device, &queue, &caps, config)
            .await
            .unwrap();
        let mut absent = GameSetup {
            clouds: CloudSelection::Off,
            ..GameSetup::default()
        }
        .prepare(&device, &queue, &caps, config)
        .await
        .unwrap();
        assert_eq!(original.storage_report(), absent.storage_report());
        let a = original.scene_graph().unwrap();
        let b = absent.scene_graph().unwrap();
        assert_eq!(
            a.pass_names().collect::<Vec<_>>(),
            b.pass_names().collect::<Vec<_>>()
        );
        assert!(b.pass_names().all(|name| !name.contains("cloud ")));
        assert_eq!(absent.cloud_resource_bytes(), Vec::<(String, u64)>::new());
        assert!(absent.cloud_sky_products().is_none());
        assert_eq!(
            original.capture_pixels().unwrap(),
            absent.capture_pixels().unwrap()
        );
    });
}

fn half(bytes: &[u8]) -> f32 {
    let n = u16::from_le_bytes([bytes[0], bytes[1]]);
    let sign = if n & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((n >> 10) & 31);
    let fraction = f32::from(n & 1023);
    if exponent == 0 {
        sign * fraction * 2f32.powi(-24)
    } else if exponent == 31 {
        sign * f32::INFINITY
    } else {
        sign * (1.0 + fraction / 1024.0) * 2f32.powi(exponent - 15)
    }
}

fn pixels(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<Pixel> {
    use std::sync::mpsc;
    assert_eq!(texture.format(), wgpu::TextureFormat::Rgba16Float);
    let stride = (texture.width() * 8).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cloud frame contract readback"),
        size: u64::from(stride) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |r| {
        send.send(r).expect("receiver");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(60)),
        })
        .expect("Metal readback");
    receive
        .recv_timeout(Duration::from_secs(60))
        .expect("callback")
        .expect("mapped");
    let mapped = buffer.get_mapped_range(..).expect("mapped range");
    let mut result = Vec::new();
    for row in mapped.chunks_exact(stride as usize) {
        for pixel in row[..texture.width() as usize * 8].as_chunks::<8>().0 {
            result.push(std::array::from_fn(|i| half(&pixel[i * 2..i * 2 + 2])));
        }
    }
    result
}

fn frame(
    host: &mut Host,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &thrum_render::runtime::SurfaceOutput<'_>,
    now: Duration,
) -> (thrum_render::runtime::FrameSample, Vec<Pixel>) {
    assert!(
        host.drive(now, Some(surface))
            .expect("display frame")
            .submitted
    );
    let witness = host
        .submitted_temporal_witness()
        .expect("declared submitted temporal reader")
        .clone();
    let values = pixels(device, queue, &witness);
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = host.poll_completed().expect("completion") {
            return (sample, values);
        }
        assert!(
            Instant::now() < deadline,
            "submitted cloud frame completion"
        );
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("completion");
    }
}

struct Projection {
    view: glam::Mat4,
    eye: glam::Vec3,
    raster: glam::Mat4,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "bounded test pixel coordinates and CPU model travel narrow to renderer input precision"
)]
fn assert_motion(
    projection: &Projection,
    actual: &[Pixel],
    clear: &[Pixel],
    front: &[Pixel],
    travel: [f64; 2],
    reset: bool,
) {
    let mut compared = 0;
    for y in 0..EXTENT[1] as usize {
        for x in 0..EXTENT[0] as usize {
            let i = y * EXTENT[0] as usize + x;
            assert_eq!(actual[i][1], clear[i][1], "cloud wrote depth at {x},{y}");
            let f = front[(y / 2) * (EXTENT[0] / 2) as usize + x / 2];
            if actual[i][1] != 0.0 {
                continue;
            }
            if f[1] <= 0.5 {
                assert_eq!(
                    &actual[i][2..],
                    &clear[i][2..],
                    "thin cloud replaced sky motion at {x},{y}"
                );
                continue;
            }
            if reset {
                assert_eq!(
                    &actual[i][2..],
                    &[65504.0; 2],
                    "reset cloud claims valid motion"
                );
                assert!(actual[i][0] >= 0.49, "reset layer is not reactive");
            } else {
                let uv = glam::Vec2::new(
                    (x as f32 + 0.5) / EXTENT[0] as f32,
                    (y as f32 + 0.5) / EXTENT[1] as f32,
                );
                let far = projection.raster.inverse()
                    * glam::Vec4::new(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
                let ray = (far.truncate() / far.w - projection.eye).normalize();
                let point = projection.eye + ray * (f[0] * 1000.0);
                let old = point - glam::Vec3::new(travel[0] as f32, 0.0, travel[1] as f32);
                let current = projection.view * point.extend(1.0);
                let previous = projection.view * old.extend(1.0);
                let expected = (previous.truncate() / previous.w - current.truncate() / current.w)
                    * glam::Vec3::new(EXTENT[0] as f32 * 0.5, -(EXTENT[1] as f32) * 0.5, 0.0);
                for axis in 0..2 {
                    let bound = 0.002 + 0.001 * expected[axis].abs();
                    assert!(
                        (actual[i][axis + 2] - expected[axis]).abs() <= bound,
                        "motion {x},{y} axis {axis}: got {} expected {}",
                        actual[i][axis + 2],
                        expected[axis]
                    );
                }
            }
            compared += 1;
        }
    }
    assert!(
        compared > 100,
        "probe must contain dominant clouds, got {compared}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn submitted_cloud_motion_tracks_model_display_travel_and_resets_whole_without_depth_writes() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_cloud_motion_tracks_model_display_travel_and_resets_whole_without_depth_writes",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", true);
        let (view, profile) = support::presentation(&device, EXTENT);
        let surface = thrum_render::runtime::SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        let config = FrameConfig {
            render_extent: EXTENT,
            surface: Some(profile),
        };
        let parameters = thrum_cloud_model::Parameters {
            wind: [40.0, 0.0],
            time_scale: 60.0,
            ..v1_game::clouds::initial_state().parameters()
        };
        let setup = GameSetup {
            reconstruction: thrum_render::synthetic::Reconstruction::Taau,
            display_overlay: thrum_render::synthetic::DisplayOverlay::TemporalInputs,
            ..GameSetup::default()
        };
        let mut clear = setup
            .prepare(&device, &queue, &caps, config)
            .await
            .expect("absent cloud game");
        let mut sky = GameSetup {
            clouds: CloudSelection::Sky,
            cloud_weather: parameters,
            ..setup
        }
        .prepare(&device, &queue, &caps, config)
        .await
        .expect("visible cloud game");
        sky.set_game_quality_knob("cloud refresh exponent", 1)
            .expect("start with sparse refresh before testing a profile change");
        let eye = [0.0, 40.0, 0.0];
        let target = [0.0, 1600.0, -3000.0];
        let fov = std::f32::consts::FRAC_PI_3;
        // The opaque camera ends before the cloud band. Cloud motion must
        // still replace sky motion where a dominant cloud is traced.
        let opaque_far = 700.0;
        let camera = v1_game::SceneCamera::perspective(eye, target, fov, 0.1, opaque_far)
            .expect("upward real camera");
        clear.set_scene_camera(camera).expect("clear camera");
        sky.set_scene_camera(camera).expect("cloud camera");
        let mut projection = Projection {
            view: glam::Mat4::from_cols_array_2d(
                &thrum_render::synthetic::SyntheticView::perspective(
                    eye,
                    target,
                    [0.0, 1.0, 0.0],
                    fov,
                    16.0 / 9.0,
                    0.1,
                    opaque_far,
                )
                .unwrap()
                .view_proj_columns(),
            ),
            eye: glam::Vec3::from_array(eye),
            raster: glam::Mat4::IDENTITY,
        };
        let mut state = thrum_cloud_model::State::new(parameters, 5.0, 0).unwrap();
        let dt = Duration::from_nanos(16_666_667);
        let mut previous = None;
        // Two display frames of tick one, then tick two, with jitter selected.
        for (now, tick, reset, quality, cut) in [
            (Duration::ZERO, 0, true, false, false),
            (dt, 1, false, false, false),
            (Duration::from_millis(25), 1, false, false, false),
            (dt * 2 + Duration::from_millis(4), 2, false, false, false),
            (Duration::from_millis(40), 2, true, true, false),
            (Duration::from_millis(45), 2, false, false, false),
            (Duration::from_millis(48), 2, true, false, true),
        ] {
            if quality {
                sky.set_game_quality_knob("cloud refresh exponent", 0)
                    .expect("profile change");
            }
            if cut {
                clear.set_scene_camera(camera).unwrap();
                sky.set_scene_camera(camera).unwrap();
            }
            while state.tick() < tick {
                state.step(state.tick() + 1, dt.as_secs_f64()).unwrap();
            }
            let phase = if tick == 0 {
                1.0
            } else {
                (now.as_secs_f64() - tick as f64 * dt.as_secs_f64()) / dt.as_secs_f64()
            };
            let travel = state.display_travel(phase, dt.as_secs_f64(), cut);
            let delta =
                previous.map_or([0.0; 2], |p: [f64; 2]| [travel[0] - p[0], travel[1] - p[1]]);
            let (clear_sample, clear_pixels) = frame(&mut clear, &device, &queue, &surface, now);
            let (sample, actual) = frame(&mut sky, &device, &queue, &surface, now);
            assert_eq!(sample.tick, tick);
            assert_eq!(sample.tick, clear_sample.tick);
            let products = sky.cloud_sky_products().expect("submitted layer handles");
            projection.raster = glam::Mat4::from_cols_array_2d(&products.raster_view_projection);
            assert_eq!(products.history_valid, !reset);
            let fronts = pixels(&device, &queue, &products.front);
            assert_motion(&projection, &actual, &clear_pixels, &fronts, delta, reset);
            if reset {
                assert_eq!(
                    pixels(&device, &queue, &products.transport),
                    pixels(&device, &queue, &products.fresh_transport),
                    "first reset frame must contain its whole fresh layer"
                );
            }
            if quality {
                assert_eq!(products.refresh_stride, 1);
            }
            previous = Some(travel);
        }
    });
}

#[test]
#[cfg(target_os = "macos")]
fn visible_layer_prepares_physical_light_and_submits_late_transport() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "visible_layer_prepares_physical_light_and_submits_late_transport",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", true);
        let (view, profile) = support::presentation(&device, [160, 90]);
        let surface = thrum_render::runtime::SurfaceOutput {
            view: &view,
            extent: [160, 90],
        };
        let mut game = GameSetup {
            clouds: CloudSelection::Sky,
            reconstruction: thrum_render::synthetic::Reconstruction::Taau,
            ..GameSetup::default()
        }
        .prepare(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: [160, 90],
                surface: Some(profile),
            },
        )
        .await
        .expect("physical cloud game");
        assert!(
            game.drive(Duration::ZERO, Some(&surface))
                .expect("first frame")
                .submitted
        );
        let deadline = Instant::now() + Duration::from_secs(60);
        let sample = loop {
            if let Some(sample) = game.poll_completed().expect("completion") {
                break sample;
            }
            assert!(
                Instant::now() < deadline,
                "submitted cloud frame completion"
            );
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("Metal completion");
        };
        let timings = sample.gpu_pass_ns.expect("named GPU timings");
        let names: Vec<&str> = timings.passes().iter().map(|p| p.name.as_ref()).collect();
        let positions = ["cloud hemisphere light", "cloud trace", "cloud compose"].map(|name| {
            names
                .iter()
                .position(|n| n.ends_with(name))
                .expect("visible cloud pass")
        });
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            game.capture_pixels().expect("submitted cloud pixels").len(),
            160 * 90 * 4
        );
    });
}
