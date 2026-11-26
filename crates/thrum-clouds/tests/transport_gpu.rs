//! Production transport kernels against retained Python Beer/slab references.
use wgpu::util::DeviceExt;

const QUERY: &str = r"
@group(0) @binding(0) var<storage,read> inputs:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> outputs:array<vec4<f32>>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let p=inputs[id.x];
    var result=CloudTransport(vec3(0.0),1.0,0.0);
    var distance=0.0;
    while distance<p.z {
        let step=min(p.w,p.z-distance);
        let source=vec3(p.y);
        result=cloud_then(result,cloud_beer(p.x,source,step));
        distance+=step;
    }
    let aerial=cloud_with_air(result,vec3(0.7),vec3(0.04));
    let pixel=vec3(0.2)*aerial.transmittance+aerial.scattering;
    outputs[id.x]=vec4(result.transmittance,result.scattering.x,pixel.x,result.scattering.y);
}
";

#[test]
fn known_slabs_conserve_and_compose_once_at_coarse_and_fine_steps() {
    let Some(thrum_test_support::Opened { device, queue, .. }) = thrum_test_support::open(
        "cloud slab transport",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let rows: Vec<Vec<f32>> = include_str!("data/transport.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            l.split_whitespace()
                .map(|v| v.parse().expect("retained float"))
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), 5);
    let mut inputs = Vec::new();
    let mut expected = Vec::new();
    for row in &rows {
        assert_eq!(row.len(), 8);
        for step in [row[3], row[4]] {
            inputs.push([row[0], row[1], row[2], step]);
            expected.push([row[5], row[6], row[7], row[6]]);
        }
    }
    inputs.push([0.0, 0.0, 800.0, 25.0]);
    expected.push([1.0, 0.0, 0.2, 0.0]);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("cloud retained slab observer"),
        source: wgpu::ShaderSource::Wgsl(
            format!("{}\n{QUERY}", include_str!("../src/layer/transport.wgsl")).into(),
        ),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[entry(0, true), entry(1, false)],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("cloud retained slab observer"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = input.size();
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
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
        pass.dispatch_workgroups(u32::try_from(inputs.len()).expect("ten cases"), 1, 1);
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(&device, &mut encoder, &output, ..);
    let submission = queue.submit([encoder.finish()]);
    let actual: Vec<[f32; 4]> = readback
        .wait(&device, submission)
        .as_chunks::<16>()
        .0
        .iter()
        .map(|chunk| bytemuck::cast::<[u8; 16], [f32; 4]>(*chunk))
        .collect();
    for (i, (got, want)) in actual.iter().zip(expected).enumerate() {
        for channel in 0..4 {
            let bound = 3e-5 * (1.0 + want[channel].abs());
            assert!(
                (got[channel] - want[channel]).abs() <= bound,
                "case {i} channel {channel}: {got:?} expected {want:?}"
            );
        }
        assert!((0.0..=1.0).contains(&got[0]));
        if inputs[i][0] > 0.0 && inputs[i][1] > 0.0 {
            let energy = got[0] + got[1] / (inputs[i][1] / inputs[i][0]);
            assert!(
                energy <= 1.0 + 3e-5,
                "constant slab creates energy: {energy}"
            );
        }
    }
    assert_eq!(actual.last(), Some(&[1.0, 0.0, 0.2, 0.0]));
}

fn entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
