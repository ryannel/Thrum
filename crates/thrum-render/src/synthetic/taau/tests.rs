use super::super::*;
use wgpu::util::DeviceExt;

fn depth_probe_source() -> String {
    format!(
        "{}\n{}",
        include_str!("../taau.wgsl"),
        include_str!("depth_probe.wgsl")
    )
}

#[test]
fn taau_shaders_validate() {
    for source in [
        include_str!("../taau.wgsl").to_owned(),
        include_str!("../reference.wgsl").to_owned(),
        depth_probe_source(),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(&source).expect("WGSL");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("valid temporal shader");
    }
}

fn depth_probe_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production TAAU depth helper probe"),
        source: wgpu::ShaderSource::Wgsl(depth_probe_source().into()),
    });
    let mut entries: Vec<_> = [0, 5]
        .into_iter()
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        })
        .collect();
    entries.extend(
        [12, 13]
            .into_iter()
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: binding == 13,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }),
    );
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bindings)],
        immediate_size: 0,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&layout),
        module: &shader,
        entry_point: Some("probe_depth"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn depth_probe_texture(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 2,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

#[test]
fn taau_depth_reprojection_rejects_mixed_surfaces_on_metal() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let pipeline = depth_probe_pipeline(&device);
        // Independent scene model: reciprocal view depth is the affine plane
        // 0.031*x + 0.019*y + 0.12. Evaluate the requested ray directly.
        let expected = 1.0 / (0.031 * 0.31 + 0.019 * 0.73 + 0.12);
        let cases: [[f32; 4]; 4] = [
            [expected, 0.31, 0.73, 0.0],
            [12.0, 0.5, 0.5, 0.0],
            [0.0, 0.5, 0.5, 1.0],
            [12.0, 0.5, 0.5, 0.0],
        ];
        let depths: [f32; 16] = [
            1.0 / 0.12,
            1.0 / (0.12 + 0.031),
            1.0 / (0.12 + 0.019),
            1.0 / (0.12 + 0.031 + 0.019),
            4.0,
            4.0,
            12.0,
            12.0, // Foreground/background discontinuity.
            0.0,
            0.0,
            0.0,
            0.0, // Sky.
            0.0,
            0.0,
            12.0,
            12.0, // Sky/geometry boundary.
        ];
        let colour = depth_probe_texture(&device, wgpu::TextureFormat::Rgba16Float);
        let depth = depth_probe_texture(&device, wgpu::TextureFormat::R32Float);
        queue.write_texture(
            depth.as_image_copy(),
            &depths
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(8),
            },
            depth.size(),
        );
        let inputs = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &cases
                .iter()
                .flatten()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>(),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 128,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 128,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&colour_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: inputs.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 128);
        let (send, receive) = std::sync::mpsc::channel();
        encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
            send.send(result).unwrap();
        });
        queue.submit([encoder.finish()]);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receive.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let values: Vec<[f32; 4]> = mapped
            .as_chunks::<16>()
            .0
            .iter()
            .map(|row| {
                std::array::from_fn(|i| {
                    f32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap())
                })
            })
            .collect();
        eprintln!(
            "depth probe adapter={:?}; samples={values:?}",
            adapter.get_info()
        );
        assert!((values[0][2] - expected).abs() < 0.00002);
        assert_eq!(
            values
                .iter()
                .step_by(2)
                .map(|row| row[3].to_bits())
                .collect::<Vec<_>>(),
            [1.0_f32, 0.0, 1.0, 0.0].map(f32::to_bits),
            "accept the projected plane and sky; reject mixed surfaces"
        );
        assert_eq!(
            values[3][0].to_bits(),
            1.0_f32.to_bits(),
            "old range control accepts stale foreground"
        );
    });
}

#[test]
fn taau_prepares_and_resets_on_real_metal() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let mut scene = SyntheticScene::grid(4).unwrap();
        scene.view = scene
            .view
            .with_jitter_sequence(0, [64, 64])
            .with_reconstruction(true);
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .unwrap();
        let mut targets: Vec<_> = (0..2)
            .map(|_| OffscreenTarget::new(&device, &prepared, 64, 64).unwrap())
            .collect();
        let (first, second) = targets.split_at_mut(1);
        second[0].share_reconstruction_history(&prepared, &first[0]);
        for i in 0..4 {
            let target = &mut targets[i % 2];
            scene.view = scene.view.with_history_reset(i == 3);
            prepared
                .update_frame(
                    &queue,
                    target,
                    &scene,
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                    1.0,
                )
                .unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared.encode(&mut encoder, 41, target, None).unwrap();
            queue.submit([encoder.finish()]);
            prepared.commit_frame();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        }
        assert!(prepared.history_identity()[1] > 0);
    });
}
