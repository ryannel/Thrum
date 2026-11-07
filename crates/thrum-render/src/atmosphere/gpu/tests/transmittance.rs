//! Actual production transmittance lookup versus the separate CPU direct-light
//! quadrature, plus physical endpoint coverage of both LUT mappings.

use super::super::{EARTH, EXTENTS, LUT_PLANES, LUT_SHADER};
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
    let size=plane_size(textureDimensions(transmittance),TRANS_PLANES);
    var params=query.xy;
    if query.w>0.0 { params=trans_params(query.xy); }
    let uv=trans_uv(params.x,params.y,size);
    let sun=vec3(sqrt(max(0.0,1.0-params.y*params.y)),params.y,0.0);
    results[id.x*3u]=vec4(sample_sun_transmittance(params.x,sun.y,transmittance,lut_sampler),1.0);
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
    frame: Option<[f32; 32]>,
) -> Vec<[f32; 12]> {
    let [width, height] = size;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("production transmittance without unrelated sky work"),
        size: wgpu::Extent3d {
            width,
            height: height * LUT_PLANES[0],
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
    let lookup = pipeline(
        device,
        shader,
        if frame.is_some() {
            "check_surface"
        } else {
            "check_transmittance"
        },
    );
    let producer_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &producer.get_bind_group_layout(0),
        entries: &[entry(3, wgpu::BindingResource::TextureView(&view))],
    });
    let uniform = frame.map(|data| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("translated camera and unattenuated Sun"),
            contents: data.map(f32::to_le_bytes).as_flattened(),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    });
    let surface_group = uniform.as_ref().map(|uniform| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("real surface Sun bindings"),
            layout: &lookup.get_bind_group_layout(1),
            entries: &[
                entry(6, wgpu::BindingResource::Sampler(&sampler)),
                entry(7, uniform.as_entire_binding()),
                entry(11, wgpu::BindingResource::TextureView(&view)),
            ],
        })
    });
    let mut lookup_entries = vec![
        entry(6, input.as_entire_binding()),
        entry(7, output.as_entire_binding()),
    ];
    if frame.is_none() {
        lookup_entries.extend([
            entry(1, wgpu::BindingResource::TextureView(&view)),
            entry(4, wgpu::BindingResource::Sampler(&sampler)),
        ]);
    }
    let lookup_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &lookup.get_bind_group_layout(0),
        entries: &lookup_entries,
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
        if let Some(group) = &surface_group {
            pass.set_bind_group(1, group, &[]);
        }
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
    // At a zero projected channel, apply the same bound to a 1e-3 denominator.
    let budget = [0.006_f32, 0.009, 0.016];
    for (query, record) in queries.iter().zip(records).take(DIRECT_QUERIES) {
        let [height, mu, elevation, _] = *query;
        let expected = model::sun_transmittance(height, mu);
        for (channel, truth) in expected.into_iter().enumerate() {
            let relative = (record[channel] - truth).abs() / truth.max(1e-3);
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
            source: wgpu::ShaderSource::Wgsl(
                EARTH.shader(&format!("{LUT_SHADER}\n{LOOKUP}")).into(),
            ),
        });
        // The smaller fixture catches accidental hardcoded production extents;
        // physical lookup tolerances apply only to the production profile.
        for size in [EXTENTS[0], [128, 32]] {
            let queries = queries(size);
            let records = observe(&device, &queue, &shader, size, &queries, None);
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

const SURFACE_LOOKUP: &str = concat!(
    // The unused aerial body still resolves against its actual view dependency.
    "struct ViewAux { viewport_inv_viewport: vec4<f32> }\n@group(2) @binding(0) var<uniform> view_aux: ViewAux;\n",
    include_str!("../../../synthetic_aerial.wgsl"),
    "\n",
    include_str!("../../../synthetic_sun.wgsl"),
    r"
@compute @workgroup_size(64)
fn check_surface(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=arrayLength(&queries) { return; }
    results[id.x*3u]=vec4(sun_irradiance(queries[id.x].xyz),1.0);
    results[id.x*3u+1u]=vec4(0.0);
    results[id.x*3u+2u]=vec4(0.0);
}
"
);

#[expect(
    clippy::cast_possible_truncation,
    reason = "the independent spherical receiver geometry uses f64; bounded height and cosine feed the existing f32 CPU quadrature"
)]
fn reference_receiver_transmission(point: [f32; 3], sun: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = point.map(f64::from);
    let radial = (x * x + (RADIUS + y).powi(2) + z * z).sqrt();
    let sun_projection = x * f64::from(sun[0]) + (RADIUS + y) * f64::from(sun[1]);
    let shell = RADIUS + TOP;
    if radial > shell {
        let closest_radius_sq = radial.powi(2) - sun_projection.powi(2);
        if sun_projection >= 0.0 || closest_radius_sq >= shell.powi(2) {
            return [1.0; 3];
        }
        let shell_mu = -(shell.powi(2) - closest_radius_sq).sqrt() / shell;
        return model::sun_transmittance(TOP as f32, shell_mu as f32);
    }
    model::sun_transmittance((radial - RADIUS) as f32, (sun_projection / radial) as f32)
}

fn check_receiver_case(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader: &wgpu::ShaderModule,
    physical_camera_height: f32,
    points: &[[f32; 3]],
    elevation: f32,
    split_horizon: bool,
) {
    let colours = [0.8, 1.04, 1.12];
    let lux = 117_000.0;
    let angle = elevation.to_radians();
    let sun = [angle.cos(), angle.sin(), 0.0];
    let mut baseline: Option<Vec<[f32; 12]>> = None;
    for eye in [[0.0, 245.0, 16.0], [8132.0, 123.0, -3211.0]] {
        let queries: Vec<_> = points
            .iter()
            .map(|&[x, h, z]| {
                [
                    eye[0] + x,
                    eye[1] + h - physical_camera_height,
                    eye[2] + z,
                    0.0,
                ]
            })
            .collect();
        let mut frame = [0.0_f32; 32];
        frame[16..20].copy_from_slice(&[eye[0], eye[1], eye[2], physical_camera_height]);
        frame[20..24].copy_from_slice(&[sun[0], sun[1], sun[2], lux]);
        frame[24..28].copy_from_slice(&[colours[0], colours[1], colours[2], 0.125]);
        let records = observe(device, queue, shader, EXTENTS[0], &queries, Some(frame));
        for (point, record) in points.iter().zip(&records) {
            let expected = reference_receiver_transmission(*point, sun);
            for channel in 0..3 {
                let actual = record[channel] / (lux * colours[channel]);
                assert!(actual.is_finite() && actual >= 0.0);
                // This receiver test allows the existing LUT interpolation error
                // near the tangent. The separate sea-level probe bounds it tighter.
                let tolerance = 0.05 * expected[channel] + 0.005;
                assert!(
                    (actual - expected[channel]).abs() <= tolerance,
                    "receiver={point:?} e={elevation} channel={channel}: GPU={actual}, CPU={}",
                    expected[channel]
                );
            }
        }
        if split_horizon {
            assert_eq!(records[0][..3], [0.0; 3]);
            assert_eq!(records[2][..3], [0.0; 3]);
            assert!(
                records[3][0] > 100.0,
                "a high receiver remains sunlit after the camera horizon"
            );
            assert!(
                records[5][0] > records[6][0],
                "local horizon includes receiver XZ"
            );
        }
        if let Some(baseline) = &baseline {
            assert_eq!(
                &records, baseline,
                "world translation must preserve the physical frame"
            );
        } else {
            baseline = Some(records);
        }
    }
}

#[test]
fn surface_sun_uses_receiver_height_and_local_horizon_not_camera_transmission() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("surface Sun adapter");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("surface Sun device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("production surface Sun and isolated observer"),
            source: wgpu::ShaderSource::Wgsl(
                EARTH
                    .shader(&format!("{LUT_SHADER}\n{LOOKUP}\n{SURFACE_LOOKUP}"))
                    .into(),
            ),
        });
        let points = [
            [0.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 245.0, 0.0],
            [0.0, 1500.0, 0.0],
            [0.0, 5000.0, 0.0],
            [20_000.0, 245.0, 0.0],
            [-20_000.0, 245.0, 0.0],
        ];
        for (elevation, split_horizon) in [
            (-2.0, false),
            (-0.6, true),
            (0.0, false),
            (1.0, false),
            (30.0, false),
        ] {
            check_receiver_case(
                &device,
                &queue,
                &shader,
                245.0,
                &points,
                elevation,
                split_horizon,
            );
        }
        // At a 100km camera, these receivers are within the 32km aerial range.
        // Cover a planet hit, a Sun ray through the shell, and misses in both directions.
        let upper = [
            [0.0, 100_000.0, 0.0],
            [0.0, 120_000.0, 0.0],
            [0.0, 130_000.0, 0.0],
        ];
        for elevation in [-13.0, -10.07, -1.0, 5.0] {
            check_receiver_case(
                &device, &queue, &shader, 100_000.0, &upper, elevation, false,
            );
        }
        assert!(
            scope.pop().await.is_none(),
            "surface Sun pipeline validates"
        );
    });
}
