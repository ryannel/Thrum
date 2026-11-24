use std::{io::Write as _, sync::Arc, time::Duration};
use thrum_cloud_model::{Field, Parameters, State};
use thrum_clouds::{Shade, ShadeConfig, SkyProfile, WeatherMap};
use thrum_render::{
    contribution::EffectTime,
    runtime::{DeviceSite, FrameConfig},
    synthetic::SyntheticView,
};
use thrum_test_support::{Opened, Profile, Restriction, readback};

use crate::{placed_fixture, scene_fixture};

// Coverage and type coordinates of a small analytical fixture. The game owns its
// own type table. Here the low-density rows keep vertical transmission representable.
const PRESETS: [[f64; 2]; 5] = [
    [0.0, 1.0],
    [0.5, 1.0],
    [0.712, 0.42],
    [1.0, 0.0],
    [0.0, 1.0],
];
const TEXEL: f64 = 10.0;
const STEP: Duration = Duration::from_millis(100);

fn config() -> ShadeConfig {
    let mut config = placed_fixture::weather_config();
    config.model.erosion = 0.08;
    config.types[0].deck = 1.0;
    config.types[0].base = 100.0;
    config.types[0].depth = [130.0, 160.0];
    config.types[1].depth = [155.0, 220.0];
    config
}

fn parameters([coverage, kind]: [f64; 2], wind: [f64; 2]) -> Parameters {
    Parameters {
        coverage,
        kind,
        wind,
        time_scale: 1.0,
    }
}

fn metal(label: &str) -> Option<Opened> {
    thrum_test_support::open(
        label,
        Some(wgpu::Backend::Metal),
        Profile::Portable,
        &Restriction::default(),
    )
}

pub(super) fn check_map() {
    let Some(gpu) = metal("weather presets and ramps") else {
        return;
    };
    let config = config();
    let field = Field::new(config.seed, config.model, config.types.clone()).expect("field");
    let site = DeviceSite::untracked(gpu.device.clone());
    let mut map = WeatherMap::new(
        &site,
        config.seed,
        config.model,
        config.types,
        config.weather,
    )
    .expect("weather map");
    map.rebuild(1, 1);
    let mut state = State::new(parameters(PRESETS[0], [40.0, 0.0]), 0.8, 0).expect("weather");
    let mut peak = [0.0_f64; 5];
    assert_eq!(compare_map(&gpu, &mut map, &field, &state, &mut peak), 0);
    for (index, &preset) in PRESETS[1..].iter().enumerate() {
        let wind = if index < 2 { [40.0, 0.0] } else { [0.0, 40.0] };
        let before = state.travel();
        state.set_target(parameters(preset, wind)).expect("target");
        assert_eq!(state.travel().map(f64::to_bits), before.map(f64::to_bits));
        for step in 1..=10 {
            let previous = state.travel();
            state
                .step(state.tick() + 1, STEP.as_secs_f64())
                .expect("step");
            assert_eq!(
                state.previous_travel().map(f64::to_bits),
                previous.map(f64::to_bits)
            );
            let integrated = std::array::from_fn(|axis| {
                previous[axis] + state.parameters().wind[axis] * STEP.as_secs_f64()
            });
            assert_eq!(
                state.travel().map(f64::to_bits),
                integrated.map(f64::to_bits)
            );
            if !matches!(step, 2 | 4 | 6 | 10) {
                continue;
            }
            let count = compare_map(&gpu, &mut map, &field, &state, &mut peak);
            if step == 10 {
                assert_eq!(state.parameters(), parameters(preset, wind));
                if index == 3 {
                    assert_eq!(count, 0);
                } else {
                    assert!(count > 0);
                }
            }
        }
    }
    writeln!(
        std::io::stderr().lock(),
        "Metal weather map maximum errors [base, top, coverage, type, tau]: {peak:?}"
    )
    .expect("record errors");
}

fn compare_map(
    gpu: &Opened,
    map: &mut WeatherMap,
    field: &Field,
    state: &State,
    peak: &mut [f64; 5],
) -> usize {
    let time = EffectTime {
        epoch: 1,
        timeline: 1,
        tick: state.tick(),
        tick_duration: STEP,
        alpha: 1.0,
        display_phase: 1.0,
        capture: false,
    };
    map.upload(&gpu.queue, time, state, [0.0; 2])
        .expect("upload");
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.encode(&mut encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default()));
    let copies = [&map.textures()[0], &map.textures()[1]].map(|texture| {
        readback::Readback::texture(
            &gpu.device,
            &mut encoder,
            texture,
            readback::Mip::level(0),
            ..,
        )
    });
    let submission = gpu.queue.submit([encoder.finish()]);
    map.submitted();
    let [column_bytes, optical_bytes] =
        copies.map(|copy| copy.wait(&gpu.device, submission.clone()));
    let column = column_bytes.as_chunks::<16>().0;
    let optical = optical_bytes.as_chunks::<16>().0;
    let side = map.profile().size;
    let mut occupied = 0;
    for (level, (&texel, origin)) in map.profile().texels.iter().zip(map.origins()).enumerate() {
        for y in 0..side {
            let stamp = map.row_stamps()[level][y as usize];
            for x in 0..side {
                let air = [
                    origin[0] + (f64::from(x) + 0.5) * texel,
                    origin[1] + (f64::from(y) + 0.5) * texel,
                ];
                let want = field.column(air, stamp.time, stamp.parameters);
                let at = (level * side as usize + y as usize) * side as usize + x as usize;
                let column_words = column[at].as_chunks::<4>().0;
                let actual = [
                    column_words[0],
                    column_words[1],
                    column_words[2],
                    column_words[3],
                    optical[at].as_chunks::<4>().0[1],
                ]
                .map(|word| f64::from(f32::from_le_bytes(word)));
                let expected = [
                    want.base,
                    want.top,
                    want.coverage,
                    want.kind,
                    want.optical_depth,
                ];
                let cloudy = actual[2] > 0.0;
                occupied += usize::from(cloudy);
                assert_eq!(cloudy, want.coverage > 0.0, "air={air:?} stamp={stamp:?}");
                // f32 shader arithmetic against f64 at the same, unfiltered texel centre.
                for (i, tolerance) in [0.001, 0.002, 0.000_02, 0.000_01, 0.000_03]
                    .into_iter()
                    .enumerate()
                {
                    let error = (actual[i] - expected[i]).abs();
                    peak[i] = peak[i].max(error);
                    assert!(
                        error <= tolerance,
                        "channel {i}: {} vs {} at {air:?}",
                        actual[i],
                        expected[i]
                    );
                }
            }
        }
    }
    occupied
}

pub(super) fn check_procedural_trace(label: &str, preset: [f64; 2]) {
    let Some(gpu) = metal("visible weather columns") else {
        return;
    };
    let config = config();
    let field = Field::new(config.seed, config.model, config.types.clone()).expect("field");
    pollster::block_on(async {
        let mut state = State::new(parameters(preset, [0.0; 2]), 0.8, 0).expect("weather");
        state.step(1, 450.0).expect("established weather");
        let mut core = None;
        let mut clear = None;
        for z in -100..100 {
            for x in -100..100 {
                let point = [x, z].map(|index| (f64::from(index) + 0.5) * TEXEL);
                let here = field.query([point[0], 0.0, point[1]], &state);
                if (0.6..5.0).contains(&here.optical_depth)
                    && core.is_none_or(|(_, _, coverage)| coverage < here.coverage)
                {
                    core = Some((point, here.optical_depth, here.coverage));
                }
                if clear.is_none()
                    && (-1..=1).all(|dx| {
                        (-1..=1).all(|dz| {
                            field
                                .query(
                                    [
                                        point[0] + f64::from(dx) * TEXEL,
                                        0.0,
                                        point[1] + f64::from(dz) * TEXEL,
                                    ],
                                    &state,
                                )
                                .optical_depth
                                == 0.0
                        })
                    })
                {
                    clear = Some(point);
                }
            }
        }
        let (point, expected, _) = core.expect("condensate at a map texel centre");
        let state = Arc::new(state);
        let actual = trace(&gpu, &config, &state, point).await;
        compare_column(label, actual, expected, 0.12);
        assert_eq!(
            trace(&gpu, &config, &state, clear.expect("clear neighbourhood"))
                .await
                .to_bits(),
            0.0_f64.to_bits(),
        );
    });
}

pub(super) fn check_placed_trace() {
    let Some(gpu) = metal("visible placed columns") else {
        return;
    };
    pollster::block_on(async {
        let fixture = placed_fixture::Fixture::new();
        let field = Field::new(
            fixture.config.seed,
            fixture.config.model,
            fixture.config.types.clone(),
        )
        .expect("placed field");
        // Keep the camera on a weather texel centre. Run both the resident
        // and column-only branches with the same physical placed footprint.
        for (range, resident) in [([25.0, 40.0], true), ([0.0, 10.0], false)] {
            let (mut host, input) = fixture
                .submit(&gpu, range, true, resident, [0.5, 0.866_025_4, 0.0])
                .await;
            assert!(fixture.shape.footprint().column([0.0; 2])[0] > 0.0);
            assert!(
                placed_fixture::ground(&gpu, &input)
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .any(|texel| texel[..2] != [0, 0])
            );
            let point = [-5.0, 5.0];
            host.source_mut().view = vertical_view(point);
            placed_fixture::frame(
                &gpu,
                &mut host,
                &placed_fixture::output(&gpu),
                Duration::from_secs(1),
            );
            let expected = field
                .query([point[0], 0.0, point[1]], &fixture.state)
                .optical_depth;
            compare_column("placed", centre_depth(&gpu, &input), expected, 0.12);
            let clear = [-75.0, 5.0];
            for du in [-TEXEL, 0.0, TEXEL] {
                for dv in [-TEXEL, 0.0, TEXEL] {
                    assert_eq!(
                        field
                            .query([clear[0] + du, 0.0, clear[1] + dv], &fixture.state)
                            .optical_depth
                            .to_bits(),
                        0.0_f64.to_bits()
                    );
                }
            }
            host.source_mut().view = vertical_view(clear);
            placed_fixture::frame(
                &gpu,
                &mut host,
                &placed_fixture::output(&gpu),
                Duration::from_secs(2),
            );
            assert_eq!(centre_depth(&gpu, &input).to_bits(), 0.0_f64.to_bits());
        }
    });
}

fn compare_column(label: &str, actual: f64, expected: f64, allowance: f64) {
    let fits = |depth: f64| depth.is_finite() && depth <= expected + allowance;
    writeln!(
        std::io::stderr().lock(),
        "{label}: GPU optical depth {actual:.5}, CPU {expected:.5}, allowance {allowance}"
    )
    .expect("record column depth");
    assert!(
        expected > 0.0 && actual > 0.0 && fits(actual),
        "{label}: {actual} versus {expected}"
    );
    assert!(
        !fits(actual * 2.0),
        "{label}: doubled density must fail the same ceiling"
    );
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "small fixture coordinates are exact in f32"
)]
fn vertical_view(point: [f64; 2]) -> SyntheticView {
    let [x, z] = point.map(|value| value as f32);
    SyntheticView::perspective(
        [x, 40.0, z],
        [x, 160.0, z],
        [0.0, 0.0, -1.0],
        0.65,
        1.0,
        0.1,
        200_000.0,
    )
    .expect("vertical ray")
}

async fn trace(gpu: &Opened, config: &ShadeConfig, state: &Arc<State>, point: [f64; 2]) -> f64 {
    let extent = [66, 66];
    let mut host = scene_fixture::prepare_view(
        &gpu.capabilities,
        &gpu.queue,
        Some(config.sun.ground_size),
        FrameConfig {
            render_extent: extent,
            surface: Some(thrum_render::present::SurfaceProfile {
                extent,
                format: wgpu::TextureFormat::Bgra8Unorm,
            }),
        },
        [0.5, 0.866_025_4, 0.0],
        state.tick(),
        &vertical_view(point),
    )
    .await;
    let shade = Shade::new(config.clone())
        .expect("shade")
        .with_sky(SkyProfile {
            extent,
            step: 5.0,
            light_samples: 0,
            refresh_stride: 1,
        })
        .expect("sky");
    let input = shade.input();
    input.publish(Arc::clone(state)).expect("publish");
    host.set_scene_contribution(Box::new(shade))
        .expect("attach");
    placed_fixture::frame(gpu, &mut host, &placed_fixture::output(gpu), Duration::ZERO);
    centre_depth(gpu, &input)
}

fn centre_depth(gpu: &Opened, input: &thrum_clouds::WeatherInput) -> f64 {
    let sky = input.sky_products().expect("submitted trace");
    let texture = &sky.fresh_transport;
    let bytes = readback::texture(
        &gpu.device,
        &gpu.queue,
        texture,
        readback::Mip::level(0),
        ..,
    );
    let side = texture.width() as usize;
    assert_eq!(side % 2, 1, "central ray is vertical");
    let at = ((side / 2) * side + side / 2) * 8 + 6;
    let word = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let exponent = i32::from((word >> 10) & 31);
    let fraction = f64::from(word & 1023) / 1024.0;
    let transmission = if exponent == 0 {
        fraction * 2.0_f64.powi(-14)
    } else {
        (1.0 + fraction) * 2.0_f64.powi(exponent - 15)
    };
    if word == 0x3c00 {
        0.0
    } else {
        -transmission.ln()
    }
}
