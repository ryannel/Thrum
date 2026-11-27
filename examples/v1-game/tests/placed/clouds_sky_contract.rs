//! Inspect the Ground layer where its result enters colour composition and reconstruction.

use super::{
    EXTENT, Occluder, composed_transmittance, fixture::Host, output_texture, stop_channel,
    submitted_linear,
};
use std::{
    sync::{Arc, atomic::AtomicU32},
    time::Duration,
};
use thrum_cloud_model::Parameters;
use thrum_render::{
    present::SurfaceProfile,
    runtime::{CaptureLimits, FrameConfig, SurfaceOutput},
    synthetic::{CaptureSource, DisplayOverlay},
};
use thrum_test_support::{
    Opened, Profile, Restriction,
    readback::{self, Mip},
};
use v1_game::{
    CloudSelection, GameHost, GameReconstruction, GameSetup, GroundWeather, SceneCamera,
    clouds::WeatherPreset,
};

fn open(label: &str) -> Option<Opened> {
    thrum_test_support::open(
        label,
        Some(wgpu::Backend::Metal),
        Profile::Portable,
        &Restriction::default(),
    )
}

fn prepare(gpu: &Opened, setup: GameSetup, extent: [u32; 2], view: &wgpu::Texture) -> Host {
    pollster::block_on(setup.prepare(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                format: view.format(),
                extent,
            }),
        },
    ))
    .expect("V1 cloud profile")
}

fn read(gpu: &Opened, texture: &wgpu::Texture) -> Vec<u8> {
    readback::texture(&gpu.device, &gpu.queue, texture, Mip::level(0), ..)
}

fn half(bytes: &[u8], pixel: usize, channel: usize) -> f32 {
    let offset = pixel * 8 + channel * 2;
    half::f16::from_le_bytes([bytes[offset], bytes[offset + 1]]).to_f32()
}

fn submit(host: &mut Host, output: &SurfaceOutput<'_>, milliseconds: u64) {
    assert!(
        host.drive(Duration::from_millis(milliseconds), Some(output))
            .expect("drive frame")
            .submitted
    );
}

fn retire(gpu: &Opened, host: &mut Host) {
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("Metal completion");
    while host.poll_completed().expect("frame report").is_some() {}
}

#[test]
fn sky_and_ground_allocate_their_declared_stops() {
    let Some(gpu) = open("Sky and Ground layer storage") else {
        return;
    };
    let drawable = output_texture(&gpu.device, EXTENT);
    let view = drawable.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent: EXTENT,
    };
    for (clouds, layers) in [
        (CloudSelection::Sky, 1),
        (CloudSelection::Ground(GroundWeather::Clear), 4),
    ] {
        let mut host = prepare(
            &gpu,
            GameSetup {
                clouds,
                cloud_weather: WeatherPreset::Fair.parameters(),
                ..GameSetup::default()
            },
            EXTENT,
            &drawable,
        );
        submit(&mut host, &output, 0);
        let first = host.cloud_sky_products().expect("layer");
        retire(&gpu, &mut host);
        submit(&mut host, &output, 17);
        let other = host.cloud_sky_products().expect("other history side");
        for texture in [
            &first.fresh_transport,
            &first.fresh_front,
            &first.transport,
            &first.front,
            &other.transport,
            &other.front,
        ] {
            assert_eq!(
                [texture.width(), texture.height()],
                EXTENT.map(|n| n.div_ceil(2))
            );
            assert_eq!(texture.depth_or_array_layers(), layers);
        }
        assert_ne!(
            first.transport, other.transport,
            "both history sides were read"
        );
        retire(&gpu, &mut host);
    }
}

#[test]
fn inside_upper_cloud_compose_interpolates_only_sky_stops() {
    let Some(gpu) = open("whole-ray interpolation beside foreground depth") else {
        return;
    };
    let drawable = output_texture(&gpu.device, EXTENT);
    let view = drawable.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent: EXTENT,
    };
    for edge in [17, 18] {
        let mut host = prepare(
            &gpu,
            GameSetup {
                clouds: CloudSelection::Ground(GroundWeather::Clear),
                cloud_weather: Parameters {
                    wind: [0.0; 2],
                    ..WeatherPreset::Overcast.parameters()
                },
                ..GameSetup::default()
            },
            EXTENT,
            &drawable,
        );
        host.set_game_contribution(Box::new(Occluder {
            position: Arc::new(AtomicU32::new(edge)),
            pipeline: None,
        }))
        .expect("foreground edge");
        host.set_scene_camera(
            SceneCamera::perspective([0.0, 1850.0, 0.0], [0.0, 1880.0, -100.0], 0.35, 0.1, 4000.0)
                .expect("inside upper cloud"),
        )
        .expect("camera");
        host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
            .expect("pre-tone capture");
        submit(&mut host, &output, 0);
        let sky = host.cloud_sky_products().expect("traced stops");
        let transport = read(&gpu, &sky.fresh_transport);
        let metadata = read(&gpu, &sky.fresh_front);
        let image = submitted_linear(&mut host, &gpu.device);
        for x in 16..edge {
            let y = 8;
            let base_x = (x - 1) / 2;
            let blend_x = if x % 2 == 0 { 0.75 } else { 0.25 };
            let mut total = 0.0;
            let mut weights = 0.0;
            let mut invalid = 0.0;
            for (ty, wy) in [(3u32, 0.25), (4, 0.75)] {
                for (tx, wx) in [(base_x, 1.0 - blend_x), (base_x + 1, blend_x)] {
                    let weight = wx * wy;
                    let channel = |bytes, slot: u32| {
                        stop_channel(bytes, EXTENT, 2 * tx + slot % 2, 2 * ty + slot / 2, 3)
                    };
                    // The fixture writes zero depth to the left of the edge. Derive
                    // tap eligibility from that geometry, independent of the shader.
                    if tx * 2 < edge {
                        assert!(channel(&metadata, 0).abs() < f32::EPSILON);
                        total += weight * channel(&transport, 0);
                        weights += weight;
                    } else {
                        assert!(channel(&metadata, 0) > 0.0);
                    }
                    invalid += weight * channel(&transport, 3);
                }
            }
            let expected = total / weights;
            let actual = composed_transmittance(&image, x, y);
            assert!(expected < 0.8, "the sky ray must contain cloud");
            assert!(
                (actual - expected).abs() < 0.002,
                "edge {edge}, {x},{y}: composed {actual}, expected {expected} ({weights} eligible weight), unmasked {invalid}"
            );
            if x + 1 == edge {
                assert!(
                    (invalid - expected).abs() > 0.1,
                    "fixture detects a surface tap"
                );
            }
        }
    }
}

const REACTIVE_EXTENT: [u32; 2] = [96, 64];
const REACTIVE_EYE: [f32; 3] = [0.0, 40.0, 0.0];
// R8Unorm coverage allows one quantisation step and a half-float readback.
const MASK_TOLERANCE: f32 = 0.005;
const CHANGE_STEP: f32 = 0.08;

struct ReactiveFrame {
    stops: Vec<u8>,
    fronts: Vec<u8>,
    raster: glam::Mat4,
    mask_and_depth: Vec<u8>,
    history: bool,
}

impl ReactiveFrame {
    fn pixels() -> impl Iterator<Item = (u32, u32)> {
        (2..REACTIVE_EXTENT[1] - 2).flat_map(|y| (2..REACTIVE_EXTENT[0] - 2).map(move |x| (x, y)))
    }

    fn transmittance(&self, (x, y): (u32, u32)) -> f32 {
        stop_channel(&self.stops, REACTIVE_EXTENT, x, y, 3)
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "test texel coordinates are bounded by the small render extent"
    )]
    fn without_wind_history(&self, prior: &Self, (x, y): (u32, u32)) -> Option<f32> {
        let extent = glam::Vec2::new(REACTIVE_EXTENT[0] as f32, REACTIVE_EXTENT[1] as f32);
        let uv = glam::Vec2::new((x / 2 * 2 + 1) as f32, (y / 2 * 2 + 1) as f32) / extent;
        let eye = glam::Vec3::from_array(REACTIVE_EYE);
        let far = self.raster.inverse()
            * glam::Vec4::new(uv.x.mul_add(2.0, -1.0), uv.y.mul_add(-2.0, 1.0), 0.0, 1.0);
        let ray = (far.truncate() / far.w - eye).normalize();
        let front = stop_channel(&self.fronts, REACTIVE_EXTENT, x, y, 0);
        let clip = prior.raster * (eye + ray * (1000.0 * front)).extend(1.0);
        let old_uv = clip.truncate().truncate() / clip.w * glam::Vec2::new(0.5, -0.5) + 0.5;
        let grid = old_uv * extent * 0.5 - 0.5;
        let base = grid.floor();
        let f = grid - base;
        if clip.w <= 0.0 || base.min_element() < 0.0 || (base + 1.0).cmpge(extent * 0.5).any() {
            return None;
        }
        let mut result = 0.0;
        for (dx, wx) in [(0, 1.0 - f.x), (1, f.x)] {
            for (dy, wy) in [(0, 1.0 - f.y), (1, f.y)] {
                let pixel = (2 * (base.x as u32 + dx), 2 * (base.y as u32 + dy));
                if !prior.sky(pixel) {
                    return None;
                }
                result += wx * wy * prior.transmittance(pixel);
            }
        }
        Some(result)
    }

    fn sky(&self, (x, y): (u32, u32)) -> bool {
        half(
            &self.mask_and_depth,
            (y * REACTIVE_EXTENT[0] + x) as usize,
            1,
        )
        .abs()
            < f32::EPSILON
    }

    fn mask(&self, (x, y): (u32, u32)) -> f32 {
        half(
            &self.mask_and_depth,
            (y * REACTIVE_EXTENT[0] + x) as usize,
            0,
        )
    }
}

fn reactive_frame(
    gpu: &Opened,
    host: &mut Host,
    output: &SurfaceOutput<'_>,
    time: u64,
) -> ReactiveFrame {
    submit(host, output, time);
    let stops = host.cloud_sky_products().expect("submitted layer");
    let witness = host
        .submitted_temporal_witness()
        .expect("submitted reconstruction inputs");
    let frame = ReactiveFrame {
        stops: read(gpu, &stops.transport),
        fronts: read(gpu, &stops.front),
        raster: glam::Mat4::from_cols_array_2d(&stops.raster_view_projection),
        mask_and_depth: read(gpu, witness),
        history: stops.history_valid,
    };
    retire(gpu, host);
    frame
}

#[test]
fn reactive_mask_distinguishes_held_reset_changing_and_drifting_clouds() {
    let Some(gpu) = open("Ground sky reactive coverage at the reconstructor") else {
        return;
    };
    let drawable = output_texture(&gpu.device, REACTIVE_EXTENT);
    let view = drawable.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent: REACTIVE_EXTENT,
    };
    let held_parameters = Parameters {
        wind: [0.0; 2],
        time_scale: 60.0,
        ..WeatherPreset::Fair.parameters()
    };
    let prepare = |weather| {
        let mut host = prepare(
            &gpu,
            GameSetup {
                clouds: CloudSelection::Ground(GroundWeather::Clear),
                cloud_weather: weather,
                reconstruction: GameReconstruction::Taau,
                display_overlay: DisplayOverlay::TemporalInputs,
                ..GameSetup::default()
            },
            REACTIVE_EXTENT,
            &drawable,
        );
        host.set_scene_camera(
            SceneCamera::perspective([0.0, 40.0, 0.0], [0.0, 1600.0, -3000.0], 0.9, 0.1, 4000.0)
                .expect("cloud edges in sky"),
        )
        .expect("camera");
        host
    };
    let mut held = prepare(held_parameters);
    let opaque_frames =
        [0, 17, 34, 51, 68].map(|time| reactive_frame(&gpu, &mut held, &output, time));
    let opaque = ReactiveFrame::pixels()
        .find(|&p @ (x, y)| {
            opaque_frames.iter().all(|frame| {
                [p, (x - 2, y), (x + 2, y), (x, y - 2), (x, y + 2)]
                    .into_iter()
                    .all(|q| frame.sky(q) && frame.transmittance(q) <= 0.005)
            })
        })
        .expect("one held opaque sky interior throughout the five frames");
    for frame in &opaque_frames[1..] {
        assert!(frame.history);
        assert!(
            frame.mask(opaque) <= MASK_TOLERANCE,
            "steady opaque sky {opaque:?}: {}",
            frame.mask(opaque)
        );
    }
    held.set_game_quality_knob("cloud refresh exponent", 1)
        .expect("reset only the layer");
    let reset = reactive_frame(&gpu, &mut held, &output, 85);
    assert!(!reset.history);
    let partial =
        |frame: &ReactiveFrame, p| frame.sky(p) && (0.25..0.75).contains(&frame.transmittance(p));
    let pixel = ReactiveFrame::pixels()
        .find(|&p| partial(&reset, p))
        .expect("partial cloud after layer reset");
    assert!((reset.mask(pixel) - (1.0 - reset.transmittance(pixel))).abs() <= MASK_TOLERANCE);

    held.edit_cloud_weather_for_test(Parameters {
        coverage: 0.85,
        ..held_parameters
    });
    let mut prior = reset;
    let mut changed = false;
    for time in (102..=306).step_by(17) {
        let frame = reactive_frame(&gpu, &mut held, &output, time);
        assert!(frame.history);
        let sample = ReactiveFrame::pixels()
            .filter(|&p| partial(&frame, p))
            .find_map(|p| {
                let opacity = 1.0 - frame.transmittance(p);
                let old = frame.without_wind_history(&prior, p)?;
                let difference = (frame.transmittance(p) - old).abs();
                (difference > CHANGE_STEP).then_some((p, opacity, difference))
            });
        if let Some((p, opacity, difference)) = sample {
            let actual = frame.mask(p);
            let expected = opacity * difference;
            assert!(
                actual > MASK_TOLERANCE
                    && actual < opacity
                    && (actual - expected).abs() <= MASK_TOLERANCE,
                "changed sky at {p:?}: mask {actual}, opacity {opacity}, expected {expected}"
            );
            changed = true;
        }
        if changed {
            break;
        }
        prior = frame;
    }
    assert!(
        changed,
        "a partial stationary cloud must write opacity times the transmittance change"
    );

    let mut wind = prepare(Parameters {
        wind: [40.0, 0.0],
        ..held_parameters
    });
    let mut prior = reactive_frame(&gpu, &mut wind, &output, 0);
    let mut followed_wind = false;
    for time in (17..=170).step_by(17) {
        let frame = reactive_frame(&gpu, &mut wind, &output, time);
        assert!(frame.history);
        followed_wind |= ReactiveFrame::pixels()
            .filter(|&p| partial(&frame, p) && prior.sky(p))
            .any(|p| {
                let current = frame.transmittance(p);
                let same_screen = (current - prior.transmittance(p)).abs();
                same_screen > CHANGE_STEP
                    && frame
                        .without_wind_history(&prior, p)
                        .is_some_and(|without_wind| {
                            let unadvected = (current - without_wind).abs();
                            unadvected > CHANGE_STEP
                                && frame.mask(p)
                                    < 0.5 * (1.0 - current) * same_screen.min(unadvected)
                        })
            });
        if followed_wind {
            break;
        }
        prior = frame;
    }
    assert!(
        followed_wind,
        "drifting edge must retain less than half the same-screen difference"
    );
}
