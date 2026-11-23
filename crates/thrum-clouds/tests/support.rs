use std::{io::Write as _, time::Duration};
use thrum_cloud_model::{Field, Parameters, Profile, State, Type};
use thrum_clouds::{WeatherMap, WeatherProfile};
use thrum_render::{contribution::EffectTime, runtime::DeviceSite};
mod sun;

// A broken-sky window with fractional camera coordinates in both axes.
const CAMERA: [f64; 2] = [-602.75, -729.75];

fn weather_profile() -> WeatherProfile {
    WeatherProfile {
        size: 96,
        texels: vec![20.0, 80.0, 320.0, 1600.0],
        maximum_age: 0.5,
    }
}

fn data() -> (Profile, Vec<Type>) {
    // Dimensioned analytical fixture, rather than any game's weather type.
    let row = Type {
        base: 100.0,
        depth: [100.0, 200.0],
        liquid_water: 0.0001,
        droplet_radius: 0.00001,
        deck: 0.3,
        profile: [0.2, 0.3],
    };
    (
        Profile {
            pitch: 1000.0,
            turret_pitch: 300.0,
            turret_least: 0.18,
            turret_fade: 0.12,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.4,
        },
        vec![
            row,
            Type {
                base: 120.0,
                depth: [150.0, 260.0],
                deck: 0.0,
                ..row
            },
        ],
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test compares the hash's deliberate low-word integer addressing and bounded floor coordinates"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one conformance fixture checks weather and Sun-path products through replacement"
)]
pub(super) fn conformance(test: &str, portable: bool) {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        test,
        Some(wgpu::Backend::Metal),
        profile(portable),
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let site = DeviceSite::untracked(device.clone());
        let (profile, types) = data();
        let field = Field::new(7, profile, types.clone()).expect("fixture");
        let mut map = WeatherMap::new(&site, 7, profile, types, weather_profile()).expect("map");
        assert!(map.profile().texels[3] > profile.turret_pitch * 2.0);
        map.rebuild(1, 1);
        let mut state = State::new(
            Parameters {
                coverage: 0.7,
                kind: 0.4,
                wind: [40.0, -13.0],
                time_scale: 1.0,
            },
            5.0,
            0,
        )
        .expect("state");
        state.step(1, 86400.0).expect("day of travel");
        let mut largest = [0.0_f64; 6];
        let mut first_counts = [0; 2];
        let saved = state.clone();
        let size = map.profile().size as usize;
        for index in 0..12u64 {
            if index == 5 {
                state
                    .set_target(Parameters {
                        time_scale: 60.0,
                        ..state.parameters()
                    })
                    .expect("time lapse");
            }
            if index == 10 {
                state = saved.clone();
                map.rebuild(2, 1);
            }
            if index > 0 {
                state
                    .step(
                        state.tick() + 1,
                        if index == 5 || index == 9 {
                            5.0
                        } else {
                            1.0 / 60.0
                        },
                    )
                    .expect("step");
            }
            let tick = state.tick();
            let time = EffectTime {
                epoch: 1,
                timeline: 1,
                tick,
                tick_duration: Duration::from_secs_f64(1.0 / 60.0),
                alpha: 0.5,
                display_phase: 0.5,
                capture: false,
            };
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            map.upload(&queue, time, &state, CAMERA).expect("upload");
            let origins = map.origins().to_vec();
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                map.encode(&mut pass);
            }
            queue.submit([encoder.finish()]);
            map.submitted();
            let outputs = read(&device, &queue, map.textures());
            let counts = coarsest_centres(&outputs[0], &outputs[1], size);
            if index == 0 {
                first_counts = counts;
            }
            for (level, origin) in origins.iter().enumerate() {
                for y in 0..size {
                    let stamp = map.row_stamps()[level][y];
                    assert!(
                        state.time() - stamp.time <= 0.5 + 1e-8,
                        "weather-time row age"
                    );
                    for x in 0..size {
                        let air = std::array::from_fn(|a| {
                            origin[a] + ([x, y][a] as f64 + 0.5) * map.profile().texels[level]
                        });
                        let column = field.column(air, stamp.time, stamp.parameters);
                        let at = ((level * size + y) * size + x) * 4;
                        let words = |output: usize, j: usize| outputs[output][at + j];
                        let values = [
                            column.base,
                            column.top,
                            column.coverage,
                            column.kind,
                            column.age,
                            column.optical_depth,
                        ];
                        let actual = [
                            f32::from_bits(words(0, 0)),
                            f32::from_bits(words(0, 1)),
                            f32::from_bits(words(0, 2)),
                            f32::from_bits(words(0, 3)),
                            f32::from_bits(words(1, 0)),
                            f32::from_bits(words(1, 1)),
                        ];
                        for j in 0..6 {
                            largest[j] = largest[j].max((f64::from(actual[j]) - values[j]).abs());
                        }
                        if actual[4] > 0.0 && column.age > 0.0 {
                            assert!(
                                (f64::from(actual[4]) - column.age).abs() < 1e-5,
                                "nonzero cell age"
                            );
                        }
                        let indices = air.map(|p| (p / profile.pitch).floor() as i64);
                        assert_eq!(words(2, 0), indices[0] as u32);
                        assert_eq!(words(2, 1), indices[1] as u32);
                        let h = hash(indices[0] as u32, indices[1] as u32, 0, 1, 7);
                        let generation = (stamp.time / profile.period
                            - f64::from(h) / 4_294_967_296.0)
                            .floor() as i64;
                        assert_eq!(
                            words(2, 2),
                            generation as u32,
                            "generation integer identity"
                        );
                        for di in -1..=1 {
                            for dj in -1..=1 {
                                let h = hash(
                                    (indices[0] + di) as u32,
                                    (indices[1] + dj) as u32,
                                    0,
                                    1,
                                    7,
                                );
                                let reference = (stamp.time / profile.period
                                    - f64::from(h) / 4_294_967_296.0)
                                    .floor() as i64;
                                let bit = ((di + 1) * 3 + dj + 1) as u32;
                                let actual = (stamp.time / profile.period).floor() as i64
                                    - i64::from((words(2, 3) >> bit) & 1);
                                assert_eq!(actual, reference, "neighbour generation identity");
                                assert_eq!(
                                    actual - 1,
                                    reference - 1,
                                    "previous generation identity"
                                );
                            }
                        }
                    }
                }
            }
            let mut repeated =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            map.upload(&queue, time, &state, CAMERA).expect("repeat");
            assert!(!map.runs(), "same tick must leave all texels alone");
            {
                let mut pass = repeated.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                map.encode(&mut pass);
            }
            queue.submit([repeated.finish()]);
            map.submitted();
            assert_eq!(outputs, read(&device, &queue, map.textures()));
        }
        replacement(
            &gpu.fresh_adapter(),
            &device,
            &queue,
            &mut map,
            &state,
            portable,
        );
        writeln!(
            std::io::stderr().lock(),
            "adapter={:?}; portable={portable}; coarsest clear/cloudy centres={first_counts:?}; weather maximum errors [base,top,coverage,type,age,tau]={largest:?}",
            gpu.adapter.get_info()
        )
        .expect("weather conformance record");
        assert!(
            largest[0] <= 0.000_01
                && largest[1] <= 0.0016
                && largest[2] <= 0.000_009
                && largest[3] <= 0.000_01
                && largest[4] <= 0.21
                && largest[5] <= 3.5e-6,
            "measured weather conformance: {largest:?}"
        );
        sun::run(
            &device, &queue, &site, &field, &mut map, &mut state, portable,
        );
    });
}

fn coarsest_centres(columns: &[u32], optical: &[u32], size: usize) -> [usize; 2] {
    let last = columns.len() - size * size * 4;
    let mut counts = [0; 2];
    let mut occupied_turrets = 0;
    for (column, path) in columns[last..]
        .as_chunks::<4>()
        .0
        .iter()
        .zip(optical[last..].as_chunks::<4>().0)
    {
        counts[usize::from(f32::from_bits(column[2]) > 0.0)] += 1;
        occupied_turrets += usize::from(f32::from_bits(path[0]) > 0.0);
    }
    assert!(
        counts.into_iter().all(|n| n > 0) && occupied_turrets > 0,
        "{counts:?}, {occupied_turrets} turrets"
    );
    counts
}

fn filled(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    map: &mut WeatherMap,
    state: &State,
) -> Vec<Vec<u32>> {
    map.rebuild(9, 1);
    map.upload(
        queue,
        EffectTime {
            epoch: 1,
            timeline: 1,
            tick: state.tick(),
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
            alpha: 0.5,
            display_phase: 0.5,
            capture: false,
        },
        state,
        CAMERA,
    )
    .expect("replacement upload");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        map.encode(&mut pass);
    }
    queue.submit([encoder.finish()]);
    map.submitted();
    read(device, queue, map.textures())
}

fn replacement(
    gpu: &thrum_test_support::Gpu,
    old_device: &wgpu::Device,
    old_queue: &wgpu::Queue,
    map: &mut WeatherMap,
    state: &State,
    portable: bool,
) {
    let expected = filled(old_device, old_queue, map, state);
    // A fresh adapter on the admitted instance stands in for a replaced physical device.
    let (device, queue) = gpu.device(
        profile(portable),
        &thrum_test_support::Restriction::default(),
    );
    let (profile, types) = data();
    let mut fresh = WeatherMap::new(
        &DeviceSite::untracked(device.clone()),
        7,
        profile,
        types,
        map.profile().clone(),
    )
    .expect("replacement resources");
    assert_eq!(
        expected,
        filled(&device, &queue, &mut fresh, state),
        "fresh physical device preserves every column and integer identity"
    );
}

const fn profile(portable: bool) -> thrum_test_support::Profile {
    if portable {
        thrum_test_support::Profile::Portable
    } else {
        thrum_test_support::Profile::Native
    }
}

fn hash(i: u32, j: u32, g: u32, stream: u32, seed: u32) -> u32 {
    let mut x = i
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(j.wrapping_mul(0x85eb_ca77))
        .wrapping_add(g.wrapping_mul(0xc2b2_ae3d))
        .wrapping_add(
            stream
                .wrapping_add(131u32.wrapping_mul(seed))
                .wrapping_mul(0x27d4_eb2f),
        );
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297a_2d39);
    x ^ (x >> 15)
}

fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &[wgpu::Texture; 3],
) -> Vec<Vec<u32>> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let readbacks: Vec<_> = textures
        .iter()
        .map(|texture| {
            thrum_test_support::readback::Readback::texture(
                device,
                &mut encoder,
                texture,
                thrum_test_support::readback::Mip::level(0),
                ..,
            )
        })
        .collect();
    let submission = queue.submit([encoder.finish()]);
    readbacks
        .into_iter()
        .map(|readback| {
            readback
                .wait(device, submission.clone())
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect()
        })
        .collect()
}
