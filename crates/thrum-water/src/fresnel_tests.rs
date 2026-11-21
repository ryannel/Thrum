//! Independent area integration, rather than a second copy of the shader rule.
use wgpu::util::DeviceExt;

fn dielectric(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    let t = (1.0 - (1.0 - c * c) / 1.333_f64.powi(2)).sqrt();
    let perpendicular = ((c - 1.333 * t) / (c + 1.333 * t)).powi(2);
    let parallel = ((1.333 * c - t) / (1.333 * c + t)).powi(2);
    perpendicular.midpoint(parallel)
}

fn reference(input: [f32; 4]) -> f64 {
    let [cosine, xx, xz, zz] = input.map(f64::from);
    if xx + zz == 0.0 {
        return dielectric(cosine);
    }
    let vx = (1.0 - cosine * cosine).sqrt();
    let a = xx.sqrt();
    let b = xz / a;
    let c = (zz - b * b).sqrt();
    let mut visible = 0.0;
    let mut reflected = 0.0;
    // Uniform area in a Box-Muller disk is a Gaussian in slope space. The
    // independent 64x128 grid shares no nodes or weights with Gauss-Hermite.
    for radial in 0..64 {
        let radius = (-2.0 * ((f64::from(radial) + 0.5) / 64.0).ln()).sqrt();
        for angular in 0..128 {
            let phi = std::f64::consts::TAU * (f64::from(angular) + 0.5) / 128.0;
            let x = radius * phi.cos();
            let z = radius * phi.sin();
            let sx = a * x;
            let sz = b * x + c * z;
            let ny = (1.0 + sx * sx + sz * sz).sqrt().recip();
            let nv = (cosine - vx * sx) * ny;
            let weight = (cosine - vx * sx).max(0.0);
            visible += weight;
            reflected += weight * dielectric(nv);
        }
    }
    reflected / visible
}

#[test]
fn unresolved_grazing_slopes_do_not_become_a_flat_perfect_mirror() {
    let Some(thrum_test_support::Opened { device, queue, .. }) = thrum_test_support::open(
        "water Fresnel",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let inputs = [
        [0.3_f32, 0.0, 0.0, 0.0],
        [0.01, 0.01, 0.0, 0.01],
        [0.03, 0.02, 0.01, 0.03],
        [0.1, 0.04, -0.01, 0.02],
    ];
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Fresnel inputs"),
        contents: bytemuck::cast_slice(&inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Fresnel results"),
        size: 32,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let source = format!(
        "{}\n{}",
        include_str!("fresnel.wgsl"),
        r"
@group(0) @binding(0) var<storage,read> inputs:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> results:array<vec2<f32>>;
@compute @workgroup_size(4) fn probe(@builtin(global_invocation_id) id:vec3<u32>) {
    let p=inputs[id.x];let v=vec3(sqrt(1.0-p.x*p.x),p.x,0.0);
    results[id.x]=vec2(water_fresnel(p.x),water_mean_fresnel(vec2(0.0),p.yzw,v));
}"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fresnel probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("Fresnel probe"),
        layout: None,
        module: &module,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Fresnel probe"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
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
    let readback =
        thrum_test_support::readback::Readback::buffer(&device, &mut encoder, &output, 0..32);
    let submission = queue.submit([encoder.finish()]);
    let mapped = readback.wait(&device, submission);
    let values: &[f32] = bytemuck::cast_slice(&mapped);
    for (index, input) in inputs.into_iter().enumerate() {
        let flat = f64::from(values[index * 2]);
        let mean = f64::from(values[index * 2 + 1]);
        let expected = reference(input);
        assert!((0.0..=1.0).contains(&mean));
        if index == 0 {
            assert!((mean - expected).abs() < 1e-6);
        } else {
            assert!(
                (mean - expected).abs() < 0.08,
                "{index}: {mean} vs {expected}"
            );
            assert!((mean - expected).abs() < 0.5 * (flat - expected).abs());
        }
    }
}
