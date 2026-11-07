//! Shared GPU readback for the production Sun visibility referees.
use std::{sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

pub(super) const SAMPLE_FLOATS: usize = super::super::SAMPLE_BYTES / 4;

pub(super) async fn device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("real adapter required for the Sun visibility tests");
    #[cfg(target_os = "macos")]
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("Sun visibility device")
}

pub(super) fn constant_depth_maps(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    side: u32,
    depths: &[f32],
) -> wgpu::Texture {
    let maps = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("analytic constant shadow depths"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: u32::try_from(depths.len()).expect("few test depth layers"),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (index, &depth) in depths.iter().enumerate() {
        let view = maps.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: u32::try_from(index).expect("test layer"),
            array_layer_count: Some(1),
            ..Default::default()
        });
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(depth),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
    }
    queue.submit([encoder.finish()]);
    maps
}

pub(super) fn pipeline(device: &wgpu::Device, observer: &str) -> wgpu::ComputePipeline {
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
            include_str!("../../../synthetic/horizon_taps.wgsl"),
            "\n",
            include_str!("../../../synthetic_plain.wgsl")
        ),
        observer
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

pub(super) fn read_visibility(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    samples: &[f32; SAMPLE_FLOATS],
    maps: &wgpu::TextureView,
    direction: [f32; 4],
    count: u32,
) -> Vec<f32> {
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: samples.map(f32::to_le_bytes).as_flattened(),
        usage: wgpu::BufferUsages::UNIFORM,
    });
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
                resource: wgpu::BindingResource::TextureView(maps),
            },
        ],
    });
    let mut light_bytes = vec![0; super::super::super::LIGHT_BYTES];
    light_bytes[..16].copy_from_slice(direction.map(f32::to_le_bytes).as_flattened());
    let light = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &light_bytes,
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
    let size = u64::from(count) * 4;
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
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &shadow, &[]);
        pass.set_bind_group(1, &light_group, &[]);
        pass.set_bind_group(2, &observation, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
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
