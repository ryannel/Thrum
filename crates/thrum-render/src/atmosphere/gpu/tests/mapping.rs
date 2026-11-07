//! Exercise the actual shipping WGSL mapping and sampler, including its two
//! distinct stored tangent limits. The colour fixture cannot hide cross-filtering.

use super::super::super::model;
use std::{f64::consts::PI, sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

const SHADER: &str = concat!(
    concat!(
        include_str!("../../storage.wgsl"),
        "\n",
        concat!(
            include_str!("../../geometry.wgsl"),
            "\n",
            include_str!("../../common.wgsl")
        )
    ),
    r"
@group(0) @binding(1) var colours: texture_2d<f32>;
@group(0) @binding(2) var lookup: sampler;
@group(0) @binding(3) var<storage,read> queries: array<vec4<f32>>;
@group(0) @binding(4) var<storage,read_write> results: array<vec4<f32>>;
@compute @workgroup_size(8)
fn check_mapping(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=arrayLength(&queries) { return; }
    let query=queries[id.x];
    var direction=normalize(query.xyz);
    var distance=-1.0;
    var ground=-1.0;
    if query.w==0.0 {
        let ray=sky_view_ray(query.xy,u32(query.z));
        direction=ray.direction;
        distance=ray.distance;
        ground=f32(ray.terminal_ground);
    }
    var uv=sky_uv(direction,atmosphere.sun_lux.xyz);
    if query.w==2.0 { uv=query.xy; }
    let size=vec2<f32>(textureDimensions(colours));
    let colour=textureSampleLevel(colours,lookup,clamp(uv,0.5/size,1.0-0.5/size),0.0);
    results[id.x*3u]=vec4(uv,direction.y,distance);
    results[id.x*3u+1u]=vec4(colour.rgb,ground);
    results[id.x*3u+2u]=vec4(direction,0.0);
}
"
);

fn queries(height: f32) -> Vec<[f32; 4]> {
    let mut queries: Vec<_> = (0..108)
        .map(|row| [0.5, (row as f32 + 0.5) / 108.0, row as f32, 0.0])
        .collect();
    let horizon = f64::from(model::sky_mapping(height)[1]);
    for elevation in [-PI * 0.5, horizon - 1e-5, horizon + 1e-5, 0.0, PI * 0.5] {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "finite unit directions round to the shipping GPU query ABI"
        )]
        let direction = [elevation.cos() as f32, elevation.sin() as f32, 0.0, 1.0];
        queries.push(direction);
    }
    for azimuth in [-PI - 1e-4, -PI + 1e-4, PI - 1e-4, PI + 1e-4] {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "finite unit directions round to the shipping GPU query ABI"
        )]
        let direction = [
            (0.25_f64.cos() * azimuth.cos()) as f32,
            0.25_f64.sin() as f32,
            (0.25_f64.cos() * azimuth.sin()) as f32,
            1.0,
        ];
        queries.push(direction);
    }
    // Exact tangency selects sky in the inverse. Also sample the stored ground
    // limit directly, so both half-centre hardware footprints are falsified.
    queries.extend([[0.5, 53.5 / 108.0, 0.0, 2.0], [0.5, 54.5 / 108.0, 0.0, 2.0]]);
    queries
}

fn observe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    height: f32,
    sun: [f32; 3],
    queries: &[[f32; 4]],
    texture: &wgpu::Texture,
) -> Vec<[f32; 12]> {
    let mut frame = [0.0_f32; 32];
    frame[19] = height;
    frame[20..23].copy_from_slice(&sun);
    frame[28..32].copy_from_slice(&model::sky_mapping(height));
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: frame.map(f32::to_le_bytes).as_flattened(),
        usage: wgpu::BufferUsages::UNIFORM,
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
    let bytes = u64::try_from(queries.len() * 48).expect("bounded query bytes");
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(queries.len())
                .expect("bounded query count")
                .div_ceil(8),
            1,
            1,
        );
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("mapping receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("mapping GPU completion");
    receive
        .recv()
        .expect("mapping callback")
        .expect("mapping readback");
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapped records");
    mapped
        .as_chunks::<48>()
        .0
        .iter()
        .map(|record| {
            std::array::from_fn(|word| {
                f32::from_le_bytes(
                    record[word * 4..word * 4 + 4]
                        .try_into()
                        .expect("f32 field"),
                )
            })
        })
        .collect()
}

#[test]
fn gpu_horizon_rows_roundtrip_endpoints_azimuth_and_never_cross_filter_halves() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter for sky mapping");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("mapping device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shipping sky mapping falsifier"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("check_mapping"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("disjoint ground red / sky blue"),
            size: wgpu::Extent3d {
                width: 4,
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
        let data: Vec<_> = (0..108)
            .flat_map(|row| {
                (0..4).flat_map(move |_| {
                    if row < 54 {
                        [0x3c00_u16, 0, 0, 0x3c00]
                    } else {
                        [0, 0, 0x3c00, 0x3c00]
                    }
                })
            })
            .flat_map(u16::to_le_bytes)
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(32),
                rows_per_image: Some(108),
            },
            texture.size(),
        );
        for height in [0.0, 2.0, 100_000.0] {
            for sun in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.6, 0.8]] {
                let queries = queries(height);
                let records = observe(&device, &queue, &pipeline, height, sun, &queries, &texture);
                check_rows(height, sun, &queries, &records);
            }
        }
        assert!(scope.pop().await.is_none());
    });
}

fn check_rows(height: f32, sun: [f32; 3], queries: &[[f32; 4]], records: &[[f32; 12]]) {
    let mapping = model::sky_mapping(height);
    for (index, (&query, record)) in queries.iter().zip(records).enumerate() {
        assert!(
            record.iter().all(|value| value.is_finite()),
            "h={height}, query={index}: {record:?}"
        );
        let expected_sky = if query[3].to_bits() == 2.0_f32.to_bits() {
            query[1] > 0.5
        } else {
            record[2] >= mapping[0]
        };
        let expected = if expected_sky {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        for (value, expected) in record[4..7].iter().zip(expected) {
            assert!(
                (value - expected).abs() < 1e-6,
                "cross-half filter h={height}, query={index}: {record:?}"
            );
        }
        if query[3] == 0.0 {
            let row = index;
            assert_eq!(record[7], if row < 54 { 1.0 } else { 0.0 });
            let expected_row = if row == 53 { 54 } else { row };
            assert!(
                (f64::from(record[1]) * 108.0 - 0.5 - expected_row as f64).abs() < 0.002,
                "row roundtrip h={height}, row={row}: {record:?}"
            );
            let q = if row < 54 {
                (53 - row) as f64 / 53.0
            } else {
                (row - 54) as f64 / 53.0
            };
            let r = 6_371_000.0;
            let h = f64::from(height);
            let mu_h = -(h * (2.0 * r + h)).sqrt() / (r + h);
            let eh = mu_h.asin();
            let angle = if row < 54 {
                eh - (PI * 0.5 + eh) * q * q
            } else {
                eh + (PI * 0.5 - eh) * q * q
            };
            assert!(
                (f64::from(record[2]) - angle.sin()).abs() < 2e-6,
                "independent angular-square geometry"
            );
            if row == 53 || row == 54 {
                assert_eq!(
                    record[2], mapping[0],
                    "identical tangent mu, distinct path limits"
                );
                let tangent = (h * (2.0 * r + h)).sqrt();
                let expected_distance = if row == 53 {
                    tangent
                } else {
                    tangent + ((r + 100_000.0).powi(2) - r * r).sqrt()
                };
                assert!(
                    (f64::from(record[3]) - expected_distance).abs()
                        < expected_distance * 2e-6 + 1e-4,
                    "forced tangent path"
                );
            }
        }
        if query[3].to_bits() != 2.0_f32.to_bits() {
            let d = &record[8..11];
            let azimuth = if d[0] * d[0] + d[2] * d[2] <= 1e-10 {
                0.0
            } else {
                let sun_azimuth = if sun[0] * sun[0] + sun[2] * sun[2] <= 1e-10 {
                    0.0
                } else {
                    f64::from(sun[2]).atan2(f64::from(sun[0]))
                };
                let relative = f64::from(d[2]).atan2(f64::from(d[0])) - sun_azimuth;
                relative.sin().atan2(relative.cos())
            };
            let expected = azimuth / (2.0 * PI) + 0.5;
            let delta = (f64::from(record[0]) - expected).abs();
            assert!(
                delta.min((1.0 - delta).abs()) < 2e-6,
                "azimuth wrap/zenith Sun h={height}, query={index}"
            );
        }
    }
    assert_eq!(records[0][2], -1.0);
    assert_eq!(records[107][2], 1.0);
}
