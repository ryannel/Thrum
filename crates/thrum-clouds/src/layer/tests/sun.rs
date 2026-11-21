//! Cumulative Sun paths must preserve plateaux, slab slopes and Beer bounds.

use thrum_render::binding;

#[test]
fn cumulative_depth_stays_monotone_without_ringing_or_negative_extinction() {
    let Some(thrum_test_support::Opened { device, queue, .. }) = thrum_test_support::open(
        "cloud cumulative interpolation",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let source = format!(
        "{}\n{}",
        include_str!("../sun.wgsl"),
        r"
        struct Constants {heights:vec4<f32>}
        const cloud=Constants(vec4<f32>(0.0,1.0,0.0,0.0));
        // The other Sun-path helpers need this declaration when parsed, even
        // though this entry point exercises only the cubic interpolation.
        struct Mapping {path_border:vec4<f32>}
        const mapping=Mapping(vec4<f32>(0.0,1.0,0.0,0.0));
        @group(0) @binding(0) var<storage,read_write> output:array<vec4<f32>>;
        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) id:vec3<u32>) {
            let curves=array<vec4<f32>,5>(vec4(0.0,1.0,2.0,3.0),vec4(0.0,0.0,0.0,3.0),vec4(0.0,0.1,8.0,9.0),vec4(0.0,3.0,3.0,3.0),vec4(0.0,0.0,3.0,3.0));
            let row=id.x/64u;let t=f32(id.x%64u)/63.0;
            output[id.x]=vec4(cloud_path_cubic(curves[row],t),t,0.0,0.0);
        }
    "
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("cloud cumulative interpolation"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[binding::storage(0, wgpu::ShaderStages::COMPUTE, false, 0)],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("cloud cumulative interpolation"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bytes = 5 * 64 * 16;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(5, 1, 1);
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(&device, &mut encoder, &output, ..);
    let submission = queue.submit([encoder.finish()]);
    let values: Vec<[f32; 4]> = readback
        .wait(&device, submission)
        .as_chunks::<16>()
        .0
        .iter()
        .map(|chunk| bytemuck::cast::<[u8; 16], [f32; 4]>(*chunk))
        .collect();
    for (row, bounds) in [
        (1.0_f32, 2.0),
        (0.0, 0.0),
        (0.1, 8.0),
        (3.0, 3.0),
        (0.0, 3.0),
    ]
    .into_iter()
    .enumerate()
    {
        let curve = &values[row * 64..(row + 1) * 64];
        for (i, v) in curve.iter().enumerate() {
            assert!(
                v[0].is_finite() && v[0] >= bounds.0 && v[0] <= bounds.1,
                "bounded cumulative depth: {row} {i} {v:?}"
            );
            if i > 0 {
                assert!(v[0] >= curve[i - 1][0], "nonnegative local extinction");
            }
            if row == 0 {
                assert!((v[0] - (1.0 + v[1])).abs() < 2e-6, "homogeneous slab slope");
            }
            if row == 1 {
                assert_eq!(v[0].to_bits(), 0, "clear prefix stays exactly clear");
            }
            if row == 3 {
                assert_eq!(v[0], 3.0, "no cloud creates no new optical depth");
            }
            assert!(
                (0.0..=1.0).contains(&(-v[0]).exp()),
                "passive Beer transport"
            );
        }
        assert_eq!(curve[0][0], bounds.0);
        assert_eq!(curve[63][0], bounds.1);
    }
}
