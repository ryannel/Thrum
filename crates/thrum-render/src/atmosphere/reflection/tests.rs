//! Exercise the production prefilter and DFG, with a separate incident-direction
//! quadrature as the BRDF oracle. No production COPY_SRC usage is needed.

use super::{CUBE_SIZE, DFG_SIZE, MIP_COUNT, ReflectionPrepared};
use std::{f64::consts::PI, sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

const READ: &str = r"
@group(0) @binding(0) var source: texture_2d_array<f32>;
@group(0) @binding(1) var<storage,read_write> output: array<vec4<f32>>;
@compute @workgroup_size(8,8)
fn read(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(source);
    if any(id.xy>=size) || id.z>=textureNumLayers(source) { return; }
    output[(id.z*size.y+id.y)*size.x+id.x]=textureLoad(source,vec2<i32>(id.xy),i32(id.z),0);
}
";

const SAMPLE: &str = r"
@group(0) @binding(0) var source: texture_cube<f32>;
@group(0) @binding(1) var lookup: sampler;
@group(0) @binding(2) var<storage,read> queries: array<vec4<f32>>;
@group(0) @binding(3) var<storage,read_write> output: array<vec4<f32>>;
@compute @workgroup_size(64)
fn sample_cube(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=arrayLength(&queries) { return; }
    let query=queries[id.x];
    output[id.x]=textureSampleLevel(source,lookup,normalize(query.xyz),query.w);
}
";

const CONSUMER: &str = concat!(
    include_str!("../storage.wgsl"),
    "\n",
    include_str!("../brdf.wgsl"),
    r"
struct AerialFrame {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    sky_mapping: vec4<f32>,
}
@group(1) @binding(7) var<uniform> aerial_frame: AerialFrame;
fn ambient_irradiance(n: vec3<f32>) -> vec3<f32> {
    return 3.14159265359*vec3(1.0,0.5,0.25)
        *aerial_frame.sun_lux.w*aerial_frame.colour_pre_exposure.rgb;
}
",
    include_str!("../../synthetic_environment.wgsl"),
    r"
@group(0) @binding(0) var<storage,read> queries: array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> output: array<vec4<f32>>;
@compute @workgroup_size(64)
fn check_consumer(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=arrayLength(&queries) { return; }
    let q=queries[id.x];
    let view=vec3(sqrt(max(0.0,1.0-q.w*q.w)),0.0,q.w);
    output[id.x]=vec4(indirect_radiance(vec3(q.x),q.y,q.z,vec3(0.0,0.0,1.0),view,1.0),1.0);
}
"
);

fn reader(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("reflection sampled readback"),
        source: wgpu::ShaderSource::Wgsl(READ.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("reflection sampled readback"),
        layout: None,
        module: &shader,
        entry_point: Some("read"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    texture: &wgpu::Texture,
    mip: u32,
) -> Vec<[f32; 4]> {
    let width = (texture.width() >> mip).max(1);
    let height = (texture.height() >> mip).max(1);
    let layers = texture.depth_or_array_layers();
    let bytes = u64::from(width) * u64::from(height) * u64::from(layers) * 16;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        base_mip_level: mip,
        mip_level_count: Some(1),
        ..Default::default()
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), layers);
    }
    collect(device, queue, encoder, &output, bytes)
}

fn collect(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    output: &wgpu::Buffer,
    bytes: u64,
) -> Vec<[f32; 4]> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(output, 0, &readback, 0, bytes);
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("reflection receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("reflection correctness completes");
    receive
        .recv()
        .expect("reflection map callback")
        .expect("reflection mapped");
    readback
        .slice(..)
        .get_mapped_range()
        .expect("reflection data")
        .as_chunks::<16>()
        .0
        .iter()
        .map(|bytes| {
            std::array::from_fn(|word| {
                f32::from_le_bytes(
                    bytes[word * 4..word * 4 + 4]
                        .try_into()
                        .expect("f32 record"),
                )
            })
        })
        .collect()
}

fn sample_cube(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    queries: &[[f32; 4]],
) -> Vec<[f32; 4]> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual cube sampling"),
        source: wgpu::ShaderSource::Wgsl(SAMPLE.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("actual cube sampling"),
        layout: None,
        module: &shader,
        entry_point: Some("sample_cube"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let input: Vec<_> = queries
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &input,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = u64::try_from(queries.len()).expect("bounded probes") * 16;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(queries.len())
                .expect("bounded probes")
                .div_ceil(64),
            1,
            1,
        );
    }
    collect(device, queue, encoder, &output, bytes)
}

fn check_consumer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    uniform: &wgpu::Buffer,
    prepared: &ReflectionPrepared,
) {
    let queries: Vec<_> = [0.0_f32, 0.18, 0.65, 1.0]
        .into_iter()
        .flat_map(|base| {
            [0.0, 1.0].into_iter().flat_map(move |metal| {
                [0.045, 0.5, 1.0].into_iter().flat_map(move |roughness| {
                    [-0.5, 0.0, 0.01, 0.5, 1.0].map(|no_v| [base, metal, roughness, no_v])
                })
            })
        })
        .collect();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual indirect consumer"),
        source: wgpu::ShaderSource::Wgsl(CONSUMER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("actual indirect consumer"),
        layout: None,
        module: &shader,
        entry_point: Some("check_consumer"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let input: Vec<_> = queries
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &input,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = u64::try_from(queries.len()).expect("bounded material sweep") * 16;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let inputs = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let (cube, dfg, sampler) = prepared.sampled();
    let environment = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 7,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: wgpu::BindingResource::TextureView(cube),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: wgpu::BindingResource::TextureView(dfg),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    let mut baseline = Vec::new();
    for (run, solar) in [1.0_f32, 0.0, 2.0].into_iter().enumerate() {
        queue.write_buffer(
            uniform,
            92,
            [solar, 1.0, 1.0, 1.0].map(f32::to_le_bytes).as_flattened(),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &inputs, &[]);
            pass.set_bind_group(1, &environment, &[]);
            pass.dispatch_workgroups(
                u32::try_from(queries.len())
                    .expect("bounded sweep")
                    .div_ceil(64),
                1,
                1,
            );
        }
        let samples = collect(device, queue, encoder, &output, bytes);
        for (index, (sample, query)) in samples.iter().zip(&queries).enumerate() {
            for (channel, source) in [1.0, 0.5, 0.25].into_iter().enumerate() {
                let actual = sample[channel];
                assert!(
                    actual.is_finite() && actual >= 0.0,
                    "consumer {query:?}: {sample:?}"
                );
                // Two fp16 products and DFG interpolation allow a small
                // rounding envelope, not unaccounted diffuse+specular energy.
                assert!(
                    actual <= solar * source * 1.002 + 1e-6,
                    "white furnace solar={solar} query={query:?}: {sample:?}"
                );
                if query[3] <= 0.0 || run == 1 {
                    assert_eq!(actual, 0.0, "back-facing/zero-source consumer {query:?}");
                } else if query[0] >= 0.18 && query[1] > 0.5 {
                    assert!(
                        actual > 0.0,
                        "metal receives environment without any direct-light binding"
                    );
                }
                if run == 2 {
                    let first: [f32; 4] = baseline[index];
                    assert!(
                        (actual - 2.0 * first[channel]).abs() < 2e-6,
                        "consumer solar scaling {query:?}: {sample:?} versus {first:?}"
                    );
                }
            }
        }
        if run == 0 {
            baseline = samples;
        }
    }
}

// All fixture values lie in [1024,2048]. Round their known finite-positive
// f32 representation to nearest-even fp16; this is not a general half codec.
#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded fixture radiance rounds to f32 before fp16"
)]
fn fixture_half(value: f64) -> [u8; 2] {
    assert!((1024.0..=2048.0).contains(&value));
    let bits = (value as f32).to_bits();
    let rounded = ((bits + 0xfff + ((bits >> 13) & 1)) >> 13) - 0x1c000;
    u16::try_from(rounded)
        .expect("fixture half range")
        .to_le_bytes()
}

fn analytic_sky(sun_azimuth: f64) -> Vec<u8> {
    let mapping = crate::atmosphere::model::sky_mapping(2.0).map(f64::from);
    let mut bytes = Vec::with_capacity(192 * 108 * 8);
    for row in 0..108_u32 {
        let (q, span) = if row < 54 {
            (f64::from(53 - row) / 53.0, -mapping[3])
        } else {
            (f64::from(row - 54) / 53.0, mapping[2])
        };
        let elevation = mapping[1] + span * q * q;
        for column in 0..192 {
            let azimuth = ((f64::from(column) + 0.5) / 192.0 * 2.0 - 1.0) * PI + sun_azimuth;
            let direction = [
                elevation.cos() * azimuth.cos(),
                elevation.sin(),
                elevation.cos() * azimuth.sin(),
            ];
            for component in direction {
                bytes.extend(fixture_half((1.5 + 0.5 * component) * 1024.0));
            }
            bytes.extend(0_u16.to_le_bytes());
        }
    }
    bytes
}

fn probe_groups() -> Vec<Vec<[f32; 3]>> {
    let mut groups = Vec::new();
    for axis in 0..3 {
        for sign in [-1.0, 1.0] {
            let mut direction = [0.0; 3];
            direction[axis] = sign;
            groups.push(vec![direction]);
        }
    }
    // Every edge: approach from the two adjacent face interiors.
    for absent in 0..3 {
        for a in [-1.0, 1.0] {
            for b in [-1.0, 1.0] {
                let mut direction = [0.0; 3];
                direction[(absent + 1) % 3] = a;
                direction[(absent + 2) % 3] = b;
                groups.push(
                    [(absent + 1) % 3, (absent + 2) % 3]
                        .map(|axis| {
                            let mut perturbed = direction;
                            perturbed[axis] *= 1.0001;
                            perturbed
                        })
                        .to_vec(),
                );
            }
        }
    }
    // Every corner: approach from each of its three incident faces.
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                groups.push(
                    (0..3)
                        .map(|axis| {
                            let mut direction = [x, y, z];
                            direction[axis] *= 1.0001;
                            direction
                        })
                        .collect(),
                );
            }
        }
    }
    groups
}

fn check_analytic_cube(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    uniform: &wgpu::Buffer,
    sky: &wgpu::Texture,
    prepared: &ReflectionPrepared,
) {
    // Unit radiance stays in [1,2]: two fp16 roundings contribute at most
    // 1/1024. The separate approximation allowance covers the 128-point
    // filter and finite chart/cube resolution, not oracle uncertainty.
    const HALF_ALLOWANCE: f64 = 1.0 / 1024.0;
    const FILTER_ALLOWANCE: f64 = 0.019;
    let groups = probe_groups();
    let queries: Vec<_> = (0..MIP_COUNT)
        .flat_map(|mip| {
            groups
                .iter()
                .flatten()
                .map(move |d| [d[0], d[1], d[2], mip as f32])
        })
        .collect();
    for (sun, azimuth) in [([1.0_f32, 0.0, 0.0], 0.0), ([0.0, 0.0, 1.0], PI * 0.5)] {
        queue.write_buffer(uniform, 80, sun.map(f32::to_le_bytes).as_flattened());
        queue.write_texture(
            sky.as_image_copy(),
            &analytic_sky(azimuth),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(192 * 8),
                rows_per_image: Some(108),
            },
            sky.size(),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for stage in 1..=MIP_COUNT as usize {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            prepared
                .encode(stage, &mut pass)
                .expect("analytic cube stage");
        }
        queue.submit([encoder.finish()]);
        let samples = sample_cube(device, queue, &prepared.cube, &queries);
        let mut index = 0;
        for mip in 0..MIP_COUNT {
            for (group_index, directions) in groups.iter().enumerate() {
                let group_start = index;
                for direction in directions {
                    let length = direction
                        .iter()
                        .map(|v| f64::from(*v).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    for (channel, component) in direction.iter().enumerate() {
                        let actual = f64::from(samples[index][channel]) / 1024.0;
                        // At mip0 the .045 lobe is nearly a delta. At mip6,
                        // GGX alpha1's normalized cosine filter gives 2/3 N;
                        // only axes coincide with the six 1x1 stored centres.
                        if mip == 0 || (mip == MIP_COUNT - 1 && group_index < 6) {
                            let scale = if mip == 0 { 0.5 } else { 1.0 / 3.0 };
                            let expected = 1.5 + scale * f64::from(*component) / length;
                            assert!(
                                (actual - expected).abs() < HALF_ALLOWANCE + FILTER_ALLOWANCE,
                                "analytic cube az={azimuth} mip={mip} dir={direction:?} channel={channel}: {actual} != {expected}"
                            );
                        }
                        assert!(actual.is_finite());
                        // Same-edge/corner approaches must remain continuous
                        // at every mip, without claiming coarse-level accuracy.
                        let first = f64::from(samples[group_start][channel]) / 1024.0;
                        assert!(
                            (actual - first).abs() < 0.01,
                            "cube seam az={azimuth} mip={mip} group={group_index}: {actual} != {first}"
                        );
                    }
                    index += 1;
                }
            }
        }
    }
}

// Gauss-Legendre integration over incident NoL and azimuth. Reflection symmetry
// permits twice the [0,pi] integral, clustering nodes near the grazing corner.
// This does not sample the producer's GGX/VNDF distribution.
fn legendre(order: u32) -> Vec<(f64, f64)> {
    (0..order)
        .map(|index| {
            let mut x = (PI * (f64::from(index) + 0.75) / (f64::from(order) + 0.5)).cos();
            let derivative = loop {
                let (mut previous, mut current) = (1.0, x);
                for degree in 2..=order {
                    let next = ((2.0 * f64::from(degree) - 1.0) * x * current
                        - (f64::from(degree) - 1.0) * previous)
                        / f64::from(degree);
                    (previous, current) = (current, next);
                }
                let derivative = f64::from(order) * (x * current - previous) / (x * x - 1.0);
                let correction = current / derivative;
                x -= correction;
                if correction.abs() < 1e-15 {
                    break derivative;
                }
            };
            (
                x.midpoint(1.0),
                1.0 / ((1.0 - x * x) * derivative * derivative),
            )
        })
        .collect()
}

fn dfg_reference(no_v: f64, roughness: f64, order: u32) -> [f64; 2] {
    let alpha_squared = roughness.powi(4);
    let view_radial = (1.0 - no_v * no_v).sqrt();
    let quadrature = legendre(order);
    let mut result = [0.0; 2];
    for &(no_l, weight) in &quadrature {
        let light_radial = (1.0 - no_l * no_l).sqrt();
        for &(azimuth, azimuth_weight) in &quadrature {
            let phi = PI * azimuth;
            let l_dot_v = no_l * no_v + light_radial * view_radial * phi.cos();
            let half_length = (2.0 + 2.0 * l_dot_v).sqrt();
            let no_h = (no_l + no_v) / half_length;
            let vo_h = half_length * 0.5;
            let distribution =
                alpha_squared / (PI * (1.0 - no_h * no_h + alpha_squared * no_h * no_h).powi(2));
            // G2/(4 NoV) analytically cancelled before evaluating NoV=0.
            let smith_denominator = no_l
                * (alpha_squared + (1.0 - alpha_squared) * no_v * no_v).sqrt()
                + no_v * (alpha_squared + (1.0 - alpha_squared) * no_l * no_l).sqrt();
            let measure = distribution * no_l / (2.0 * smith_denominator)
                * weight
                * 2.0
                * PI
                * azimuth_weight;
            let fresnel_power = (1.0 - vo_h.clamp(0.0, 1.0)).powi(5);
            result[0] += measure * (1.0 - fresnel_power);
            result[1] += measure * fresnel_power;
        }
    }
    result
}

fn check_dfg(samples: &[[f32; 4]]) {
    assert_eq!(samples.len(), (DFG_SIZE * DFG_SIZE) as usize);
    for (index, sample) in samples.iter().enumerate() {
        assert!(
            sample.iter().all(|value| value.is_finite()),
            "DFG {index}: {sample:?}"
        );
        assert!(
            sample[0] >= 0.0 && sample[1] >= 0.0,
            "DFG {index}: {sample:?}"
        );
        assert!(
            sample[0] + sample[1] <= 1.001,
            "DFG energy {index}: {sample:?}"
        );
    }
    let lookup = |x: u32, y: u32| samples[(y * DFG_SIZE + x) as usize];
    let normal = lookup(64, 64);
    assert!((f64::from(normal[0] + normal[1]) - (1.0 - 2.0_f64.ln())).abs() < 0.001);
    let grazing = lookup(0, 64);
    assert!((grazing[0] + grazing[1] - 1.0).abs() < 0.001);
    for (x, y) in [(0, 32), (8, 48), (32, 32), (32, 64), (64, 32), (64, 64)] {
        let no_v = f64::from(x) / 64.0;
        let roughness = 0.045 + 0.955 * f64::from(y) / 64.0;
        let coarse = dfg_reference(no_v, roughness, 128);
        let refined = dfg_reference(no_v, roughness, 256);
        let actual = lookup(x, y);
        for channel in 0..2 {
            assert!(
                (coarse[channel] - refined[channel]).abs() < 1e-6,
                "unresolved hemisphere quadrature ({x},{y}) {coarse:?} -> {refined:?}"
            );
            // Declared absolute A/B tolerance for the 1024-point production
            // estimator plus fp16; separate from the oracle refinement check.
            assert!(
                (f64::from(actual[channel]) - refined[channel]).abs() < 0.006,
                "DFG ({x},{y}) actual={actual:?}, independent={refined:?}"
            );
        }
    }
}

#[test]
fn independent_hemisphere_integral_has_analytic_smith_limits() {
    for (no_v, expected) in [(1.0, 1.0 - 2.0_f64.ln()), (0.0, 1.0)] {
        let actual = dfg_reference(no_v, 1.0, 64);
        assert!((actual[0] + actual[1] - expected).abs() < 1e-12);
    }
}

#[test]
fn gpu_reflection_constant_all_faces_mips_and_independent_dfg() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("reflection correctness requires a GPU");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("reflection correctness device");
        let mut frame = [0.0_f32; 32];
        frame[19] = 2.0;
        frame[21] = 1.0;
        frame[28..32].copy_from_slice(&crate::atmosphere::model::sky_mapping(2.0));
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: frame.map(f32::to_le_bytes).as_flattened(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sky = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("constant encoded unit-Sun sky"),
            size: wgpu::Extent3d {
                width: 192,
                height: 108,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes: Vec<_> = (0..192 * 108)
            .flat_map(|_| [0x6400_u16, 0x6000, 0x5c00, 0])
            .flat_map(u16::to_le_bytes)
            .collect();
        queue.write_texture(
            sky.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(192 * 8),
                rows_per_image: Some(108),
            },
            sky.size(),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let prepared = ReflectionPrepared::new(
            &device,
            &uniform,
            &sky.create_view(&wgpu::TextureViewDescriptor::default()),
            &sampler,
        );
        assert_eq!(prepared.cube.width(), CUBE_SIZE);
        assert_eq!(prepared.cube.depth_or_array_layers(), 6);
        assert_eq!(prepared.cube.mip_level_count(), MIP_COUNT);
        assert_eq!([prepared.dfg.width(), prepared.dfg.height()], [DFG_SIZE; 2]);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for stage in 0..=MIP_COUNT as usize {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            prepared
                .encode(stage, &mut pass)
                .expect("prepared reflection stage");
        }
        queue.submit([encoder.finish()]);
        let reader = reader(&device);
        for mip in 0..MIP_COUNT {
            let texels = read_texture(&device, &queue, &reader, &prepared.cube, mip);
            assert_eq!(texels.len(), (6 * (CUBE_SIZE >> mip).pow(2)) as usize);
            for (index, texel) in texels.iter().enumerate() {
                assert!(texel.iter().all(|value| value.is_finite()));
                for (actual, expected) in texel[..3].iter().zip([1024.0, 512.0, 256.0]) {
                    assert!(
                        (actual - expected).abs() <= expected / 1024.0,
                        "constant RGB mip={mip}, texel={index}: {texel:?}"
                    );
                }
            }
        }
        check_dfg(&read_texture(&device, &queue, &reader, &prepared.dfg, 0));
        check_consumer(&device, &queue, &uniform, &prepared);
        check_analytic_cube(&device, &queue, &uniform, &sky, &prepared);
    });
}
