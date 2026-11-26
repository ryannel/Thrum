//! Observe the scene's leased, jittered sample points and the provider's written coefficients.
mod ground_support;
#[expect(
    dead_code,
    reason = "other clouds tests exercise the shared scene helpers"
)]
mod scene_fixture;

use thrum_cloud_model::{Handover, Weather};
use thrum_render::{atmosphere::ViewVolumeMediumTarget, binding, runtime::DeviceSite};
use thrum_test_support::{
    Opened,
    readback::{Mip, Readback},
};

#[test]
fn metal_medium_writes_the_near_share_at_the_current_jittered_points() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    let ground = ground_support::fixture(ground_support::fog(false), false);
    let field = ground_support::field();
    let weather = Weather {
        field: &field,
        surface: Some(&ground.surface),
        ground: Some(&ground.fog),
    };
    let state = ground_support::state(0.0, 1.0);
    let (_host, products, _) = ground_support::render(&gpu, &ground, &state, 25.0);
    let target = &products.medium;
    let values = ground_support::read(&gpu, &target.coefficients, Mip::level(0));
    let points = read_points(&gpu, target);
    assert_eq!(points.len(), values.len());
    let ramp = Handover::new(300.0, 500.0).expect("near/far share");
    let mut covered = [0_usize; 2];
    let mut rejected_missing_weight = false;
    for ([x, y, z, at], [extinction, sun, ambient, _]) in points.into_iter().zip(values) {
        let sigma = weather
            .extinction([x, y, z].map(f64::from), &state, |_| None)
            .value;
        let near = ramp.weights(f64::from(at))[0];
        let expected = sigma * near;
        let matches = |candidate: f64| (f64::from(extinction) - candidate).abs() < 2e-6;
        assert!(
            matches(expected),
            "{at:.2} m, y={y:.2} m: extinction {extinction} != {expected}"
        );
        assert!(sun.is_finite() && ambient.is_finite());
        if expected == 0.0 {
            assert_eq!([sun, ambient], [0.0; 2]);
        }
        if sigma > 0.0 {
            covered[usize::from(at > 300.0)] += 1;
            if (340.0..470.0).contains(&at) {
                rejected_missing_weight |= !matches(sigma);
            }
        }
    }
    assert!(covered.into_iter().all(|count| count > 0));
    assert!(
        rejected_missing_weight,
        "removing the near share must fail in the ramp"
    );
    eprintln!("sample cells with fog before/inside the ramp: {covered:?}");
}

fn read_points(gpu: &Opened, target: &ViewVolumeMediumTarget) -> Vec<[f32; 4]> {
    let device = DeviceSite::untracked(gpu.device.clone());
    let extent = target.coefficients.size();
    let bytes = u64::from(extent.width)
        * u64::from(extent.height)
        * u64::from(extent.depth_or_array_layers)
        * 16;
    let points = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("leased medium sample points"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let abc = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("leased medium grid"),
        entries: &ViewVolumeMediumTarget::layout_entries(),
    });
    let output = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("medium point output"),
        entries: &[binding::storage(0, wgpu::ShaderStages::COMPUTE, false, 0)],
    });
    let groups = [
        target.bind(&device, &abc),
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("medium point readback"),
            layout: &output,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: points.as_entire_binding(),
            }],
        }),
    ];
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("medium point readback"),
        bind_group_layouts: &[Some(&abc), Some(&output)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("medium point readback"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                target.shader_source(),
                r"
                @group(1) @binding(0) var<storage, read_write> points: array<vec4<f32>>;
                @compute @workgroup_size(8, 8)
                fn probe(@builtin(global_invocation_id) cell: vec3<u32>) {
                    let size = textureDimensions(medium_out);
                    if any(cell.xy >= size) { return; }
                    let index = (cell.z * size.y + cell.y) * size.x + cell.x;
                    points[index] = medium_sample_point(cell);
                }
            ",
            )
            .into(),
        ),
    });
    let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("medium point readback"),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &groups[0], &[]);
        pass.set_bind_group(1, &groups[1], &[]);
        pass.dispatch_workgroups(
            extent.width.div_ceil(8),
            extent.height.div_ceil(8),
            extent.depth_or_array_layers,
        );
    }
    let readback = Readback::buffer(&device, &mut encoder, &points, ..);
    let submitted = gpu.queue.submit([encoder.finish()]);
    readback
        .wait(&device, submitted)
        .as_chunks::<16>()
        .0
        .iter()
        .copied()
        .map(bytemuck::cast)
        .collect()
}
