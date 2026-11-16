use std::{fmt::Write as _, sync::mpsc, time::Duration};
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
            texel: 1600.0,
            slices: 16,
            maximum_age: 0.5,
        },
    ];
    let mut sun = SunPath::new(
        site,
        weather,
        SunProfile {
            size: 256,
            levels,
            ground_size: 1024,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
        None,
    )
    .expect("Sun product");
    sun.rebuild(1, 1);
    let mut record = String::new();
    let camera = [37.25, -89.75];
    for elevation in [60.0_f64, 20.0, 2.0] {
        let tick = state.tick() + 1;
        state.step(tick, 1.0 / 60.0).expect("weather tick");
        let time = EffectTime {
            epoch: 1,
            timeline: 1,
            tick,
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
            alpha: 1.0,
            capture: false,
        };
        let angle = elevation.to_radians();
        let direction = [angle.cos(), angle.sin(), 0.0];
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
        let mapping = buffer(device, queue, sun.mapping());
        let mapping: Vec<_> = mapping
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        assert!((f64::from(mapping[9]) - direction[1]).abs() < 1e-6);
        let mut extrema = Vec::new();
        for (level, product) in sun.products().iter().enumerate() {
            let pixels = texture(device, queue, product);
            let mut maximum = 0.0_f64;
            let mut squared = 0.0;
            let mut count = 0;
            let mut occupied = 0;
            let width = levels[level].texel * 256.0;
            let origin: [f64; 2] = std::array::from_fn(|a| {
                ((camera[a] - state.travel()[a]) / (width / 4.0)).floor() * (width / 4.0)
                    - width / 2.0
            });
            for slice in [levels[level].slices / 2, levels[level].slices - 1] {
                let height = 380.0 - f64::from(slice) * 280.0 / f64::from(levels[level].slices - 1);
                for y in [32_u32, 64, 128, 192, 224] {
                    for x in [32_u32, 64, 128, 192, 224] {
                        let base = [
                            origin[0] + (f64::from(x) + 0.5) * levels[level].texel,
                            origin[1] + (f64::from(y) + 0.5) * levels[level].texel,
                        ];
                        let point = [
                            base[0]
                                + state.travel()[0]
                                + direction[0] * (height - 100.0) / direction[1],
                            height,
                            base[1] + state.travel()[1],
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
                        let at = (((slice as usize / 4 * 256 + y as usize) * 256 + x as usize) * 4)
                            + slice as usize % 4;
                        let actual = f64::from(pixels[at]);
                        let error = ((-actual).exp() - (-reference).exp()).abs();
                        maximum = maximum.max(error);
                        squared += error * error;
                        count += 1;
                        occupied += usize::from(reference > 0.01);
                    }
                }
            }
            extrema.push((maximum, (squared / f64::from(count)).sqrt(), occupied));
        }
        let outer = texture(device, queue, &sun.products()[2]);
        let last_layer = outer.len() - 256 * 256 * 4;
        let mean = (0..256 * 256)
            .map(|i| (-f64::from(outer[last_layer + i * 4 + 3])).exp())
            .sum::<f64>()
            / 65536.0;
        assert!(
            ((-f64::from(mapping[12])).exp() - mean).abs() < 2e-6,
            "mean of transmittance, not mean depth"
        );
        writeln!(record,"elevation={elevation}; per-level (maximum T error, RMS T error, occupied samples)={extrema:?}; far_mean={mean}; portable={portable}").expect("record");
    }
    std::fs::create_dir_all("../../tmp/clouds-sun").expect("measurement directory");
    std::fs::write("../../tmp/clouds-sun/measurement.txt", record).expect("measurement record");
    continuity::run(device, queue, site, weather, state);
}

fn buffer(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Buffer) -> Vec<u8> {
    let target = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Sun test buffer readback"),
        size: source.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(source, 0, &target, 0, source.size());
    queue.submit([encoder.finish()]);
    collect(device, &target)
}

fn texture(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Texture) -> Vec<f32> {
    let row = (source.width() * 8).div_ceil(256) * 256;
    let target = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Sun test texture readback"),
        size: u64::from(row)
            * u64::from(source.height())
            * u64::from(source.depth_or_array_layers()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        source.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &target,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(source.height()),
            },
        },
        source.size(),
    );
    queue.submit([encoder.finish()]);
    let bytes = collect(device, &target);
    bytes
        .chunks_exact(row as usize)
        .flat_map(|row| {
            row[..source.width() as usize * 8]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| half(u16::from_le_bytes(*b)))
        })
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

fn collect(device: &wgpu::Device, source: &wgpu::Buffer) -> Vec<u8> {
    let (send, recv) = mpsc::channel();
    source
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).expect("readback receiver");
        });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("Metal completion");
    recv.recv().expect("callback").expect("map");
    let bytes = source.slice(..).get_mapped_range().expect("mapped output");
    let result = bytes.to_vec();
    drop(bytes);
    source.unmap();
    result
}
