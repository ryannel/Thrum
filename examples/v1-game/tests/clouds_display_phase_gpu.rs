//! Submitted game frames interpolate shade without refreshing its products.
use crate::support;

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use thrum_render::runtime::{FrameConfig, FrameSample};
use v1_game::{CloudSelection, GameHost, GameScene, GameSetup, GameWorld};

type Host = thrum_render::runtime::FrameHost<GameWorld, GameScene>;

fn complete(host: &mut Host, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = host.poll_completed().expect("completion") {
            return sample;
        }
        assert!(Instant::now() < deadline);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal");
    }
}

#[test]
#[cfg(target_os = "macos")]
fn ramp_mapping_uses_predicted_ticks_and_repeated_ticks_write_no_products() {
    use thrum_cloud_model::{Parameters, State};
    use thrum_clouds::{SunPath, WeatherMap};
    use thrum_render::{contribution::EffectTime, runtime::DeviceSite};
    let Some(test_gpu) = thrum_test_support::gpu(
        "ramp_mapping_uses_predicted_ticks_and_repeated_ticks_write_no_products",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, _) = support::device(&test_gpu, "portable", false);
        let site = DeviceSite::untracked(device.clone());
        let config = v1_game::clouds::shade_config();
        let mut weather = WeatherMap::new(
            &site,
            config.seed,
            config.model,
            config.types,
            config.weather,
        )
        .expect("weather");
        let mut sun = SunPath::new(&site, &weather, config.sun, None).expect("Sun path");
        weather.rebuild(1, 1);
        sun.rebuild(1, 1);
        let p = Parameters {
            coverage: 0.45,
            kind: 1.0,
            wind: [10.0, 0.0],
            time_scale: 60.0,
        };
        let mut state = State::new(p, 10.0, 0).unwrap();
        state
            .set_target(Parameters {
                wind: [40.0, 0.0],
                ..p
            })
            .unwrap();
        let dt = Duration::from_secs_f64(1.0 / 60.0);
        let d = dt.as_secs_f64();
        let mut last_tick = None;
        let mut previous: Option<(f64, f64)> = None;
        let mut old_stamps = None;
        // First frame seeds, then before/past endpoint, >one tick past it,
        // next commit at the same target, equal target intervals and a cut.
        for (tick, offset, lead, cut, changed) in [
            (1, 0.0, 0.0, false, false),
            (1, 0.004, 0.0, false, false),
            (1, 0.008, 0.012, false, false),
            (1, 0.009, 0.05, false, false),
            (2, 0.0, 0.059 - d, false, false),
            (2, 0.001, 0.05, false, false),
            (2, 0.002, 0.05, false, false),
            (3, 0.0, 0.052 - d, false, true),
            (3, 0.001, 0.0, true, false),
            (3, d, 0.0, false, false),
        ] {
            if changed {
                state
                    .set_target(Parameters {
                        wind: [-40.0, 0.0],
                        ..p
                    })
                    .unwrap();
            }
            while state.tick() < tick {
                state.step(state.tick() + 1, d).unwrap();
            }
            let observation = f64::from(u32::try_from(tick).unwrap()) * d + offset;
            let target = observation + lead;
            let phase =
                ((target - f64::from(u32::try_from(tick).unwrap()) * d) / d).min(1.0 + 0.05 / d);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "effect phase is an f32 host input"
            )]
            let phase = phase as f32;
            let time = EffectTime {
                epoch: 1,
                timeline: 1,
                tick,
                tick_duration: dt,
                alpha: phase.min(1.0),
                display_phase: phase,
                capture: false,
            };
            weather
                .upload(&queue, time, &state, [1000.0, 1000.0])
                .unwrap();
            sun.upload(
                &queue,
                &weather,
                time,
                &state,
                [1000.0, 1000.0],
                [0.0, 1.0, 0.0],
                cut,
            )
            .unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            if last_tick == Some(tick) {
                assert!(!weather.runs());
                assert!(
                    (0..5).all(|i| !sun.runs(i)),
                    "no Sun/ground writes on a repeated tick"
                );
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                weather.encode(&mut pass);
                for i in 0..5 {
                    sun.encode(i, &mut pass);
                }
            }
            queue.submit([encoder.finish()]);
            weather.submitted();
            sun.submitted();
            let stamps: Vec<_> = sun
                .row_stamps()
                .iter()
                .flatten()
                .map(|s| (s.tick, s.time.to_bits()))
                .collect();
            if last_tick == Some(tick) {
                assert_eq!(old_stamps.as_ref().unwrap(), &stamps);
            }
            old_stamps = Some(stamps);
            let actual = f64::from(mapping(&device, &queue, sun.mapping())[16]) + 10240.0;
            // Independent per-tick wind arithmetic, not State's prediction or
            // the phase: 14,18,22,26,...; after target change 14,10,... .
            let display_tick = if cut {
                f64::from(u32::try_from(tick).unwrap())
            } else {
                target / d - 1.0
            };
            let mut expected = 0.0;
            for n in 1..=6 {
                let weight = (display_tick - f64::from(n - 1)).clamp(0.0, 1.0);
                let wind = if changed || tick >= 3 {
                    if n <= 2 {
                        10.0 + 4.0 * f64::from(n)
                    } else {
                        18.0 - 4.0 * f64::from(n - 2)
                    }
                } else {
                    10.0 + 4.0 * f64::from(n)
                };
                expected += weight * wind * (d * 60.0);
            }
            assert!(
                (actual - expected).abs() < 0.002,
                "tick {tick}: {actual} != {expected}"
            );
            if let Some((old_actual, old_expected)) = previous {
                assert!((actual - old_actual - (expected - old_expected)).abs() < 0.003);
            }
            println!(
                "tick={tick} observation={observation:.9} lead={lead:.9} target={target:.9} mapping={actual:.6} expected={expected:.6}"
            );
            previous = Some((actual, expected));
            last_tick = Some(tick);
        }
        // At 2.5 predicted ticks a one-increment implementation misses 8 m;
        // carrying it across the next commit introduces a 6 m catch-up step.
        assert_eq!((14.0 * 2.5) - (14.0 + 18.0 + 22.0 * 0.5), -8.0);
    });
}

fn mapping(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Buffer) -> Vec<f32> {
    let target = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("submitted shade mapping readback"),
        size: source.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(source, 0, &target, 0, source.size());
    queue.submit([encoder.finish()]);
    let (send, receive) = mpsc::channel();
    target.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        send.send(r).expect("receiver");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("Metal");
    receive.recv().expect("callback").expect("map");
    let bytes = target.slice(..).get_mapped_range().expect("readback");
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

#[test]
#[cfg(target_os = "macos")]
fn submitted_mapping_moves_at_display_time_without_product_writes() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_mapping_moves_at_display_time_without_product_writes",
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
        let mut host = GameSetup {
            clouds: CloudSelection::Shade,
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
        .expect("cloud game");
        host.drive(Duration::ZERO, Some(&surface)).expect("origin");
        complete(&mut host, &device);
        host.drive(Duration::from_nanos(16_666_667), Some(&surface))
            .expect("tick one");
        let first = complete(&mut host, &device);
        let (_, buffer) = host.cloud_ground_product().expect("GPU provider");
        let a = mapping(&device, &queue, &buffer);
        host.drive(Duration::from_millis(25), Some(&surface))
            .expect("same tick");
        let second = complete(&mut host, &device);
        let b = mapping(&device, &queue, &buffer);
        assert_eq!(first.tick, second.tick);
        let expected = 10.0 * (0.025 - 0.016_666_667);
        assert!(expected > 0.05, "distinct display times");
        // Near-window positions retain millimetre precision at this view.
        assert!((b[16] - a[16] - expected).abs() < 0.002);
        assert!((b[17] - a[17]).abs() < 0.002);
        assert!(
            !second
                .gpu_pass_ns
                .expect("GPU pass times")
                .passes()
                .iter()
                .any(|p| p.name.contains("cloud "))
        );
        let mut prior = b.clone();
        let mut prior_target = 0.025;
        for (observation, lead) in [(26, 8), (27, 40), (34, 0), (35, 8), (36, 40)] {
            host.set_presentation_lead(Duration::from_millis(lead));
            host.drive(Duration::from_millis(observation), Some(&surface))
                .expect("display phase frame");
            let sample = complete(&mut host, &device);
            let now = mapping(&device, &queue, &buffer);
            let target = Duration::from_millis(observation + lead).as_secs_f64();
            let expected = 10.0 * (target - prior_target);
            assert!((f64::from(now[16] - prior[16]) - expected).abs() < 0.002);
            if observation == 26 {
                assert_eq!(sample.presentation_alpha, 1.0);
                assert!(sample.display_phase > 1.0);
            }
            if observation == 27 {
                assert!(sample.display_phase > 2.0);
            }
            prior = now;
            prior_target = target;
        }
        host.set_presentation_lead(Duration::ZERO);
        host.set_scene_camera(
            v1_game::SceneCamera::perspective(
                [0.0, 0.0, 2.0],
                [0.0, 0.0, 0.0],
                65.0_f32.to_radians(),
                0.1,
                20.0,
            )
            .expect("checked cut camera"),
        )
        .expect("camera cut");
        host.drive(Duration::from_millis(37), Some(&surface))
            .expect("cut frame");
        complete(&mut host, &device);
        let c = mapping(&device, &queue, &buffer);
        let expected_cut = 10.0 * (2.0 / 60.0 - (prior_target - 1.0 / 60.0));
        assert!(
            (f64::from(c[16] - prior[16]) - expected_cut).abs() < 0.002,
            "cut uses committed travel"
        );
    });
}
