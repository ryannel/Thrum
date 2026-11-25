//! H2/H4 filtered-decode referee, independent f64 addressing of read-back texels.
use super::observer;
use crate::{
    atmosphere::{AtmosphereFrame, EnvironmentProducts},
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView},
};
use std::f64::consts::PI;
use wgpu::util::DeviceExt;

struct Field {
    width: usize,
    height: usize,
    values: Vec<[f64; 3]>,
    steps: Vec<[f64; 3]>,
}

impl Field {
    fn assert_known_contrast(&self) {
        let layers = self.values.len() / (self.width * self.height);
        for (index, first) in self.values.iter().enumerate() {
            let x = index % self.width;
            let y = index / self.width % self.height;
            let z = index / (self.width * self.height);
            for next in [
                (x + 1 < self.width).then_some(index + 1),
                (y + 1 < self.height).then_some(index + self.width),
                (z + 1 < layers).then_some(index + self.width * self.height),
            ]
            .into_iter()
            .flatten()
            {
                for (c, value) in first.iter().enumerate() {
                    let quantum = self.steps[index][c].max(self.steps[next][c]);
                    assert!(
                        (value - self.values[next][c]).abs() >= 32.0 * quantum,
                        "known field must differ by at least 32 stored steps: texel {index} neighbour {next} channel {c}"
                    );
                }
            }
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped nonnegative texture coordinates select bounded integer texels"
    )]
    fn sample(
        &self,
        uv: [f64; 2],
        low: usize,
        high: usize,
        depth_weight: f64,
    ) -> ([f64; 3], [f64; 3]) {
        let x = (uv[0] * self.width as f64 - 0.5).clamp(0.0, (self.width - 1) as f64);
        let y = (uv[1] * self.height as f64 - 0.5).clamp(0.0, (self.height - 1) as f64);
        let ix = x.floor() as usize;
        let iy = y.floor() as usize;
        let mut sum = [0.0; 3];
        let mut minimum = [f64::INFINITY; 3];
        let mut maximum = [f64::NEG_INFINITY; 3];
        let mut steps = [0.0_f64; 3];
        for (layer, wz) in [(low, 1.0 - depth_weight), (high, depth_weight)] {
            for (row, wy) in [
                (iy, 1.0 - y.fract()),
                ((iy + 1).min(self.height - 1), y.fract()),
            ] {
                for (column, wx) in [
                    (ix, 1.0 - x.fract()),
                    ((ix + 1).min(self.width - 1), x.fract()),
                ] {
                    let index = (layer * self.height + row) * self.width + column;
                    let texel = self.values[index];
                    for channel in 0..3 {
                        sum[channel] += texel[channel] * wx * wy * wz;
                        minimum[channel] = minimum[channel].min(texel[channel]);
                        maximum[channel] = maximum[channel].max(texel[channel]);
                        steps[channel] = steps[channel].max(self.steps[index][channel]);
                    }
                }
            }
        }
        let bound = std::array::from_fn(|i| {
            (maximum[i] - minimum[i]) / 32.0 + 1e-5 * maximum[i] + steps[i]
        });
        (sum, bound)
    }
}

// One representable RGBA16Float interval at the stored magnitude, then decoded.
pub(super) fn stored_step(value: f64, scale: f64) -> f64 {
    let stored = (value * scale).abs();
    let exponent = stored.log2().floor().max(-14.0);
    2.0_f64.powf(exponent - 10.0) / scale
}

fn sky_uv(direction: [f32; 4], azimuth: f64) -> [f64; 2] {
    let d = direction.map(f64::from);
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let y = d[1] / length;
    let horizon = -(2.0_f64 * (2.0 * 6_371_000.0 + 2.0)).sqrt() / (6_371_000.0 + 2.0);
    let tangent = horizon.asin();
    let angle = y.clamp(-1.0, 1.0).asin();
    let row = if y >= horizon {
        54.5 + 53.0
            * ((angle - tangent) / (PI / 2.0 - tangent))
                .clamp(0.0, 1.0)
                .sqrt()
    } else {
        53.5 - 53.0
            * ((tangent - angle) / (tangent + PI / 2.0))
                .clamp(0.0, 1.0)
                .sqrt()
    };
    let longitude = if d[0] * d[0] + d[2] * d[2] <= 1e-10 {
        0.0
    } else {
        let a = d[2].atan2(d[0]) - azimuth;
        a.sin().atan2(a.cos())
    };
    [longitude / (2.0 * PI) + 0.5, row / 108.0]
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded physical query directions round once for the GPU"
)]
fn direction(uv: [f64; 2], azimuth: f64) -> [f32; 4] {
    let horizon = -(2.0_f64 * (2.0 * 6_371_000.0 + 2.0)).sqrt() / (6_371_000.0 + 2.0);
    let tangent = horizon.asin();
    let angle = if uv[1] < 0.5 {
        let q = (53.5 - uv[1] * 108.0) / 53.0;
        tangent - (tangent + PI / 2.0) * q * q
    } else {
        let q = (uv[1] * 108.0 - 54.5) / 53.0;
        tangent + (PI / 2.0 - tangent) * q * q
    };
    let longitude = (uv[0] * 2.0 - 1.0) * PI + azimuth;
    [
        (angle.cos() * longitude.cos()) as f32,
        angle.sin() as f32,
        (angle.cos() * longitude.sin()) as f32,
        0.0,
    ]
}

fn within(actual: [f32; 4], expected: ([f64; 3], [f64; 3])) -> bool {
    (0..3).all(|i| (f64::from(actual[i]) - expected.0[i]).abs() <= expected.1[i])
}

fn variant(products: &EnvironmentProducts, old: &str, replacement: &str) -> EnvironmentProducts {
    assert!(
        products.shader_source.contains(old),
        "wrong-lookup mutation must hit the actual operation"
    );
    EnvironmentProducts {
        layout: products.layout.clone(),
        shader_source: products.shader_source.replace(old, replacement),
        groups: products.groups.clone(),
    }
}

pub(super) fn exposure_two(
    prepared: &PreparedSynthetic,
    queue: &wgpu::Queue,
) -> EnvironmentProducts {
    with_exposure(prepared, queue, 2.0)
}

pub(super) fn with_exposure(
    prepared: &PreparedSynthetic,
    queue: &wgpu::Queue,
    scale: f32,
) -> EnvironmentProducts {
    let surface = prepared.scene_surface(queue);
    let exposure = surface
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("H row fixed exposure scale"),
            contents: [scale, 0.0, 0.0, 0.0].map(f32::to_le_bytes).as_flattened(),
            usage: wgpu::BufferUsages::UNIFORM,
        });
    super::super::create(
        surface.environment.expect("atmosphere surface"),
        surface.device,
        surface.directional,
        surface.environment_light,
        &[exposure.clone(), exposure],
        false,
        None,
    )
}

pub(super) fn submit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &OffscreenTarget,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 140, target, None)
        .expect("production graph encodes");
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("production graph completes");
    prepared.commit_frame();
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded f64 physical query directions round once for the GPU"
)]
fn solar_grid_directions() -> Vec<[f32; 4]> {
    let horizon = -(6_371_000.0_f64 / (6_371_000.0 + 2.0)).acos().to_degrees();
    [
        (90.0, 0.0),
        (15.0, 0.0),
        (15.0, 90.0),
        (15.0, 180.0),
        (1.0, 0.0),
        (1.0, 90.0),
        (1.0, 180.0),
        (-1.0, 0.0),
        (-15.0, 0.0),
        (horizon - 0.005, 0.0),
        (horizon + 0.005, 0.0),
    ]
    .into_iter()
    .map(|(e, a): (f64, f64)| {
        let (sy, cy) = e.to_radians().sin_cos();
        let (sa, ca) = a.to_radians().sin_cos();
        [(cy * ca) as f32, sy as f32, (cy * sa) as f32, 0.0]
    })
    .collect()
}

#[test]
fn metal_h2_filtered_decode_rejects_each_wrong_lookup_on_every_known_field_query() {
    pollster::block_on(async {
        let Some(crate::test_device::Opened {
            gpu: test_gpu,
            device,
            queue,
            ..
        }) = crate::test_device::open(
            "metal_h2_filtered_decode_rejects_each_wrong_lookup_on_every_known_field_query",
            Some(wgpu::Backend::Metal),
            crate::capabilities::DeviceProfile::Portable,
            &crate::capabilities::DeviceRestriction {
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            },
        )
        else {
            return;
        };
        let adapter = test_gpu.adapter.clone();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let caps = DeviceCapabilities::capture(&adapter, &device, 140);
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = SyntheticView::perspective(
            [0.0, 2.0, 0.0],
            [0.0, 2.0, -100.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            32_000.0,
        )
        .unwrap();
        let initial = AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(initial),
            1.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
        let products = prepared
            .scene_surface(&queue)
            .environment_products()
            .unwrap();
        for degrees in [45.0_f32, 0.0, -3.0, -6.0, -9.0, -12.0] {
            let (y, x) = degrees.to_radians().sin_cos();
            let frame = AtmosphereFrame::new([x, y, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(frame),
                    1.0,
                )
                .unwrap();
            submit(&device, &queue, &mut prepared, &target);
            let chart = &prepared
                .scene_surface(&queue)
                .environment
                .unwrap()
                .textures
                .sky_view;
            let field = Field {
                width: 192,
                height: 108,
                steps: observer::texels(&device, &queue, chart)
                    .iter()
                    .map(|texel| std::array::from_fn(|i| stored_step(texel[i], 1.0) / 1024.0))
                    .collect(),
                values: observer::texels(&device, &queue, chart)
                    .iter()
                    .map(|texel| std::array::from_fn(|i| texel[i] / 1024.0))
                    .collect(),
            };
            let queries = solar_grid_directions();
            let result = observer::sample(
                &device,
                &queue,
                &products,
                "vec4(environment_sky_radiance(q.xyz),1.0)",
                &queries,
                0,
            );
            for (query, actual) in queries.iter().zip(result) {
                assert!(
                    within(actual, field.sample(sky_uv(*query, 0.0), 0, 0, 0.0)),
                    "H2 live Sun {degrees}, query {query:?}, actual {actual:?}, uv {:?}, expected {:?}",
                    sky_uv(*query, 0.0),
                    field.sample(sky_uv(*query, 0.0), 0, 0, 0.0)
                );
            }
        }
        known_sky_fields(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &products,
        );
        spectral_row(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &products,
        );
    });
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped physical depth selects a slice in 0 through 31"
)]
fn depth(query: [f32; 4]) -> (usize, usize, f64) {
    let distance = f64::from(query[2]).min(32_000.0);
    let low = ((distance / 32_000.0).sqrt() * 32.0).floor().min(31.0) as usize;
    let first = 32_000.0 * (low as f64 / 32.0).powi(2);
    let second = 32_000.0 * ((low + 1) as f64 / 32.0).powi(2);
    (
        low,
        low + 1,
        ((distance - first) / (second - first)).clamp(0.0, 1.0),
    )
}

// Every filtered, clamped and fixed query lands in this cell of the live volume.
const AERIAL_CELL: [u16; 2] = [9, 11];

fn aerial_expected(field: &Field, query: [f32; 4]) -> ([f64; 3], [f64; 3]) {
    let (low, high, weight) = depth(query);
    let uv =
        std::array::from_fn(|i| (f64::from(query[i]) * 32.0 - f64::from(AERIAL_CELL[i])) / 2.0);
    field.sample(uv, low, high, weight)
}

fn aerial_probe(device: &wgpu::Device, products: &EnvironmentProducts) -> observer::Probe<2> {
    observer::Probe::new(
        device,
        products,
        "let sample = environment_aerial(vec3(0.0, 2.0, -q.z), q.xy);",
        [
            "vec4(sample.transmission, 1.0)",
            "vec4(sample.scatter, 1.0)",
        ],
    )
}

const AERIAL_LAYERS: u32 = 33;

fn aerial_cell_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stored aerial cell readback"),
        size: wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: AERIAL_LAYERS * 2,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn stored_aerial_cell(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    producer: &[wgpu::Texture; 2],
    cell: &wgpu::Texture,
) -> Vec<u8> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    // Copy from the producer's textures directly so a wrong lent binding
    // cannot change both the shader result and the CPU reference.
    for (source, base_layer) in producer.iter().zip([0, AERIAL_LAYERS]) {
        assert_eq!(
            [
                source.width(),
                source.height(),
                source.depth_or_array_layers()
            ],
            [32, 32, AERIAL_LAYERS]
        );
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d {
                    x: u32::from(AERIAL_CELL[0]),
                    y: u32::from(AERIAL_CELL[1]),
                    z: 0,
                },
                ..source.as_image_copy()
            },
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: base_layer,
                },
                ..cell.as_image_copy()
            },
            wgpu::Extent3d {
                depth_or_array_layers: AERIAL_LAYERS,
                ..cell.size()
            },
        );
    }
    let readback = thrum_test_support::readback::Readback::texture(
        device,
        &mut encoder,
        cell,
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    let submission = queue.submit([encoder.finish()]);
    readback.wait(device, submission)
}

fn aerial_fields(stored: &[u8]) -> (Field, Field) {
    let (scatter_texels, transmittance_texels) = stored.split_at(stored.len() / 2);
    let mut transmission = Field {
        width: 2,
        height: 2,
        steps: Vec::new(),
        values: Vec::new(),
    };
    let mut scatter = Field {
        width: 2,
        height: 2,
        steps: Vec::new(),
        values: Vec::new(),
    };
    for (t, s) in
        observer::decode_texels(transmittance_texels).zip(observer::decode_texels(scatter_texels))
    {
        transmission.values.push([t[0], t[1], t[2]]);
        transmission
            .steps
            .push(std::array::from_fn(|i| stored_step(t[i], 1.0)));
        scatter
            .values
            .push(std::array::from_fn(|i| s[i] / 1024.0 + t[3] / 65_536.0));
        scatter.steps.push(std::array::from_fn(|i| {
            stored_step(s[i], 1.0) / 1024.0 + stored_step(t[3], 1.0) / 65_536.0
        }));
    }
    (transmission, scatter)
}

#[test]
fn metal_h4_filtered_decode_rejects_each_wrong_lookup_on_every_known_field_query() {
    pollster::block_on(async {
        let Some(crate::test_device::Opened {
            gpu: test_gpu,
            device,
            queue,
            ..
        }) = crate::test_device::open(
            "metal_h4_filtered_decode_rejects_each_wrong_lookup_on_every_known_field_query",
            Some(wgpu::Backend::Metal),
            crate::capabilities::DeviceProfile::Portable,
            &crate::capabilities::DeviceRestriction {
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            },
        )
        else {
            return;
        };
        let adapter = test_gpu.adapter.clone();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let caps = DeviceCapabilities::capture(&adapter, &device, 140);
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = SyntheticView::perspective(
            [0.0, 2.0, 0.0],
            [0.0, 2.0, -100.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            32_000.0,
        )
        .unwrap();
        let initial = AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(initial),
            1.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
        let products = prepared
            .scene_surface(&queue)
            .environment_products()
            .unwrap();
        let probe = aerial_probe(&device, &products);
        let cell = aerial_cell_texture(&device);
        let mut queries = Vec::new();
        for low in [0_u16, 15, 31] {
            for wz in [0.25_f32, 0.5, 0.75] {
                for wy in [0.25, 0.5, 0.75] {
                    for wx in [0.25, 0.5, 0.75] {
                        let first = 32_000.0 * (f32::from(low) / 32.0).powi(2);
                        let second = 32_000.0 * (f32::from(low + 1) / 32.0).powi(2);
                        queries.push([
                            (9.5 + wx) / 32.0,
                            (11.5 + wy) / 32.0,
                            first + (second - first) * wz,
                            0.0,
                        ]);
                    }
                }
            }
        }
        let clamped = [10.0 / 32.0, 12.0 / 32.0, 40_000.0, 0.0];
        let mut with_clamp = queries.clone();
        with_clamp.push(clamped);
        let fixed_queries = [0_u16, 15, 31].map(|low| {
            [
                10.5 / 32.0,
                12.5 / 32.0,
                32_000.0 * (f32::from(low) / 32.0).powi(2),
                0.0,
            ]
        });
        let live_queries: Vec<_> = with_clamp.iter().chain(&fixed_queries).copied().collect();
        for degrees in [2.0_f32, 0.0, -2.0, -4.0, -6.0, -9.0] {
            let (y, x) = degrees.to_radians().sin_cos();
            let frame = AtmosphereFrame::new([x, y, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(frame),
                    1.0,
                )
                .unwrap();
            submit(&device, &queue, &mut prepared, &target);
            let actual = probe.sample(&device, &queue, &products, &live_queries, 0);
            let (filtered, fixed) = actual.split_at(with_clamp.len());
            let surface = prepared.scene_surface(&queue);
            let atmosphere = surface.environment.unwrap();
            let stored = stored_aerial_cell(&device, &queue, &atmosphere.aerial_textures, &cell);
            let (transmission, scatter) = aerial_fields(&stored);
            for (query, &[t, s]) in with_clamp.iter().zip(filtered) {
                assert!(
                    within(t, aerial_expected(&transmission, *query)),
                    "H4 live transmission Sun {degrees} query {query:?} actual {t:?}, expected {:?}",
                    aerial_expected(&transmission, *query)
                );
                assert!(
                    within(s, aerial_expected(&scatter, *query)),
                    "H4 live scatter Sun {degrees} query {query:?} actual {s:?}, expected {:?}",
                    aerial_expected(&scatter, *query)
                );
            }
            for (q, &[t, s]) in fixed_queries.iter().zip(fixed) {
                for (actual, field) in [(t, &transmission), (s, &scatter)] {
                    let expected = aerial_expected(field, *q).0;
                    for c in 0..3 {
                        assert!(
                            (f64::from(actual[c]) - expected[c]).abs()
                                <= 1e-5 * expected[c].abs().max(1e-12),
                            "H4 fixed-texel decode {q:?} channel {c} {actual:?} expected {expected:?}"
                        );
                    }
                }
            }
        }
        let surface = prepared.scene_surface(&queue);
        let atmosphere = surface.environment.unwrap();
        observer::write_field(&queue, &atmosphere.aerial_textures[0], |x, y, z| {
            let [x, y, z] = [x, y, z].map(|v| (v % 2) as f32);
            [
                1.0 + x + y + z,
                1.0 + x + 2.0 * y + 3.0 * z,
                1.0 + 3.0 * x + y + 2.0 * z,
                0.0,
            ]
        });
        observer::write_field(&queue, &atmosphere.aerial_textures[1], |x, y, z| {
            let [x, y, z] = [x, y, z].map(|v| (v % 2) as f32);
            [
                0.25 + (x + y + z) / 16.0,
                0.25 + (x + 2.0 * y + 3.0 * z) / 16.0,
                0.25 + (3.0 * x + y + 2.0 * z) / 16.0,
                0.0,
            ]
        });
        let filtered = probe.sample(&device, &queue, &products, &with_clamp, 0);
        let stored = stored_aerial_cell(&device, &queue, &atmosphere.aerial_textures, &cell);
        let (transmission, scatter) = aerial_fields(&stored);
        transmission.assert_known_contrast();
        scatter.assert_known_contrast();
        let exposed = probe.sample(
            &device,
            &queue,
            &exposure_two(&prepared, &queue),
            &with_clamp,
            0,
        );
        for results in [&filtered, &exposed] {
            for (query, &[t, s]) in with_clamp.iter().zip(results) {
                assert!(
                    within(t, aerial_expected(&transmission, *query)),
                    "H4 known transmission {query:?} {t:?}"
                );
                assert!(
                    within(s, aerial_expected(&scatter, *query)),
                    "H4 known scatter {query:?} {s:?}"
                );
            }
        }
        let blend = "let blend = clamp((distance - first_distance) / max(0.001, second_distance - first_distance), 0.0, 1.0);";
        let nearest=products.shader_source.replace(
            "fn aerial_sample(distance_metres: f32, uv: vec2<f32>) -> AerialSample {",
            "fn aerial_sample(distance_metres: f32, uv: vec2<f32>) -> AerialSample {\n    let nearest_uv=(floor(uv*vec2<f32>(textureDimensions(aerial_scatter)))+vec2(0.5))/vec2<f32>(textureDimensions(aerial_scatter));",
        ).replace("uv, i32(low)","nearest_uv, i32(low)").replace("uv, i32(high)","nearest_uv, i32(high)");
        assert_eq!(nearest.matches("nearest_uv, i32(").count(), 4);
        let wrong = [
            variant(&products, blend, "let blend = 0.0;"),
            variant(&products, blend, "let blend = 1.0;"),
            EnvironmentProducts {
                layout: products.layout.clone(),
                shader_source: nearest,
                groups: products.groups.clone(),
            },
        ];
        for (kind, substitute) in wrong.iter().enumerate() {
            let probe = aerial_probe(&device, substitute);
            let actual = probe.sample(&device, &queue, substitute, &queries, 0);
            for (query, [t, s]) in queries.iter().zip(actual) {
                assert!(
                    !within(t, aerial_expected(&transmission, *query)),
                    "H4 wrong transmission lookup {kind} incorrectly passed at {query:?}"
                );
                assert!(
                    !within(s, aerial_expected(&scatter, *query)),
                    "H4 wrong scatter lookup {kind} incorrectly passed at {query:?}"
                );
            }
        }
    });
}

#[test]
fn metal_h2_sampler_precision_report() {
    pollster::block_on(async {
        let Some(crate::test_device::Opened {
            gpu: test_gpu,
            device,
            queue,
            ..
        }) = crate::test_device::open(
            "metal_h2_sampler_precision_report",
            Some(wgpu::Backend::Metal),
            crate::capabilities::DeviceProfile::Portable,
            &crate::capabilities::DeviceRestriction {
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            },
        )
        else {
            return;
        };
        let adapter = test_gpu.adapter.clone();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let info = adapter.get_info();
        let caps = DeviceCapabilities::capture(&adapter, &device, 140);
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = SyntheticView::perspective(
            [0.0, 2.0, 0.0],
            [0.0, 2.0, -100.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            32_000.0,
        )
        .unwrap();
        let frame = AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0,
        )
        .await
        .unwrap();
        let target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
        submit(&device, &queue, &mut prepared, &target);
        let surface = prepared.scene_surface(&queue);
        let chart = &surface.environment.unwrap().textures.sky_view;
        let products = surface.environment_products().unwrap();
        let queries: Vec<_> = (0..=256_u16)
            .map(|i| direction([20.5 / 192.0, (70.5 + f64::from(i) / 256.0) / 108.0], 0.0))
            .collect();
        let gpu_uv = observer::sample(
            &device,
            &queue,
            &products,
            "vec4(sky_uv(normalize(q.xyz),atmosphere.sun_lux.xyz),0.0,1.0)",
            &queries,
            0,
        );
        eprintln!(
            "H2 precision target {info:?}; texture Rgba16Float; linear sampler; 257 positions across one texel interval"
        );
        for (label, base, step) in [
            ("high-contrast", 0.0_f32, 1.0),
            ("low-contrast", 64.0, 0.0625),
        ] {
            observer::write_field(&queue, chart, |_, row, _| {
                let value = base + f32::from(u8::from(row % 2 == 1)) * step;
                [value, value, value, 0.0]
            });
            let texels = observer::texels(&device, &queue, chart);
            let field = Field {
                width: 192,
                height: 108,
                steps: texels
                    .iter()
                    .map(|v| {
                        std::array::from_fn(|i| {
                            stored_step(v[i], 1.0) / 1024.0 + stored_step(v[3], 1.0) / 65_536.0
                        })
                    })
                    .collect(),
                values: texels
                    .iter()
                    .map(|v| std::array::from_fn(|i| v[i] / 1024.0 + v[3] / 65_536.0))
                    .collect(),
            };
            let actual = observer::sample(
                &device,
                &queue,
                &products,
                "vec4(environment_sky_radiance(q.xyz),1.0)",
                &queries,
                0,
            );
            let mut levels = actual.iter().map(|row| row[0]).collect::<Vec<_>>();
            levels.sort_by(f32::total_cmp);
            levels.dedup_by(|a, b| a.to_bits() == b.to_bits());
            let maximum_address_error = gpu_uv
                .iter()
                .zip(&queries)
                .map(|(uv, query)| (f64::from(uv[1]) - sky_uv(*query, 0.0)[1]).abs() * 108.0)
                .fold(0.0_f64, f64::max);
            let failed = actual
                .iter()
                .zip(&queries)
                .filter(|(value, query)| {
                    !within(**value, field.sample(sky_uv(**query, 0.0), 0, 0, 0.0))
                })
                .count();
            let maximum_error_steps = actual
                .iter()
                .zip(&queries)
                .map(|(value, query)| {
                    let expected = field.sample(sky_uv(*query, 0.0), 0, 0, 0.0).0[0];
                    (f64::from(value[0]) - expected).abs()
                        / (stored_step(f64::from(base + step), 1.0) / 1024.0)
                })
                .fold(0.0_f64, f64::max);
            eprintln!("{label}: maximum error in stored steps={maximum_error_steps:.9}");
            assert_eq!(failed, 0, "revised H2 precision bound");
            eprintln!(
                "{label}: base={base} step={step}; distinct returned values={}; outside revised H2 bound={failed}/257; maximum GPU/f64 address error in texels={maximum_address_error:.9}",
                levels.len()
            );
            for index in [64, 128, 192] {
                let expected = field.sample(sky_uv(queries[index], 0.0), 0, 0, 0.0);
                let at_gpu_address = field.sample(
                    [f64::from(gpu_uv[index][0]), f64::from(gpu_uv[index][1])],
                    0,
                    0,
                    0.0,
                );
                eprintln!(
                    "weight={:.2}: actual={:.12e}, f64={:.12e}, allowance={:.12e}, f64 at observed GPU address={:.12e}",
                    index as f64 / 256.0,
                    actual[index][0],
                    expected.0[0],
                    expected.1[0],
                    at_gpu_address.0[0]
                );
            }
        }
    });
}

fn spectral_row(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    products: &EnvironmentProducts,
) {
    let elevation = -5.084_449_f32.to_radians();
    let frame = AtmosphereFrame::new(
        [elevation.cos(), elevation.sin(), 0.0],
        1.0,
        [1.0; 3],
        244.922_67,
        1,
    )
    .expect("valid spectral handoff fixture");
    prepared
        .update_frame(queue, target, scene, FrameLighting::Atmosphere(frame), 1.0)
        .expect("valid spectral handoff fixture");
    submit(device, queue, prepared, target);
    let (sy, cy) = 20.0_f32.to_radians().sin_cos();
    let (sa, ca) = 95.104_59_f32.to_radians().sin_cos();
    for products in [products, &exposure_two(prepared, queue)] {
        let result = observer::sample(
            device,
            queue,
            products,
            "vec4(environment_sky_radiance(q.xyz),1.0)",
            &[[cy * ca, sy, cy * sa, 0.0]],
            0,
        );
        for (c, expected) in [2.982_646e-5, 4.289_131e-5, 8.114_284e-5]
            .into_iter()
            .enumerate()
        {
            assert!(
                (f64::from(result[0][c]) - expected).abs() <= 0.025 * expected,
                "H2 P11 channel {c}: {:?} expected {expected}",
                result[0]
            );
        }
    }
}

fn known_sky_fields(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    products: &EnvironmentProducts,
) {
    let old = "textureSampleLevel(environment_sky,aerial_sampler,uv,0.0)";
    let wrong = [
        variant(
            products,
            old,
            "textureLoad(environment_sky,vec2<i32>(floor(uv*size)),0)",
        ),
        variant(
            products,
            old,
            "textureSampleLevel(environment_sky,aerial_sampler,vec2(uv.x,(floor(uv.y*size.y)+0.5)/size.y),0.0)",
        ),
        variant(
            products,
            old,
            "textureSampleLevel(environment_sky,aerial_sampler,vec2((floor(uv.x*size.x)+0.5)/size.x,uv.y),0.0)",
        ),
    ];
    for azimuth in [0.0_f32, 1.0] {
        let frame =
            AtmosphereFrame::new([azimuth.cos(), 0.0, azimuth.sin()], 1.0, [1.0; 3], 2.0, 1)
                .expect("known sky fixture admitted");
        prepared
            .update_frame(queue, target, scene, FrameLighting::Atmosphere(frame), 1.0)
            .expect("known sky fixture admitted");
        submit(device, queue, prepared, target);
        let surface = prepared.scene_surface(queue);
        let chart = &surface
            .environment
            .expect("atmosphere surface")
            .textures
            .sky_view;
        observer::write_field(queue, chart, |x, y, _| {
            [
                1.0 + (x % 2) as f32 + (y % 2) as f32,
                1.0 + (x % 2) as f32 + 2.0 * (y % 2) as f32,
                1.0 + 2.0 * (x % 2) as f32 + (y % 2) as f32,
                0.0,
            ]
        });
        let field = Field {
            width: 192,
            height: 108,
            steps: observer::texels(device, queue, chart)
                .iter()
                .map(|texel| std::array::from_fn(|i| stored_step(texel[i], 1.0) / 1024.0))
                .collect(),
            values: observer::texels(device, queue, chart)
                .iter()
                .map(|texel| std::array::from_fn(|i| texel[i] / 1024.0))
                .collect(),
        };
        field.assert_known_contrast();
        let mut queries = Vec::new();
        for row in [20.0, 52.0, 54.0, 80.0] {
            for wy in [0.25, 0.5, 0.75] {
                for wx in [0.25, 0.5, 0.75] {
                    queries.push(direction(
                        [(20.5 + wx) / 192.0, (row + 0.5 + wy) / 108.0],
                        f64::from(azimuth),
                    ));
                }
            }
        }
        let correct = observer::sample(
            device,
            queue,
            products,
            "vec4(environment_sky_radiance(q.xyz),1.0)",
            &queries,
            0,
        );
        let exposed = observer::sample(
            device,
            queue,
            &exposure_two(prepared, queue),
            "vec4(environment_sky_radiance(q.xyz),1.0)",
            &queries,
            0,
        );
        for ((query, actual), scaled) in queries.iter().zip(correct).zip(exposed) {
            let expected = field.sample(sky_uv(*query, f64::from(azimuth)), 0, 0, 0.0);
            assert!(
                within(actual, expected),
                "H2 known field {query:?} {actual:?} {expected:?}"
            );
            assert!(
                within(scaled, expected),
                "H2 exposure must stay outside lookup"
            );
        }
        for (kind, substitute) in wrong.iter().enumerate() {
            let result = observer::sample(
                device,
                queue,
                substitute,
                "vec4(environment_sky_radiance(q.xyz),1.0)",
                &queries,
                0,
            );
            for (query, actual) in queries.iter().zip(result) {
                assert!(
                    !within(
                        actual,
                        field.sample(sky_uv(*query, f64::from(azimuth)), 0, 0, 0.0)
                    ),
                    "H2 wrong lookup {kind} incorrectly passed at {query:?}"
                );
            }
        }
    }
}
