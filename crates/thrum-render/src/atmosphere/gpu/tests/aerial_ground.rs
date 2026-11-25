//! Ground-continuation referee: expected transport is a production column,
//! never a second implementation of atmospheric transport.
use super::super::AtmospherePrepared;
use crate::atmosphere::stage::Chart;
use crate::{atmosphere::AtmosphereFrame, synthetic::SyntheticView};
use std::{io::Write as _, path::Path};

const ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/atmosphere/gpu/tests/fixtures/aerial-ground"
);
const SIDE: usize = 32;
const CELLS: usize = SIDE * SIDE;

struct Fixture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    air: AtmospherePrepared,
}

impl Fixture {
    fn new((device, queue, info): (wgpu::Device, wgpu::Queue, wgpu::AdapterInfo)) -> Self {
        assert_eq!(info.backend, wgpu::Backend::Metal);
        writeln!(std::io::stdout(), "M9 device {info:?}").expect("report device");
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: crate::light::ENVIRONMENT_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let air = AtmospherePrepared::new(
            &crate::runtime::DeviceSite::untracked(device.clone()),
            &queue,
            1,
            AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 3.0, 1)
                .expect("valid M9 frame")
                .with_nightglow(),
            &SyntheticView::default(),
            &directional,
            (&super::sky_samples(&device), None),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for chart in [Chart::Transmittance, Chart::MultipleScattering] {
            let extent = chart.extent();
            let (compute, group) = air.chart_stages.get(chart);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(compute);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(extent[0].div_ceil(8), extent[1].div_ceil(8), 1);
        }
        queue.submit([encoder.finish()]);
        Self { device, queue, air }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "production uniforms use f32"
    )]
    fn volume(&self, height: f64, pitch: f64, fov: f64, sun: [f64; 2]) -> Volume {
        let t = (fov.to_radians() * 0.5).tan();
        let (s, c) = pitch.sin_cos();
        let (sy, cy) = sun[0].to_radians().sin_cos();
        let (sa, ca) = sun[1].to_radians().sin_cos();
        let mut packed = [0.0_f64; 32];
        // Column 16 is centred on the view's heading. The remaining columns
        // retain the production perspective projection and its row spacing.
        packed[2] = t;
        packed[4] = -s * t;
        packed[5] = c * t;
        packed[12..16].copy_from_slice(&[c, height + s, -t / 32.0, 1.0]);
        packed[16..20].copy_from_slice(&[0.0, height, 0.0, height]);
        packed[20..24].copy_from_slice(&[cy * ca, sy, cy * sa, 1.0]);
        packed[24..28].copy_from_slice(&[1.0, 1.0, 1.0, 1.0 / 1024.0]);
        self.queue.write_buffer(
            &self.air.uniform,
            0,
            packed.map(|v| (v as f32).to_le_bytes()).as_flattened(),
        );
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.air.aerial_pipeline);
            pass.set_bind_group(0, &self.air.aerial_group, &[]);
            pass.dispatch_workgroups(4, 4, 1);
        }
        self.queue.submit([encoder.finish()]);
        Volume {
            fields: self
                .air
                .aerial_textures
                .each_ref()
                .map(|texture| read(&self.device, &self.queue, texture)),
            height,
            pitch,
            t,
        }
    }
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<[u16; 4]> {
    thrum_test_support::readback::texture(
        device,
        queue,
        texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    )
    .as_chunks::<8>()
    .0
    .iter()
    .map(|pixel| std::array::from_fn(|c| u16::from_le_bytes([pixel[c * 2], pixel[c * 2 + 1]])))
    .collect()
}

struct Volume {
    // Production order: solar scatter (gain 1024), then RGB T and night.
    fields: [Vec<[u16; 4]>; 2],
    height: f64,
    pitch: f64,
    t: f64,
}

fn distance(layer: usize) -> f64 {
    32_000.0 * (layer as f64 / 32.0).powi(2)
}
fn value(bits: u16) -> f64 {
    f64::from(half::f16::from_bits(bits).to_f32())
}
fn quantum(v: f64) -> f64 {
    2.0_f64.powf(v.abs().log2().floor().max(-14.0) - 10.0)
}

impl Volume {
    fn ground(&self, x: usize, y: usize) -> f64 {
        let dx = (2.0 * (x as f64 + 0.5) / 32.0 - 1.0 - 1.0 / 32.0) * self.t;
        let dy = (1.0 - 2.0 * (y as f64 + 0.5) / 32.0) * self.t;
        let mu = (self.pitch.sin() + self.pitch.cos() * dy) / (1.0 + dx * dx + dy * dy).sqrt();
        let b = (6_371_000.0 + self.height) * mu;
        let c = self.height * (12_742_000.0 + self.height);
        if mu >= 0.0 || b * b < c {
            return f64::INFINITY;
        }
        c / (-b + (b * b - c).sqrt()).max(0.001)
    }

    // f64 lent blend: a point lies on column 16; row and metric depth blend.
    // The stopped substitute holds each contributing column at its ground hit.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped texture coordinates and depth layers"
    )]
    fn sample(&self, angle: f64, d: f64, stopped: bool) -> [[f64; 4]; 2] {
        let row = ((1.0 - (angle - self.pitch).tan() / self.t) * 16.0 - 0.5).clamp(0.0, 31.0);
        let low = row.floor() as usize;
        let mut out = [[0.0; 4]; 2];
        for (y, wy) in [(low, 1.0 - row.fract()), ((low + 1).min(31), row.fract())] {
            let d = if stopped {
                d.min(self.ground(16, y))
            } else {
                d
            };
            let layer = ((d / 32_000.0).sqrt() * 32.0).floor().min(31.0) as usize;
            let w =
                ((d - distance(layer)) / (distance(layer + 1) - distance(layer))).clamp(0.0, 1.0);
            for (z, wz) in [(layer, 1.0 - w), (layer + 1, w)] {
                for (field, result) in out.iter_mut().enumerate() {
                    for (c, result) in result.iter_mut().enumerate() {
                        *result +=
                            value(self.fields[field][z * CELLS + y * SIDE + 16][c]) * wy * wz;
                    }
                }
            }
        }
        out
    }

    fn bytes(&self) -> Vec<u8> {
        self.fields
            .iter()
            .flatten()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

#[test]
#[ignore = "one-time recording on the unchanged pass before M9; retained bytes are checked below"]
fn above_ground_bytes_record() {
    let Some(opened) = super::open("above_ground_bytes_record", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let fixture = Fixture::new(opened);
    pollster::block_on(async {
        let output = std::env::var_os("THRUM_AERIAL_GROUND_RECORD_DIR")
            .map_or_else(|| Path::new(ROOT).to_path_buf(), std::path::PathBuf::from);
        std::fs::create_dir_all(&output).expect("M9 fixture directory");
        for h in [3, 245] {
            for pitch in [0, -30] {
                let v = fixture.volume(
                    f64::from(h),
                    f64::from(pitch).to_radians(),
                    60.0,
                    [5.0, 20.0],
                );
                std::fs::write(output.join(format!("h{h}-p{pitch}.bin")), v.bytes())
                    .expect("save pre-change bytes");
            }
        }
    });
}

#[test]
fn above_ground_bytes_preserved() {
    let Some(opened) = super::open("above_ground_bytes_preserved", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let fixture = Fixture::new(opened);
    pollster::block_on(async {
        for h in [3, 245] {
            for pitch in [0, -30] {
                let v = fixture.volume(
                    f64::from(h),
                    f64::from(pitch).to_radians(),
                    60.0,
                    [5.0, 20.0],
                );
                let before = std::fs::read(Path::new(ROOT).join(format!("h{h}-p{pitch}.bin")))
                    .expect("retained pre-change bytes");
                let after = v.bytes();
                let field_bytes = CELLS * 33 * 8;
                let mut unchanged = 0;
                let mut descending = 0;
                for y in 0..SIDE {
                    for x in 0..SIDE {
                        let hit = v.ground(x, y);
                        let mut past = Vec::new();
                        for z in 0..33 {
                            let i = z * CELLS + y * SIDE + x;
                            if distance(z) <= hit {
                                for field in 0..2 {
                                    let b = field * field_bytes + i * 8;
                                    assert_eq!(
                                        &after[b..b + 8],
                                        &before[b..b + 8],
                                        "above ground h{h} pitch{pitch} ({x},{y},{z})"
                                    );
                                }
                                unchanged += 1;
                            } else {
                                past.push(z);
                                for field in &v.fields {
                                    assert!(field[i].iter().all(|v| value(*v).is_finite()));
                                }
                                for c in 0..3 {
                                    let t = value(v.fields[1][i][c]);
                                    assert!((0.0..=1.0).contains(&t));
                                    assert!(
                                        t <= value(v.fields[1][i - CELLS][c]),
                                        "T rises at ({x},{y},{z})"
                                    );
                                }
                            }
                        }
                        if past.len() >= 2 {
                            for c in 0..3 {
                                assert!(
                                    value(
                                        v.fields[1][past[past.len() - 1] * CELLS + y * SIDE + x][c]
                                    ) < value(v.fields[1][past[0] * CELLS + y * SIDE + x][c])
                                );
                            }
                            descending += 1;
                        }
                    }
                }
                writeln!(std::io::stdout(),
                    "M9.2 h{h} pitch{pitch}: {unchanged} unchanged texels, {descending} continued columns"
                ).expect("report preservation");
            }
        }
    });
}

fn cases() -> Vec<(f64, f64, f64, bool)> {
    let mut cases = Vec::new();
    for height in [0.0_f64, 3.0, 50.0, 245.0] {
        let horizon = -(height * (12_742_000.0 + height)).sqrt().atan2(6_371_000.0);
        for d in [3000.0, 10_000.0, 30_000.0] {
            for elevation in [0.05_f64, 0.2] {
                cases.push((height, d, horizon + elevation.to_radians(), d > 29_999.0));
            }
        }
    }
    for (height, distances) in [
        (3.0, &[3000.0][..]),
        (40.0, &[3000.0, 10_000.0][..]),
        (100.0, &[3000.0, 10_000.0, 30_000.0][..]),
    ] {
        for &d in distances {
            // A point on the ground at the declared camera-to-point distance.
            let sine =
                -(height * (12_742_000.0 + height) + d * d) / (2.0 * (6_371_000.0 + height) * d);
            cases.push((height, d, sine.asin(), (d - 10_000.0).abs() < 1.0));
        }
    }
    cases
}

#[test]
fn near_ground_reads_match_their_own_column() {
    let Some(opened) = super::open(
        "near_ground_reads_match_their_own_column",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let fixture = Fixture::new(opened);
    pollster::block_on(async {
        let mut failed = Vec::new();
        for fov in [60.0_f64, 90.0] {
            let t = (fov.to_radians() * 0.5).tan();
            for sun in [
                [45.0, 60.0],
                [10.0, 180.0],
                [5.0, 20.0],
                [2.0, 10.0],
                [0.5, 0.0],
            ] {
                let decides = fov < 61.0 && sun[0] >= 2.0;
                let mut worst = [[0.0_f64; 4]; 2];
                let mut stop_worst = 0.0_f64;
                for (h, d, angle, must_reject_stop) in cases() {
                    let reference = fixture.volume(h, angle - (t / 32.0).atan(), fov, sun);
                    let expected = reference.sample(angle, d, false);
                    let far = reference.sample(angle, 32_000.0, false);
                    let mut stop_case = 0.0_f64;
                    for pitch in 0..8 {
                        // Move the horizon through one complete row, without
                        // using sampler-quantized weights in the f64 referee.
                        let camera_pitch = (t / 32.0).atan() * (f64::from(pitch) / 4.0 - 1.0);
                        let volume = fixture.volume(h, camera_pitch, fov, sun);
                        let actual = volume.sample(angle, d, false);
                        let stopped = volume.sample(angle, d, true);
                        for field in 0..2 {
                            for c in 0..4 {
                                if field == 0 && c == 3 {
                                    continue;
                                }
                                let e = expected[field][c];
                                let allowance = if field == 0 {
                                    0.025 * e + 0.0025 * far[field][c] + quantum(e)
                                } else if c == 3 {
                                    0.10 * e
                                } else {
                                    0.01 * e + 0.001 + quantum(e)
                                };
                                let share = (actual[field][c] - e).abs() / allowance.max(1e-30);
                                worst[field][c] = worst[field][c].max(share);
                                if decides && share > 1.0 {
                                    failed.push(format!("h{h} d{d} angle{} sun{sun:?} pitch{pitch} field{field} c{c}: actual={:e} expected={e:e} allowance={allowance:e} share={share}", angle.to_degrees(), actual[field][c]));
                                }
                                if c < 3 {
                                    stop_case = stop_case
                                        .max((stopped[field][c] - e).abs() / allowance.max(1e-30));
                                }
                            }
                        }
                    }
                    if decides && must_reject_stop && stop_case <= 5.0 {
                        failed.push(format!(
                            "stop negative h{h} d{d} sun{sun:?}: worst share {stop_case} <= 5"
                        ));
                    }
                    stop_worst = stop_worst.max(stop_case);
                }
                writeln!(std::io::stdout(),
                    "M9.1 fov={fov} Sun={sun:?} decides={decides} worst [S,T+night]={worst:?}; stop={stop_worst}"
                ).expect("report reads");
            }
        }
        for row in failed.iter().take(32) {
            writeln!(std::io::stdout(), "FAIL {row}").expect("report failure");
        }
        assert!(
            failed.is_empty(),
            "M9.1 {} failures; allowance unchanged",
            failed.len()
        );
    });
}
