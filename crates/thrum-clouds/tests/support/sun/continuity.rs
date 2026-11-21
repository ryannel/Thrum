use super::texture;
use std::time::Duration;
use thrum_cloud_model::State;
use thrum_clouds::{SunLevel, SunPath, SunProfile, WeatherMap};
use thrum_render::{contribution::EffectTime, runtime::DeviceSite};

pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    site: &DeviceSite,
    weather: &WeatherMap,
    state: &State,
) {
    let mut sun = SunPath::new(
        site,
        weather,
        SunProfile {
            size: 32,
            ground_size: 64,
            levels: [
                SunLevel {
                    texel: 80.0,
                    slices: 48,
                    maximum_age: 0.5,
                },
                SunLevel {
                    texel: 320.0,
                    slices: 32,
                    maximum_age: 0.5,
                },
                SunLevel {
                    texel: 1600.0,
                    slices: 16,
                    maximum_age: 0.5,
                },
            ],
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
        None,
    )
    .expect("continuity product");
    let (floor_map, floor_pixels) = snapshot(device, queue, &mut sun, weather, state, 2.0, 1);
    // Receivers under the floor's near and middle windows, held fixed while the Sun sinks.
    let projection = f64::from(floor_map[8]) * f64::from(floor_map[4]) / f64::from(floor_map[9]);
    let points: Vec<_> = (0..17)
        .flat_map(|y| {
            (0..17).map(move |x| {
                [
                    f64::from(floor_map[20])
                        + f64::from(floor_map[22]) * (f64::from(x) + 0.5) / 17.0
                        - projection,
                    0.0,
                    f64::from(floor_map[21])
                        + f64::from(floor_map[22]) * (f64::from(y) + 0.5) / 17.0,
                ]
            })
        })
        .collect();
    let floor: Vec<_> = points
        .iter()
        .map(|&point| factor(&floor_map, &floor_pixels, point))
        .collect();
    let darkest = floor.iter().copied().fold(1.0_f64, f64::min);
    let brightest = floor.iter().copied().fold(0.0_f64, f64::max);
    assert!(
        brightest - darkest > 0.02,
        "broken-sky receivers, including a gap and cloud: {darkest}..{brightest}"
    );
    let mut values = Vec::new();
    for i in 0..=2048_u32 {
        let angle = 20.0 - 22.0 * f64::from(i) / 2048.0;
        let (mapping, pixels) = snapshot(
            device,
            queue,
            &mut sun,
            weather,
            state,
            angle,
            u64::from(i) + 2,
        );
        values.push(
            points
                .iter()
                .map(|&p| factor(&mapping, &pixels, p))
                .collect::<Vec<_>>(),
        );
    }
    let fine = largest(&values, 1);
    let coarse = largest(&values, 2);
    assert!(
        fine > 1e-5 && fine <= coarse * 0.7 + 0.002,
        "Sun continuity halves: fine={fine}, coarse={coarse}"
    );
    let (below_map, below_pixels) = snapshot(device, queue, &mut sun, weather, state, -2.0, 300);
    for (point, expected) in points.iter().zip(&floor) {
        assert!(
            (factor(&below_map, &below_pixels, *point) - expected).abs() < 1e-6,
            "no switch below floor"
        );
    }
    let mut border_records = Vec::new();
    for region in [0_usize, 16, 20] {
        let width = if region == 0 {
            f64::from(floor_map[2] - floor_map[0])
        } else {
            f64::from(floor_map[region + 2])
        };
        let left = f64::from(floor_map[region]);
        let bottom = f64::from(floor_map[region + 1]);
        // The cleared turrets sharpen the factor over a texel. Resolve the
        // same border interval with finer probes before comparing halvings.
        let rows: Vec<_> = (0..=1024)
            .map(|i| {
                let x = left + width * (-0.05 + 0.25 * f64::from(i) / 1024.0);
                (0..17)
                    .map(|j| {
                        factor(
                            &floor_map,
                            &floor_pixels,
                            [
                                x - projection,
                                0.0,
                                bottom + width * (f64::from(j) + 0.5) / 17.0,
                            ],
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let f = largest(&rows, 1);
        let c = largest(&rows, 2);
        assert!(
            f <= c * 0.7 + 0.0001,
            "field continuity region={region}, fine={f}, coarse={c}"
        );
        border_records.push((region, f, c));
    }
    let far = (-f64::from(floor_map[12])).exp();
    let width = f64::from(floor_map[2] - floor_map[0]);
    let edge_trial = |abrupt| {
        (0..=64)
            .map(|i| {
                let x = f64::from(floor_map[0]) + width * (-0.001 + 0.002 * f64::from(i) / 64.0);
                (0..17)
                    .map(|j| {
                        factor_with_edge(
                            &floor_map,
                            &floor_pixels,
                            [
                                x - projection,
                                0.0,
                                f64::from(floor_map[1]) + width * (f64::from(j) + 0.5) / 17.0,
                            ],
                            abrupt,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let gradual = edge_trial(false);
    let switched = edge_trial(true);
    assert!(largest(&gradual, 1) <= 0.7 * largest(&gradual, 2) + 0.0001);
    assert!(
        largest(&switched, 1) > 0.7 * largest(&switched, 2) + 0.0001,
        "an abrupt outside/far switch must fail at gap receivers"
    );
    assert!((factor(&floor_map, &floor_pixels, [-1e9, 0.0, -1e9]) - far).abs() < 1e-12);
    // A constant floor replacement cannot match both receivers. An abrupt edge
    // replacement would keep its jump at both step sizes instead of halving it.
    assert!((brightest - darkest) / 2.0 > 0.01);
    std::fs::write("../../tmp/clouds-sun/continuity.txt", format!("Sun largest changes: coarse={coarse}, fine={fine}; floor range={darkest}..{brightest}; border changes={border_records:?}; far={far}\n")).expect("continuity receipt");
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite fixture directions narrow to GPU precision"
)]
fn snapshot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sun: &mut SunPath,
    weather: &WeatherMap,
    state: &State,
    degrees: f64,
    generation: u64,
) -> ([f32; 24], Vec<f32>) {
    sun.rebuild(generation, 1);
    let angle = degrees.to_radians();
    sun.upload(
        queue,
        weather,
        EffectTime {
            epoch: 1,
            timeline: 1,
            tick: state.tick(),
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
            alpha: 1.0,
            display_phase: 1.0,
            capture: false,
        },
        state,
        [37.25, -89.75],
        [angle.cos() as f32, angle.sin() as f32, 0.0],
        false,
    )
    .expect("Sun snapshot");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for i in 0..5 {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        sun.encode(i, &mut pass);
    }
    queue.submit([encoder.finish()]);
    sun.submitted();
    let bytes = thrum_test_support::readback::buffer(device, queue, sun.mapping(), ..);
    let mapping = std::array::from_fn(|i| {
        f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("word"))
    });
    (mapping, texture(device, queue, sun.ground()))
}

fn largest(rows: &[Vec<f64>], stride: usize) -> f64 {
    rows.iter()
        .step_by(stride)
        .zip(rows.iter().skip(stride).step_by(stride))
        .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
        .fold(0.0, f64::max)
}

fn smooth(x: f64) -> f64 {
    let a = x.clamp(0.0, 1.0);
    a * a * (3.0 - 2.0 * a)
}

fn factor(m: &[f32; 24], pixels: &[f32], point: [f64; 3]) -> f64 {
    factor_with_edge(m, pixels, point, false)
}

fn factor_with_edge(m: &[f32; 24], pixels: &[f32], point: [f64; 3], abrupt: bool) -> f64 {
    if point[1] >= f64::from(m[4]) {
        return 1.0;
    }
    let distance = (f64::from(m[4]) - point[1]) / f64::from(m[9]);
    let p = [
        point[0] + f64::from(m[8]) * distance,
        point[2] + f64::from(m[10]) * distance,
    ];
    let width = f64::from(m[2] - m[0]);
    let outer = [f64::from(m[0]), f64::from(m[1]), width];
    let region = |at| [f64::from(m[at]), f64::from(m[at + 1]), f64::from(m[at + 2])];
    let edge = |r: [f64; 3]| {
        (p[0] - r[0])
            .min(p[1] - r[1])
            .min(r[0] + r[2] - p[0])
            .min(r[1] + r[2] - p[1])
    };
    let mix = |a: f64, b: f64, w: f64| a + (b - a) * w;
    let mut tau = atlas(pixels, p, outer, [0, 1]);
    tau = mix(
        tau,
        atlas(pixels, p, region(20), [1, 0]),
        smooth(edge(region(20)) / (f64::from(m[22]) * 0.15)),
    );
    tau = mix(
        tau,
        atlas(pixels, p, region(16), [0, 0]),
        smooth(edge(region(16)) / (f64::from(m[18]) * 0.15)),
    );
    mix(
        (-f64::from(m[12])).exp(),
        (-tau).exp(),
        if abrupt {
            f64::from(u8::from(edge(outer) > 0.0))
        } else {
            smooth(edge(outer) / f64::from(m[11]))
        },
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped texture indices"
)]
fn atlas(pixels: &[f32], point: [f64; 2], region: [f64; 3], tile: [usize; 2]) -> f64 {
    let q: [f64; 2] = std::array::from_fn(|i| {
        ((point[i] - region[i]) / region[2]).clamp(1.0 / 64.0, 1.0 - 1.0 / 64.0) * 32.0
            + f64::from(u32::try_from(tile[i]).expect("tile")) * 32.0
            - 0.5
    });
    let lo = q.map(|v| v.floor() as usize);
    let f = q.map(|v| v - v.floor());
    let mut result = 0.0;
    for y in 0..2 {
        for x in 0..2 {
            result += f64::from(pixels[((lo[1] + y) * 64 + lo[0] + x) * 4])
                * if x == 0 { 1.0 - f[0] } else { f[0] }
                * if y == 0 { 1.0 - f[1] } else { f[1] };
        }
    }
    result
}
