use std::{io::Write as _, time::Duration};
use thrum_cloud_model::{Density, Field, Sample, State};
use thrum_clouds::{SunLevel, SunPath, SunProfile, WeatherMap};
use thrum_render::{contribution::EffectTime, runtime::DeviceSite};
mod continuity;

#[expect(
    clippy::cast_possible_truncation,
    reason = "fixture directions narrow to the production GPU precision"
)]
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    site: &DeviceSite,
    field: &Field,
    weather: &mut WeatherMap,
    state: &mut State,
    portable: bool,
) {
    let levels = [
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
            texel: 960.0,
            slices: 16,
            maximum_age: 0.5,
        },
    ];
    let size = 16_u32;
    let mut sun = SunPath::new(
        site,
        weather,
        SunProfile {
            size,
            levels,
            ground_size: size * 2,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
        None,
    )
    .expect("Sun product");
    sun.rebuild(1, 1);
    let side = size as usize;
    let area = side * side;
    let mut outside = Vec::new();
    let camera = super::CAMERA;
    for (angle_index, elevation) in [60.0_f64, 20.0, 2.0].into_iter().enumerate() {
        let tick = state.tick() + 1;
        state.step(tick, 1.0 / 60.0).expect("weather tick");
        let time = EffectTime {
            epoch: 1,
            timeline: 1,
            tick,
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
            alpha: 1.0,
            display_phase: 1.0,
            capture: false,
        };
        let angle = elevation.to_radians();
        // Cover ray offsets along both horizontal axes.
        let direction = if angle_index == 0 {
            [0.0, angle.sin(), angle.cos()]
        } else {
            [angle.cos(), angle.sin(), 0.0]
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        weather
            .upload(queue, time, state, camera)
            .expect("weather upload");
        sun.upload(
            queue,
            weather,
            time,
            state,
            camera,
            direction.map(|v| v as f32),
            false,
        )
        .expect("Sun upload");
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            weather.encode(&mut pass);
        }
        for pass_index in 0..5 {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            sun.encode(pass_index, &mut pass);
        }
        queue.submit([encoder.finish()]);
        weather.submitted();
        sun.submitted();
        let mapping = thrum_test_support::readback::buffer(device, queue, sun.mapping(), ..);
        let mapping: Vec<_> = mapping
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        for (actual, expected) in mapping[8..11].iter().zip(direction) {
            assert!((f64::from(*actual) - expected).abs() < 1e-6);
        }
        let mut extrema = Vec::new();
        let mut far_mean = 0.0;
        for (level, product) in sun.products().iter().enumerate() {
            let pixels = texture(device, queue, product);
            let mut maximum = 0.0_f64;
            let mut squared = 0.0;
            let mut count = 0;
            let mut occupied = 0;
            let width = levels[level].texel * f64::from(size);
            let origin: [f64; 2] = std::array::from_fn(|a| {
                ((camera[a] - state.travel()[a]) / (width / 4.0)).floor() * (width / 4.0)
                    - width / 2.0
            });
            for slice in [levels[level].slices / 2, levels[level].slices - 1] {
                let height = 380.0 - f64::from(slice) * 280.0 / f64::from(levels[level].slices - 1);
                let axis = [size / 8, size / 4, size / 2, 3 * size / 4, 7 * size / 8];
                let mut locations: Vec<_> = axis
                    .into_iter()
                    .flat_map(|y| axis.into_iter().map(move |x| (x, y)))
                    .collect();
                let sample_offset = slice as usize / 4 * area * 4 + slice as usize % 4;
                let mut ranked: Vec<_> = (0..size * size).collect();
                ranked.sort_by(|&a, &b| {
                    pixels[sample_offset + b as usize * 4]
                        .total_cmp(&pixels[sample_offset + a as usize * 4])
                });
                locations.extend(ranked.iter().take(8).map(|&i| (i % size, i / size)));
                for (x, y) in locations {
                    let base = [
                        origin[0] + (f64::from(x) + 0.5) * levels[level].texel,
                        origin[1] + (f64::from(y) + 0.5) * levels[level].texel,
                    ];
                    let point = [
                        base[0]
                            + state.travel()[0]
                            + direction[0] * (height - 100.0) / direction[1],
                        height,
                        base[1]
                            + state.travel()[1]
                            + direction[2] * (height - 100.0) / direction[1],
                    ];
                    let reference = field.optical_depth(
                        &Sample {
                            tick,
                            origin: point,
                            direction,
                            length: (380.0 - height) / direction[1],
                            steps: 512,
                        },
                        state,
                        Density::Coarse,
                        |_| None,
                        0.0,
                    );
                    let at = sample_offset + (y as usize * side + x as usize) * 4;
                    let actual = f64::from(pixels[at]);
                    let error = ((-actual).exp() - (-reference).exp()).abs();
                    maximum = maximum.max(error);
                    squared += error * error;
                    count += 1;
                    occupied += usize::from(reference > 0.01);
                }
            }
            assert!(
                occupied > 0,
                "occupied reference samples on every level: {level}, {elevation}"
            );
            let rms = (squared / f64::from(count)).sqrt();
            // Preserve the original limits where possible. M3's resolved
            // turret edges and mip-filtered Sun path moved four maxima.
            let max_bound =
                [[0.029, 0.22, 0.14], [0.08, 0.39, 0.35], [0.66, 0.55, 0.97]][angle_index][level];
            let rms_bound = [
                [0.0075, 0.05, 0.032],
                [0.013, 0.076, 0.075],
                [0.095, 0.17, 0.26],
            ][angle_index][level];
            if maximum > max_bound || rms > rms_bound {
                outside.push((elevation, level, maximum, rms));
            }
            extrema.push((maximum, rms, occupied));
            if level == 2 {
                let last_layer = pixels.len() - area * 4;
                far_mean = (0..area)
                    .map(|i| (-f64::from(pixels[last_layer + i * 4 + 3])).exp())
                    .sum::<f64>()
                    / f64::from(size * size);
            }
        }
        assert!(
            ((-f64::from(mapping[12])).exp() - far_mean).abs() < 2e-6,
            "mean of transmittance, not mean depth"
        );
        writeln!(
            std::io::stderr().lock(),
            "elevation={elevation}; per-level (maximum T error, RMS T error, occupied samples)={extrema:?}; far_mean={far_mean}; portable={portable}"
        )
        .expect("Sun-path conformance record");
    }
    assert!(
        outside.is_empty(),
        "outside recorded Sun bounds: {outside:?}"
    );
    clear_sky(device, queue, site, sun.profile());
    continuity::run(device, queue, site, weather, state);
}

fn clear_sky(device: &wgpu::Device, queue: &wgpu::Queue, site: &DeviceSite, profile: &SunProfile) {
    let (model, types) = super::data();
    let mut weather =
        WeatherMap::new(site, 7, model, types, super::weather_profile()).expect("clear map");
    let mut sun = SunPath::new(site, &weather, profile.clone(), None).expect("clear Sun product");
    let mut state = State::new(
        thrum_cloud_model::Parameters {
            coverage: 0.0,
            kind: 0.4,
            wind: [40.0, -13.0],
            time_scale: 1.0,
        },
        5.0,
        0,
    )
    .expect("clear weather");
    state.step(1, 1.0 / 60.0).expect("clear tick");
    let time = EffectTime {
        epoch: 1,
        timeline: 1,
        tick: 1,
        tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        alpha: 1.0,
        display_phase: 1.0,
        capture: false,
    };
    weather.rebuild(50, 1);
    sun.rebuild(50, 1);
    weather
        .upload(queue, time, &state, super::CAMERA)
        .expect("clear upload");
    sun.upload(
        queue,
        &weather,
        time,
        &state,
        super::CAMERA,
        [0.8, 0.6, 0.0],
        false,
    )
    .expect("clear Sun upload");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        weather.encode(&mut pass);
    }
    for i in 0..5 {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        sun.encode(i, &mut pass);
    }
    queue.submit([encoder.finish()]);
    weather.submitted();
    sun.submitted();
    for product in sun.products().iter().chain(std::iter::once(sun.ground())) {
        assert!(
            texture(device, queue, product)
                .iter()
                .all(|&tau| tau.to_bits() == 0),
            "clear optical depth is exactly zero"
        );
    }
    let mapping = thrum_test_support::readback::buffer(device, queue, sun.mapping(), ..);
    let far = f32::from_le_bytes(mapping[48..52].try_into().expect("far word"));
    assert_eq!(
        (-far).exp().to_bits(),
        1.0_f32.to_bits(),
        "exactly unit clear far factor"
    );
}

fn texture(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Texture) -> Vec<f32> {
    half_pixels(&thrum_test_support::readback::texture(
        device,
        queue,
        source,
        thrum_test_support::readback::Mip::level(0),
        ..,
    ))
}

fn half_pixels(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| half(u16::from_le_bytes(*b)))
        .collect()
}

fn half(bits: u16) -> f32 {
    assert_eq!(bits >> 15, 0, "nonnegative optical depth");
    let exponent = (bits >> 10) & 31;
    let fraction = f32::from(bits & 1023);
    if exponent == 0 {
        fraction * 2.0_f32.powi(-24)
    } else {
        (1.0 + fraction / 1024.0) * 2.0_f32.powi(i32::from(exponent) - 15)
    }
}
