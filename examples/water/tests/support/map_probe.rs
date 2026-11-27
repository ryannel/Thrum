//! Submit a query through the water map shader and read its replies.
use thrum_water::map_gpu::MapGpu;
use thrum_water_model::Map;
use wgpu::util::DeviceExt;

pub(crate) fn sample(
    gpu: &thrum_test_support::Opened,
    cpu: &Map,
    points: &[[f32; 2]],
    proof: &str,
    source: impl FnOnce(String) -> String,
) -> Vec<[f32; 4]> {
    let (device, queue) = (&gpu.device, &gpu.queue);
    let map = MapGpu::new(device, queue, cpu).expect("river map products");
    let view = map.view();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("river probe points"),
        contents: bytemuck::cast_slice(points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (size_of::<[f32; 4]>() * points.len()) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("river probe answers"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("river map probe"),
        source: wgpu::ShaderSource::Wgsl((source(map.source(0)) + proof).into()),
    });
    let pipeline = thrum_render::runtime::DeviceSite::untracked(device.clone()).compute(
        &wgpu::ComputePipelineDescriptor {
            label: Some("river map probe"),
            layout: None,
            module: &shader,
            entry_point: Some("probe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        },
    );
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: input.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: output.as_entire_binding(),
        },
    ];
    entries.extend(map.entries(&view));
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("river map bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(points.len().div_ceil(32)).expect("points"),
            1,
            1,
        );
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, 0..size);
    let submission = queue.submit([encoder.finish()]);
    bytemuck::cast_slice(&readback.wait(device, submission)).to_vec()
}
