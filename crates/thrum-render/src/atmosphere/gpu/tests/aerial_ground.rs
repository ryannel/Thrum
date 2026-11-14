//! Ground-continuation referee: expected transport is a production column,
//! never a second implementation of atmospheric transport.
use super::super::{AtmospherePrepared, EXTENTS};
use crate::{atmosphere::AtmosphereFrame, synthetic::SyntheticView};
use std::{path::Path, sync::mpsc, time::Duration};

const ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/evidence/ATMOSPHERE/M9-volumes"
);
const SIDE: usize = 32;
const CELLS: usize = SIDE * SIDE;

struct Fixture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    air: AtmospherePrepared,
}

impl Fixture {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        eprintln!("M9 device {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: crate::synthetic::LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let air = AtmospherePrepared::new(
            &crate::runtime::DeviceSite::untracked(device.clone()),
            &queue,
            1,
            AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 3.0, 1)
                .unwrap()
                .with_nightglow(),
            &SyntheticView::default(),
            &directional,
            &super::sky_samples(&device),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for (index, extent) in EXTENTS.iter().take(2).enumerate() {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&air.compute[index]);
            pass.set_bind_group(0, &air.groups[index], &[]);
            pass.dispatch_workgroups(extent[0].div_ceil(8), extent[1].div_ceil(8), 1);
        }
        queue.submit([encoder.finish()]);
        Self { device, queue, air }
    }

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
    let size = 32 * 32 * 33 * 8;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("M9 raw production volume"),
        size,
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
                bytes_per_row: Some(256),
                rows_per_image: Some(32),
            },
        },
        texture.size(),
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
        send.send(result).unwrap()
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    receive.recv().unwrap().unwrap();
    buffer
        .slice(..)
        .get_mapped_range()
        .unwrap()
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
    pollster::block_on(async {
        let fixture = Fixture::new().await;
        std::fs::create_dir_all(ROOT).unwrap();
        for h in [3, 245] {
            for pitch in [0, -30] {
                let v = fixture.volume(
                    f64::from(h),
                    f64::from(pitch).to_radians(),
                    60.0,
                    [5.0, 20.0],
                );
                std::fs::write(
                    Path::new(ROOT).join(format!("h{h}-p{pitch}.bin")),
                    v.bytes(),
                )
                .unwrap();
            }
        }
    });
}

#[test]
fn above_ground_bytes_preserved() {
    pollster::block_on(async {
        let fixture = Fixture::new().await;
        for h in [3, 245] {
            for pitch in [0, -30] {
                let v = fixture.volume(
                    f64::from(h),
                    f64::from(pitch).to_radians(),
                    60.0,
                    [5.0, 20.0],
                );
                let before =
                    std::fs::read(Path::new(ROOT).join(format!("h{h}-p{pitch}.bin"))).unwrap();
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
                eprintln!(
                    "M9.2 h{h} pitch{pitch}: {unchanged} unchanged texels, {descending} continued columns"
                );
            }
        }
    });
}
