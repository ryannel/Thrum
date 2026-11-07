//! The shipping PCSS consumer against independent finite-disk/plane geometry.
use std::{sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

const SIDE: u32 = 256;
const SAMPLES: usize = 33;
const SCALE: f32 = 0.1;

const OBSERVER: &str = r"
@group(2) @binding(6) var<storage,read_write> result: array<f32>;
@compute @workgroup_size(64)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=66u { return; }
    let cascade=id.x/33u;
    let u=0.4+0.2*f32(id.x%33u)/32.0;
    let slope=shadows.penumbra.z;
    let x=2.0*u-1.0;
    let receiver=ShadowReceiver(vec3(x,0.0,0.25+slope*x),
        vec3(1.0,0.0,slope),vec3(0.0,1.0,0.0),normalize(vec3(-slope,0.0,1.0)));
    result[id.x]=cascade_visibility(cascade,receiver);
}
";

fn oracle(u: f64, slope: f64, gap: f64) -> f64 {
    let mut visible = 0;
    for radius in 0..64 {
        let r = ((f64::from(radius) + 0.5) / 64.0).sqrt();
        for azimuth in 0..128 {
            let x = r * (std::f64::consts::TAU * (f64::from(azimuth) + 0.5) / 128.0).cos();
            let separation = gap / (1.0 - 2.0 * slope * f64::from(SCALE) * x);
            visible += usize::from(u + f64::from(SCALE) * separation * x < 0.5);
        }
    }
    visible as f64 / 8192.0
}

fn pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let source = format!(
        "{}\n{}",
        concat!(
            include_str!("../../../atmosphere/storage.wgsl"),
            "\n",
            include_str!("../../../atmosphere/brdf.wgsl"),
            "\n",
            include_str!("../../../atmosphere/irradiance.wgsl"),
            "\n",
            include_str!("../../../synthetic.wgsl"),
            "\n",
            include_str!("../../../synthetic_plain.wgsl")
        ),
        OBSERVER
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shipping finite Sun shadow observer"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("observe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn observe(device: &wgpu::Device, queue: &wgpu::Queue, slope: f32) -> Vec<f32> {
    let pipeline = pipeline(device);
    let mut samples = [0_f32; 44];
    let identity = glam::Mat4::IDENTITY.to_cols_array();
    samples[..16].copy_from_slice(&identity);
    samples[16..32].copy_from_slice(&identity);
    samples[36..38].fill(0.000_03);
    samples[40..44].copy_from_slice(&[SCALE, SCALE, slope, 0.0]);
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: samples.map(f32::to_le_bytes).as_flattened(),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("two finite-disk test blocker distances"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 2,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, gap) in [0.05, 0.4].into_iter().enumerate() {
        let bytes: Vec<_> = (0..SIDE * SIDE)
            .flat_map(|pixel| {
                let x = 2.0 * ((pixel % SIDE) as f32 + 0.5) / SIDE as f32 - 1.0;
                let receiver = 0.25 + slope * x;
                let depth = receiver + if pixel % SIDE >= SIDE / 2 { gap } else { 0.0 };
                depth.clamp(0.0, 1.0).to_le_bytes()
            })
            .collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &depth,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: u32::try_from(layer).expect("two layers"),
                },
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIDE * 4),
                rows_per_image: Some(SIDE),
            },
            wgpu::Extent3d {
                width: SIDE,
                height: SIDE,
                depth_or_array_layers: 1,
            },
        );
    }
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        compare: Some(wgpu::CompareFunction::GreaterEqual),
        ..Default::default()
    });
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let shadow = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let light = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &vec![0; super::super::super::LIGHT_BYTES],
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let light_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: light.as_entire_binding(),
        }],
    });
    let size = (SAMPLES * 2 * 4) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let observation = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(2),
        entries: &[wgpu::BindGroupEntry {
            binding: 6,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &shadow, &[]);
        pass.set_bind_group(1, &light_group, &[]);
        pass.set_bind_group(2, &observation, &[]);
        pass.dispatch_workgroups(2, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("shadow observation receiver");
    });
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(20)),
        })
        .expect("shadow observation completes");
    receive.recv().unwrap().unwrap();
    readback
        .slice(..)
        .get_mapped_range()
        .unwrap()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect()
}

#[test]
fn finite_sun_shadow_width_tracks_blocker_distance_and_tilted_receivers() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for shadow penumbra test");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        for slope in [0.0, 0.4] {
            let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
            let observed = observe(&device, &queue, slope);
            let error = scope.pop().await;
            assert!(error.is_none(), "{error:?}");
            for (cascade, gap) in [0.05, 0.4].into_iter().enumerate() {
                let row = &observed[cascade * SAMPLES..(cascade + 1) * SAMPLES];
                for (i, actual) in row.iter().enumerate() {
                    let u = 0.4 + 0.2 * i as f64 / 32.0;
                    let expected = oracle(u, f64::from(slope), gap);
                    assert!(
                        (f64::from(*actual) - expected).abs() <= 0.12,
                        "slope{slope} gap{gap} u{u}: GPU{actual} reference{expected}"
                    );
                }
                assert!(row.windows(2).all(|v| v[0] >= v[1] - 0.02));
            }
        }
    });
}
