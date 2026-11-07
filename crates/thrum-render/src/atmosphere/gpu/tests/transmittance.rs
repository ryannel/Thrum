//! Actual production transmittance lookup versus the separate CPU direct-light
//! quadrature, plus physical endpoint coverage of both LUT mappings.

use super::super::{EXTENTS, LUT_SHADER};
use crate::atmosphere::model;
use std::{sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

const RADIUS: f64 = 6_371_000.0;
const TOP: f64 = 100_000.0;
const HEIGHTS: [f32; 4] = [0.0, 0.1, 2.0, 6.15];
const ELEVATIONS: [f32; 5] = [0.0, 0.5, 1.0, 2.0, 5.0];
const DIRECT_QUERIES: usize = HEIGHTS.len() * ELEVATIONS.len();
const LOOKUP: &str = r"
@group(0) @binding(6) var<storage,read> queries: array<vec4<f32>>;
@group(0) @binding(7) var<storage,read_write> results: array<vec4<f32>>;
@compute @workgroup_size(64)
fn check_transmittance(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=arrayLength(&queries) { return; }
    let query=queries[id.x];
    let size=textureDimensions(transmittance);
    var params=query.xy;
    if query.w>0.0 { params=trans_params(query.xy); }
    let uv=trans_uv(params.x,params.y,size);
    let sun=vec3(sqrt(max(0.0,1.0-params.y*params.y)),params.y,0.0);
    results[id.x*3u]=vec4(sun_transmittance(vec3(0.0,params.x,0.0),sun),1.0);
    results[id.x*3u+1u]=vec4(uv,params);
    results[id.x*3u+2u]=vec4(trans_params(uv),0.0,0.0);
}
";

fn queries([width, height]: [u32; 2]) -> Vec<[f32; 4]> {
    let mut queries: Vec<_> = HEIGHTS
        .into_iter()
        .flat_map(|h| ELEVATIONS.map(|e| [h, e.to_radians().sin(), e, 0.0]))
        .collect();
    // All boundary texel centres, including the first and last physical
    // altitude rows, nearest vertical-up ray and nearest tangent ray. Interior
    // points ensure the producer and inverse still agree throughout the domain.
    for row in 0..height {
        for column in 0..width {
            if row == 0
                || row == height - 1
                || column == 0
                || column == width - 1
                || (row % 7 == 0 && column % 17 == 0)
            {
                queries.push([
                    (column as f32 + 0.5) / width as f32,
                    (row as f32 + 0.5) / height as f32,
                    0.0,
                    1.0,
                ]);
            }
        }
    }
    queries
}

fn pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    entry: &str,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: None,
        module: shader,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

fn observe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader: &wgpu::ShaderModule,
    size: [u32; 2],
    queries: &[[f32; 4]],
) -> Vec<[f32; 12]> {
    let [width, height] = size;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("production transmittance without unrelated sky work"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let query_bytes: Vec<_> = queries
        .iter()
        .flatten()
        .flat_map(|q| q.to_le_bytes())
        .collect();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("near-ground Sun and producer coordinates"),
        contents: &query_bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = queries.len() as u64 * 48;
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
    let producer = pipeline(device, shader, "transmittance_main");
    let lookup = pipeline(device, shader, "check_transmittance");
    let producer_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &producer.get_bind_group_layout(0),
        entries: &[entry(3, wgpu::BindingResource::TextureView(&view))],
    });
    let lookup_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &lookup.get_bind_group_layout(0),
        entries: &[
            entry(1, wgpu::BindingResource::TextureView(&view)),
            entry(4, wgpu::BindingResource::Sampler(&sampler)),
            entry(6, input.as_entire_binding()),
            entry(7, output.as_entire_binding()),
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&producer);
        pass.set_bind_group(0, &producer_group, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&lookup);
        pass.set_bind_group(0, &lookup_group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(queries.len())
                .expect("small transmittance query count")
                .div_ceil(64),
            1,
            1,
        );
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("transmittance receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(20)),
        })
        .expect("transmittance GPU completion");
    receive
        .recv()
        .expect("transmittance callback")
        .expect("transmittance mapped");
    readback
        .slice(..)
        .get_mapped_range()
        .expect("transmittance records")
        .as_chunks::<48>()
        .0
        .iter()
        .map(|record| {
            std::array::from_fn(|word| {
                f32::from_le_bytes(
                    record[word * 4..word * 4 + 4]
                        .try_into()
                        .expect("f32 record"),
                )
            })
        })
        .collect()
}

fn check_direct(queries: &[[f32; 4]], records: &[[f32; 12]]) {
    // Clamp below the first physical row (6.151m) before computing BOTH UV
    // coordinates. Relative to the true-height CPU path this deliberately
    // bounded approximation reaches 0.382/0.585/1.209% in RGB at h=0, Sun=0deg.
    // The bounds include fp16, hardware subtexel weights and f32 arithmetic.
    let budget = [0.006_f32, 0.009, 0.016];
    for (query, record) in queries.iter().zip(records).take(DIRECT_QUERIES) {
        let [height, mu, elevation, _] = *query;
        let expected = model::sun_transmittance(height, mu);
        for (channel, truth) in expected.into_iter().enumerate() {
            let relative = (record[channel] / truth - 1.0).abs();
            assert!(
                relative <= budget[channel],
                "Sun lookup h={height}m e={elevation}deg channel={channel}: GPU={} CPU={truth} relative={relative}",
                record[channel]
            );
        }
    }
}

fn check_inverse(size: [u32; 2], queries: &[[f32; 4]], records: &[[f32; 12]]) {
    let rho = (TOP * (2.0 * RADIUS + TOP)).sqrt() * 0.5 / f64::from(size[1]);
    let floor = rho * rho / ((RADIUS * RADIUS + rho * rho).sqrt() + RADIUS);
    for (query, record) in queries.iter().zip(records).take(DIRECT_QUERIES) {
        let [height, mu, _, _] = *query;
        assert!(
            (f64::from(record[8]) - f64::from(height).max(floor)).abs() < 1e-4,
            "height inverse must match the sampled physical row: {record:?}"
        );
        assert!(
            (record[5] - 0.5 / size[1] as f32).abs() < 1e-7,
            "low-altitude samples use the first texel centre: {record:?}"
        );
        assert!((record[9] - mu).abs() < 2e-6, "mu inverse: {record:?}");
    }
}

fn check_producer(queries: &[[f32; 4]], records: &[[f32; 12]]) {
    let cap = (TOP * (2.0 * RADIUS + TOP)).sqrt();
    for (query, record) in queries.iter().zip(records).skip(DIRECT_QUERIES) {
        let [u, v, _, _] = query.map(f64::from);
        let rho = cap * v;
        let h = rho * rho / ((RADIUS * RADIUS + rho * rho).sqrt() + RADIUS);
        let d = TOP - h + u * (rho + cap - (TOP - h));
        let mu = ((TOP - h) * (2.0 * RADIUS + TOP + h) - d * d) / (2.0 * (RADIUS + h) * d);
        assert!(
            (f64::from(record[6]) - h).abs() < 0.04,
            "producer height: {query:?} {record:?}"
        );
        assert!(
            (f64::from(record[7]) - mu).abs() < 3e-6,
            "producer mu: {query:?} {record:?}"
        );
        for axis in 0..2 {
            assert!(
                (record[axis + 4] - query[axis]).abs() < 3e-6,
                "producer roundtrip: {query:?} {record:?}"
            );
        }
    }
}

#[test]
fn near_ground_sun_lookup_and_producer_endpoints_agree_with_cpu_authority() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("transmittance adapter");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("transmittance device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("production transmittance and test-only observer"),
            source: wgpu::ShaderSource::Wgsl(format!("{LUT_SHADER}\n{LOOKUP}").into()),
        });
        // The smaller fixture catches accidental hardcoded production extents;
        // physical lookup tolerances apply only to the production profile.
        for size in [EXTENTS[0], [128, 32]] {
            let queries = queries(size);
            let records = observe(&device, &queue, &shader, size, &queries);
            assert!(records.iter().flatten().all(|value| value.is_finite()));
            check_inverse(size, &queries, &records);
            check_producer(&queries, &records);
            if size == EXTENTS[0] {
                check_direct(&queries, &records);
            }
        }
        assert!(
            scope.pop().await.is_none(),
            "transmittance pipeline validates"
        );
    });
}
