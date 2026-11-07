//! Independent cosine integrals on the actual half-float LUT, not an Earth-sky
//! oracle. CPU midpoint refinement and degree-two truncation are separate errors.

use super::super::{AtmospherePrepared, EXTENTS, prepare_diffuse};
use crate::{atmosphere::AtmosphereFrame, synthetic::SyntheticView};
use std::{f64::consts::PI, sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

const NORMALS: [[f64; 3]; 14] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [1.0, -1.0, -1.0],
    [-1.0, -1.0, -1.0],
];
const SCALE: [f64; 3] = [1.0 / 16384.0, 1.0 / 32768.0, 1.0 / 65536.0];
const READ: &str = concat!(
    include_str!("../../irradiance.wgsl"),
    r"
@group(0) @binding(0) var<storage,read> light: array<vec4<f32>,13>;
@group(0) @binding(1) var<storage,read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var image: texture_2d<f32>;
@compute @workgroup_size(1)
fn evaluate() {
    let normals=array<vec3<f32>,14>(
        vec3(1,0,0),vec3(-1,0,0),vec3(0,1,0),vec3(0,-1,0),vec3(0,0,1),vec3(0,0,-1),
        vec3(1,1,1),vec3(-1,1,1),vec3(1,-1,1),vec3(1,1,-1),
        vec3(-1,-1,1),vec3(-1,1,-1),vec3(1,-1,-1),vec3(-1,-1,-1));
    var coefficients: array<vec4<f32>,9>;
    for(var i=0u;i<9u;i++) { coefficients[i]=light[i+4u]; }
    for(var i=0u;i<14u;i++) {
        output[i]=vec4(max(evaluate_sky_diffuse(coefficients,normalize(normals[i])),vec3(0.0)),0.0);
    }
}
@compute @workgroup_size(8,8)
fn pixels(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(image);
    if all(id.xy<size) { output[id.y*size.x+id.x]=textureLoad(image,vec2<i32>(id.xy),0); }
}
"
);

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}
impl Gpu {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for directional diffuse verification");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("directional diffuse device");
        Self { device, queue }
    }

    fn buffer(&self, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("directional diffuse referee"),
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    fn light(&self) -> wgpu::Buffer {
        // Distinct authored direct-light prefix: the sky producer must preserve it.
        let mut values = vec![0.0_f32; crate::synthetic::LIGHT_BYTES / 4];
        for (i, value) in values[..12].iter_mut().enumerate() {
            *value = f32::from(u16::try_from(i).expect("twelve prefix words")) + 17.0;
        }
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("authored direct light and sky SH"),
                contents: values
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>()
                    .as_slice(),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC,
            })
    }

    fn collect(
        &self,
        mut encoder: wgpu::CommandEncoder,
        output: &wgpu::Buffer,
        bytes: u64,
    ) -> Vec<[f64; 4]> {
        let readback = self.buffer(
            bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        encoder.copy_buffer_to_buffer(output, 0, &readback, 0, bytes);
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
            send.send(result).expect("diffuse referee receiver lives");
        });
        let submission = self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(20)),
            })
            .expect("diffuse referee GPU completes");
        receive
            .recv()
            .expect("diffuse map callback")
            .expect("diffuse mapped");
        readback
            .slice(..)
            .get_mapped_range()
            .expect("diffuse readback")
            .as_chunks::<16>()
            .0
            .iter()
            .map(|row| {
                std::array::from_fn(|c| {
                    f64::from(f32::from_le_bytes(
                        row[c * 4..c * 4 + 4].try_into().expect("float word"),
                    ))
                })
            })
            .collect()
    }

    fn observe(
        &self,
        mut encoder: wgpu::CommandEncoder,
        light: &wgpu::Buffer,
        sky: Option<&wgpu::Texture>,
    ) -> Vec<[f64; 4]> {
        let entry = if sky.is_some() { "pixels" } else { "evaluate" };
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("production diffuse evaluator and sky reader"),
                source: wgpu::ShaderSource::Wgsl(READ.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let count = sky.map_or(27, |t| u64::from(t.width()) * u64::from(t.height()));
        let output = self.buffer(
            count * 16,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let view = sky.map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
        let entries = [
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
            if let Some(view) = &view {
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(view),
                }
            } else {
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light.as_entire_binding(),
                }
            },
        ];
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            if let Some(t) = sky {
                pass.dispatch_workgroups(t.width().div_ceil(8), t.height().div_ceil(8), 1);
            } else {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        if sky.is_none() {
            encoder.copy_buffer_to_buffer(light, 0, &output, 14 * 16, 208);
        }
        self.collect(encoder, &output, count * 16)
    }

    fn project(&self, sky: &wgpu::Texture, frame: [f32; 32]) -> Vec<[f64; 4]> {
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: frame.map(f32::to_le_bytes).as_flattened(),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let light = self.light();
        let (pipeline, group) = prepare_diffuse(
            &self.device,
            &uniform,
            &sky.create_view(&wgpu::TextureViewDescriptor::default()),
            &sampler,
            &light,
        );
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        self.observe(encoder, &light, None)
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn normal(i: usize) -> [f64; 3] {
    let n = NORMALS[i];
    let r = dot(n, n).sqrt();
    n.map(|v| v / r)
}
fn basis(n: [f64; 3]) -> [f64; 9] {
    let [x, y, z] = n;
    [
        0.25,
        x / 2.0,
        y / 2.0,
        z / 2.0,
        15.0 / 32.0 * (x * x - 1.0 / 3.0),
        15.0 / 32.0 * (y * y - 1.0 / 3.0),
        15.0 / 16.0 * x * y,
        15.0 / 16.0 * x * z,
        15.0 / 16.0 * y * z,
    ]
}

// Independent f64 sphere coordinates. Height is fixed at two metres, matching
// the declared observer, not rounded from the production mapping uniforms.
fn horizon() -> f64 {
    -(6_371_000.0_f64 / (6_371_002.0)).acos()
}
fn direction(x: usize, y: usize) -> [f64; 3] {
    let h = horizon();
    let elevation = if y < 54 {
        h - (PI / 2.0 + h) * ((53 - y) as f64 / 53.0).powi(2)
    } else {
        h + (PI / 2.0 - h) * ((y - 54) as f64 / 53.0).powi(2)
    };
    let phi = ((x as f64 + 0.5) / 192.0 * 2.0 - 1.0) * PI;
    [
        elevation.cos() * phi.cos(),
        elevation.sin(),
        elevation.cos() * phi.sin(),
    ]
}

fn half_value(bits: u16) -> f64 {
    let exponent = i32::from(bits >> 10);
    let fraction = f64::from(bits & 1023);
    if exponent == 0 {
        fraction * 2.0_f64.powi(-24)
    } else {
        (1.0 + fraction / 1024.0) * 2.0_f64.powi(exponent - 15)
    }
}
fn half(value: f64) -> u16 {
    assert!((0.0..=65504.0).contains(&value));
    let (mut lo, mut hi) = (0_u16, 0x7bff_u16);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if half_value(mid) <= value {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0x7bff {
        return lo;
    }
    let (a, b) = (value - half_value(lo), half_value(lo + 1) - value);
    if a > b || (a.total_cmp(&b).is_eq() && lo & 1 == 1) {
        lo + 1
    } else {
        lo
    }
}

fn radiance(kind: u8, n: [f64; 3]) -> f64 {
    let [x, y, z] = if kind == 2 { [n[2], n[1], -n[0]] } else { n };
    match kind {
        0 => 1.0,
        3 => f64::from(y >= 0.0),
        4 => 0.0,
        // A nonnegative, narrow zenith lobe: half maximum at mu=0.9576.
        // Smooth support avoids testing aliasing of an unresolved hard cap edge.
        5 => y.max(0.0).powi(16),
        _ => {
            0.75 + 0.2 * x - 0.3 * y + 0.1 * z + 0.15 * x * x
                - 0.1 * y * y
                - 0.05 * z * z
                - 0.12 * x * y
                + 0.16 * x * z
                - 0.08 * y * z
        }
    }
}
fn analytic(kind: u8, n: [f64; 3]) -> f64 {
    let [x, y, z] = if kind == 2 { [n[2], n[1], -n[0]] } else { n };
    match kind {
        0 => PI,
        3 => PI * (1.0 + y) / 2.0,
        4 => 0.0,
        _ => {
            PI * 0.75
                + 2.0 * PI / 3.0 * (0.2 * x - 0.3 * y + 0.1 * z)
                + PI / 4.0
                    * (0.15 * x * x - 0.1 * y * y - 0.05 * z * z - 0.12 * x * y + 0.16 * x * z
                        - 0.08 * y * z)
        }
    }
}

fn image(gpu: &Gpu, kind: u8) -> (wgpu::Texture, Vec<[f64; 3]>) {
    let [width, height] = EXTENTS[2];
    let sky = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("analytic nonnegative incident field"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut bytes = Vec::new();
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let l = radiance(kind, direction(x as usize, y as usize));
            assert!(l >= 0.0);
            let encoded = SCALE.map(|scale| half(l * scale * 1024.0));
            pixels.push(encoded.map(|v| half_value(v) / 1024.0));
            for value in encoded.into_iter().chain([0x3c00]) {
                bytes.extend(value.to_le_bytes());
            }
        }
    }
    gpu.queue.write_texture(
        sky.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: Some(height),
        },
        sky.size(),
    );
    (sky, pixels)
}

fn sample(pixels: &[[f64; 3]], n: [f64; 3], sun_phi: f64) -> [f64; 3] {
    let h = horizon();
    let e = n[1].clamp(-1.0, 1.0).asin();
    let relative = n[2].atan2(n[0]) - sun_phi;
    let x = ((relative.sin().atan2(relative.cos()) / PI + 1.0) * 96.0 - 0.5).clamp(0.0, 191.0);
    let y = if e >= h {
        54.0 + 53.0 * ((e - h) / (PI / 2.0 - h)).clamp(0.0, 1.0).sqrt()
    } else {
        53.0 - 53.0 * ((h - e) / (PI / 2.0 + h)).clamp(0.0, 1.0).sqrt()
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped shipping LUT indices"
    )]
    let (ix, iy) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    std::array::from_fn(|c| {
        let row = |j: usize| {
            pixels[j * 192 + ix][c] * (1.0 - fx) + pixels[j * 192 + (ix + 1).min(191)][c] * fx
        };
        row(iy) * (1.0 - fy) + row((iy + 1).min(107)) * fy
    })
}

struct Integral {
    coefficients: [[f64; 3]; 9],
    direct: [[f64; 3]; 14],
    mass: [f64; 3],
}
fn integrate(pixels: &[[f64; 3]], sun_phi: f64, solar: [f64; 3], resolution: u32) -> Integral {
    let mut result = Integral {
        coefficients: [[0.0; 3]; 9],
        direct: [[0.0; 3]; 14],
        mass: [0.0; 3],
    };
    let hm = horizon().sin();
    // Resolve both one-sided horizon limits. These are midpoint nodes, not
    // the producer's GL16 nodes; all accumulation and mapping remain f64.
    for (sign, span) in [(-1.0, 1.0 + hm), (1.0, 1.0 - hm)] {
        let angular_weight = 2.0 * PI / f64::from(resolution * resolution * 2);
        for y in 0..resolution {
            let t = (f64::from(y) + 0.5) / f64::from(resolution);
            let mu = hm + sign * span * t * t;
            let weight = angular_weight * 2.0 * span * t;
            for x in 0..resolution * 2 {
                let phi = 2.0 * PI * (f64::from(x) + 0.5) / f64::from(resolution * 2);
                let r = (1.0 - mu * mu).sqrt();
                let n = [r * phi.cos(), mu, r * phi.sin()];
                let l = sample(pixels, n, sun_phi);
                let b = basis(n);
                for c in 0..3 {
                    let energy = l[c] * weight * solar[c];
                    result.mass[c] += energy;
                    for (i, value) in b.iter().enumerate() {
                        result.coefficients[i][c] += energy * value;
                    }
                    for i in 0..14 {
                        result.direct[i][c] += energy * dot(n, normal(i)).max(0.0);
                    }
                }
            }
        }
    }
    result
}

#[expect(
    clippy::print_stderr,
    reason = "the numerical test emits raw projection, truncation and refinement rows for review"
)]
fn compare(
    observed: &[[f64; 4]],
    pixels: &[[f64; 3]],
    sun_phi: f64,
    solar: [f64; 3],
    label: &str,
    resolution: u32,
) {
    let coarse = integrate(pixels, sun_phi, solar, resolution / 2);
    let fine = integrate(pixels, sun_phi, solar, resolution);
    for (c, &mass) in fine.mass.iter().enumerate() {
        let maximum_coefficient_refinement = coarse
            .coefficients
            .iter()
            .zip(&fine.coefficients)
            .map(|(a, b)| (a[c] - b[c]).abs())
            .fold(0.0_f64, f64::max);
        let maximum_direct_refinement = coarse
            .direct
            .iter()
            .zip(&fine.direct)
            .map(|(a, b)| (a[c] - b[c]).abs())
            .fold(0.0_f64, f64::max);
        // Predeclared empirical convergence requirement, not an error proof.
        // A failed refinement is unresolved; it cannot enlarge GPU acceptance.
        let refinement_budget = 0.0025 * mass;
        assert!(
            maximum_coefficient_refinement <= refinement_budget
                && maximum_direct_refinement <= refinement_budget,
            "{label} channel{c} CPU reference unresolved: max coefficient delta {maximum_coefficient_refinement}, max direct delta {maximum_direct_refinement}, budget {refinement_budget}"
        );
        let projection_budget = 0.01 * mass + 1e-12;
        let mut maximum_projection_error = 0.0_f64;
        for i in 0..9 {
            let error = (observed[18 + i][c] - fine.coefficients[i][c]).abs();
            let refinement = (coarse.coefficients[i][c] - fine.coefficients[i][c]).abs();
            maximum_projection_error = maximum_projection_error.max(error);
            assert!(
                error <= projection_budget + 2.0 * refinement,
                "{label} c{c} coefficient{i}: GPU {}, CPU {}, midpoint delta {refinement}, budget {projection_budget}",
                observed[18 + i][c],
                fine.coefficients[i][c]
            );
        }
        let mut maximum_direct_error = 0.0_f64;
        let mut maximum_bright_relative_error = 0.0_f64;
        // k2(t)=3/32+t/2+15t²/32 differs from max(t,0) by at most 3/32.
        // Nonnegative L therefore bounds truncation by 3 M0/32, before clamp.
        for (i, observed_normal) in observed[..14].iter().enumerate() {
            let refinement = (coarse.direct[i][c] - fine.direct[i][c]).abs();
            let direct_error = (observed_normal[c] - fine.direct[i][c]).abs();
            maximum_direct_error = maximum_direct_error.max(direct_error);
            if fine.direct[i][c] > mass * 0.01 {
                maximum_bright_relative_error =
                    maximum_bright_relative_error.max(direct_error / fine.direct[i][c]);
            }
            let n = normal(i);
            let [x, y, z] = n;
            let evaluation = [
                1.0,
                x,
                y,
                z,
                x * x - z * z,
                y * y - z * z,
                x * y,
                x * z,
                y * z,
            ];
            let projection_error: f64 = evaluation
                .iter()
                .enumerate()
                .map(|(j, b)| b.abs() * (observed[18 + j][c] - fine.coefficients[j][c]).abs())
                .sum();
            let bound = 3.0 * mass / 32.0 + projection_error + 2.0 * refinement + 1e-12;
            assert!(
                observed_normal[c].is_finite() && direct_error <= bound,
                "{label} c{c} normal{i}: GPU {}, direct {}, truncation+projection+refinement {bound}",
                observed_normal[c],
                fine.direct[i][c]
            );
        }
        eprintln!(
            "{label} reference_resolution={resolution} channel={c} M0={} max_coefficient_error={maximum_projection_error} max_coefficient_refinement={maximum_coefficient_refinement} SH_truncation_bound={} max_direct_error={maximum_direct_error} max_direct_refinement={maximum_direct_refinement} bright_relative_error={maximum_bright_relative_error}",
            mass,
            3.0 * mass / 32.0
        );
    }
    assert!(observed.iter().flatten().all(|v| v.is_finite()));
}

#[test]
fn analytic_fields_preserve_direction_colour_and_physical_units() {
    pollster::block_on(async {
        let gpu = Gpu::new().await;
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        for (kind, lux, colour) in [
            (0, 1.0, [1.0; 3]),
            (1, 1.0, [1.0; 3]),
            (2, 127_000.0, [0.25, 2.0, 0.0]),
            (3, 2.0, [1.0, 0.5, 0.125]),
            (1, 0.0, [1.0; 3]),
            (4, 1.0, [1.0; 3]),
            (5, 1.0, [1.0; 3]),
        ] {
            let (sky, pixels) = image(&gpu, kind);
            let mut frame = [0.0_f32; 32];
            frame[19] = 2.0;
            frame[20] = 1.0;
            frame[23] = lux;
            frame[24..27].copy_from_slice(&colour);
            frame[27] = 1.0 / 1024.0;
            frame[28..32].copy_from_slice(&super::super::super::model::sky_mapping(2.0));
            let observed = gpu.project(&sky, frame);
            let solar = colour.map(|c| f64::from(c * lux));
            compare(
                &observed,
                &pixels,
                0.0,
                solar,
                &format!("analytic{kind} solar{solar:?}"),
                // The narrow polar lobe needs finer midpoint reference nodes.
                if kind == 5 { 512 } else { 128 },
            );
            if kind == 5 {
                let marker = observed[17][3];
                assert!(marker > 1.0 && marker <= 65.0 && marker.fract().abs() < f64::EPSILON);
                assert!(
                    observed[18][3] < 0.0,
                    "narrow positive sky has negative raw SH probes"
                );
                assert!(
                    observed[..14]
                        .iter()
                        .all(|v| v[..3].iter().all(|c| *c >= 0.0))
                );
            } else {
                for (i, observed_normal) in observed[..14].iter().enumerate() {
                    for (c, &actual) in observed_normal[..3].iter().enumerate() {
                        let expected = analytic(kind, normal(i)) * SCALE[c] * solar[c];
                        let budget = 0.004 * PI * SCALE[c] * solar[c] + 1e-12;
                        assert!(
                            (actual - expected).abs() <= budget,
                            "analytic{kind} normal{i} channel{c}: {actual} vs {expected}, budget {budget}"
                        );
                    }
                }
            }
            for (i, prefix) in observed[14..17].iter().enumerate() {
                for (c, &actual) in prefix.iter().enumerate() {
                    assert_eq!(actual, (i * 4 + c) as f64 + 17.0);
                }
            }
        }
        assert!(
            scope.pop().await.is_none(),
            "analytic diffuse pipeline validates"
        );
    });
}

#[test]
fn shipping_sky_projection_separates_sampling_from_sh_truncation() {
    pollster::block_on(async {
        let gpu = Gpu::new().await;
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let light = gpu.light();
        let frame = |degrees: i16, azimuth: i16| {
            let (s, c) = f32::from(degrees).to_radians().sin_cos();
            let (sa, ca) = f32::from(azimuth).to_radians().sin_cos();
            AtmosphereFrame::new([c * ca, s, c * sa], 1.0, [1.0; 3], 2.0, 1)
                .expect("declared sky frame")
        };
        let mut prepared = AtmospherePrepared::new(
            &gpu.device,
            &gpu.queue,
            1,
            frame(45, 0),
            &SyntheticView::default(),
            &light,
        );
        for (degrees, azimuth) in [
            (90, 0),
            (45, 0),
            (45, 90),
            (0, 0),
            (-3, 0),
            (-6, 0),
            (-9, 0),
            (-12, 0),
        ] {
            prepared.update(
                &gpu.queue,
                frame(degrees, azimuth),
                &SyntheticView::default(),
                [1, 1],
            );
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            for index in [0, 1, 3, 2] {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&prepared.compute[index]);
                pass.set_bind_group(0, &prepared.groups[index], &[]);
                let [w, h] = EXTENTS[index];
                if index == 3 {
                    pass.dispatch_workgroups(w, 1, 1);
                } else {
                    pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
                }
            }
            let pixels = gpu.observe(encoder, &light, Some(&prepared.textures[2]));
            let pixels: Vec<_> = pixels
                .iter()
                .map(|v| [v[0] / 1024.0, v[1] / 1024.0, v[2] / 1024.0])
                .collect();
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&prepared.diffuse_pipeline);
                pass.set_bind_group(0, &prepared.diffuse_group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
            let observed = gpu.observe(encoder, &light, None);
            compare(
                &observed,
                &pixels,
                f64::from(azimuth).to_radians(),
                [1.0; 3],
                &format!("shipping Sun{degrees} azimuth{azimuth}"),
                128,
            );
        }
        assert!(
            scope.pop().await.is_none(),
            "shipping diffuse pipeline validates"
        );
    });
}
