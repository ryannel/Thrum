//! The production material bind group is readable by compute, without a copy table.
use super::*;

#[test]
fn compute_reads_the_forward_material_table() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let caps = DeviceCapabilities::capture(&adapter, &device, 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.materials[0].base_rgba = [0.125, 0.25, 0.5, 1.0];
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .await
        .expect("prepare");
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute material read"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mapped = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material result"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let result_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let result = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &result_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label:None,source:wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var<storage,read> materials:array<vec4<f32>>; @group(1) @binding(0) var<storage,read_write> result:vec4<f32>; @compute @workgroup_size(1) fn main(){ result=materials[0]; }".into()) });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Some(&prepared.surface.material_layout),
                Some(&result_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &prepared.materials, &[]);
            pass.set_bind_group(1, &result, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &mapped, 0, 16);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        mapped
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).expect("map channel")
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(20)),
            })
            .expect("wait");
        rx.recv().expect("map callback").expect("map success");
        let view = mapped.slice(..).get_mapped_range().expect("mapped bytes");
        let values: Vec<f32> = view
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("f32")))
            .collect();
        assert_eq!(values, [0.125, 0.25, 0.5, 1.0]);
    });
}
