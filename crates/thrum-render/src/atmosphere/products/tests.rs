//! Contribution-side compilation and generation lifecycle on the shipping path.

mod diffuse;
mod filtered;
mod night;
mod observer;
mod reflection;
mod sun;
use crate::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
};
use std::{sync::mpsc, time::Duration};

const QUERY: &str = r"
@group(1) @binding(0) var<storage,read_write> result: array<vec4<f32>>;
@compute @workgroup_size(1)
fn query() {
    result[0]=vec4(sun_irradiance(vec3(0.0)),1.0);
    result[1]=vec4(environment_sky_radiance(vec3(0.0,1.0,0.0)),1.0);
    result[2]=vec4(environment_diffuse(vec3(0.0,1.0,0.0)),1.0);
    let aerial=environment_aerial(vec3(0.0,0.0,-1000.0),vec2(0.5));
    result[3]=vec4(aerial.transmission,1.0);
    result[4]=vec4(aerial.scatter,1.0);
    let reflection=environment_reflection(vec3(0.0,1.0,0.0),0.3,0.6);
    result[5]=vec4(reflection.radiance,1.0);
    result[6]=vec4(reflection.split_sum,environment_storage_scale(),1.0);
}
";

#[test]
fn metal_contribution_compiles_all_lent_operations_and_observes_environment_replacement() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 120);
        let scene = SyntheticScene::grid(1).unwrap();
        let frame = AtmosphereFrame::new([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], 2.0, 1)
            .unwrap()
            .with_simulation_time(10.0)
            .unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0,
        )
        .await
        .unwrap();
        let first = prepared.contribution_frame().environment.unwrap();
        let surface = prepared.scene_surface(&queue);
        let products = surface.environment_products().unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("independent environment contribution"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{QUERY}", products.shader_source).into()),
        });
        let output_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&products.layout), Some(&output_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("independent contribution pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("query"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 112,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 112,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &output_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        });
        let mut target = OffscreenTarget::new(&device, &prepared, 16, 16).unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared.encode(&mut encoder, 120, &target, None).unwrap();
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(
                0,
                products
                    .group(prepared.contribution_frame().exposure_side)
                    .unwrap(),
                &[],
            );
            pass.set_bind_group(1, &group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 112);
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
            send.send(result).unwrap();
        });
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        receive.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let words: Vec<_> = mapped
            .as_chunks::<4>()
            .0
            .iter()
            .map(|v| f32::from_le_bytes(*v))
            .collect();
        assert!(words.iter().all(|x| x.is_finite()));
        for row in [0, 1, 2, 4, 5] {
            assert!(words[row * 4..row * 4 + 3].iter().all(|x| *x > 0.0));
        }
        assert!(words[12..15].iter().all(|x| (0.0..=1.0).contains(x)));
        assert!(words[24] > 0.0 && words[25] >= 0.0 && words[24] + words[25] <= 1.001);
        let alternate = filtered::exposure_two(&prepared, &queue);
        let scale = observer::sample(
            &device,
            &queue,
            &alternate,
            "vec4(environment_storage_scale())",
            &[[0.0; 4]],
            0,
        );
        assert_eq!(
            scale[0][0], 2.0,
            "non-unit exposure fixture reaches the lent layout"
        );
        prepared.commit_frame();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(frame.with_simulation_time(20.0).unwrap()),
                1.0,
            )
            .unwrap();
        let scrubbed = prepared.contribution_frame().environment.unwrap();
        assert_eq!(scrubbed.epoch, first.epoch);
        assert_eq!(scrubbed.simulation_time_seconds, 20.0);
        let changed = AtmosphereFrame::new([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], 2.0, 2).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(changed),
                1.0,
            )
            .unwrap();
        let profile = prepared.contribution_frame().environment.unwrap();
        assert_ne!(profile.epoch, first.epoch);
        let (replacement_device, replacement_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let replacement_capabilities =
            DeviceCapabilities::capture(&adapter, &replacement_device, 121);
        let replacement = PreparedSynthetic::prepare_with_capabilities(
            &replacement_capabilities,
            &replacement_queue,
            &scene,
            FrameLighting::Atmosphere(changed),
            1.0,
        )
        .await
        .unwrap();
        assert_ne!(
            replacement.contribution_frame().environment.unwrap().epoch,
            profile.epoch
        );
    });
}
