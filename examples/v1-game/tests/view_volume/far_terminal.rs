//! Lent aerial transport on the same sky-ending ray at four distances.
use crate::{fixture, medium_fixture};
use thrum_render::{
    atmosphere::{AtmosphereFrame, SunOpticalDepth},
    binding,
    synthetic::{PreparedSynthetic, SyntheticView},
};
use thrum_test_support::readback::Readback;

const DISTANCES: [u32; 4] = [32_000, 40_000, 100_000, 200_000];
const PROBE: &str = r"
@group(1) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@compute @workgroup_size(1)
fn read_aerial(@builtin(global_invocation_id) row: vec3<u32>) {
    let distances = array(32000.0, 40000.0, 100000.0, 200000.0);
    let direction = vec3(cos(0.5), sin(0.5), 0.0);
    let point = atmosphere.camera_altitude.xyz + direction * distances[row.x];
    let uv = vec2(0.5);
    let sample = environment_aerial(point, uv);
    output[row.x * 2u] = vec4(sample.transmission, 0.0);
    output[row.x * 2u + 1u] = vec4(sample.scatter, 0.0);
    if row.x == 0u {
        let layer = i32(textureNumLayers(aerial_scatter)) - 1;
        let terminal = textureSampleLevel(aerial_scatter, aerial_sampler, uv, layer, 0.0);
        let diffuse = textureSampleLevel(aerial_transmittance, aerial_sampler, uv, layer, 0.0).a;
        output[8] = vec4(aerial_radiance(terminal.rgb) + vec3(diffuse), terminal.a);
    }
}
";

fn read(gpu: &fixture::Gpu, scene: &PreparedSynthetic) -> [[f32; 4]; 9] {
    let products = scene
        .environment_products_witness(&gpu.queue)
        .expect("lent atmosphere");
    let device = &gpu.device;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("far aerial terminal probe"),
        source: wgpu::ShaderSource::Wgsl(format!("{}\n{PROBE}", products.shader_source).into()),
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("far aerial readback"),
        size: 9 * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[binding::storage(
            0,
            wgpu::ShaderStages::COMPUTE,
            false,
            output.size(),
        )],
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&products.layout), Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("read lent far aerial"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("read_aerial"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, products.group(0), &[]);
        pass.set_bind_group(1, &group, &[]);
        pass.dispatch_workgroups(4, 1, 1);
    }
    let read = Readback::buffer(device, &mut encoder, &output, ..);
    let submitted = gpu.queue.submit([encoder.finish()]);
    let bytes = read.wait(device, submitted);
    std::array::from_fn(|row| {
        std::array::from_fn(|lane| {
            let at = (row * 4 + lane) * 4;
            f32::from_le_bytes(bytes[at..at + 4].try_into().expect("probe lane"))
        })
    })
}

#[test]
fn metal_lent_lookup_retains_the_near_correction_to_200_kilometres() {
    let Some(gpu) = fixture::Gpu::open(11, "lent far aerial terminal correction") else {
        return;
    };
    let mut scene = fixture::scene(false, false, false);
    let (sin, cos) = 0.5_f32.sin_cos();
    scene.view = SyntheticView::perspective(
        [0.0, 260.0, 0.0],
        [cos, 260.0 + sin, 0.0],
        [0.0, 1.0, 0.0],
        0.75,
        33.0 / 19.0,
        0.1,
        220_000.0,
    )
    .expect("sky-ending ray");
    let path = |tau| {
        SunOpticalDepth::new(
            [2, 2],
            &[tau; 4],
            2000.0,
            [-20_000.0, -20_000.0, 20_000.0, 20_000.0],
            0.01,
            1,
        )
        .expect("shade")
    };
    let air = AtmosphereFrame::new([0.0, 1.0, 0.0], 100_000.0, [1.0; 3], 260.0, 1)
        .expect("Sun")
        .with_view_volume();
    let measure = |tau, light, coeffs| {
        let mut scene = scene.clone();
        scene.sun_optical_depth = Some(path(tau));
        medium_fixture::render_and_probe(&gpu, &scene, light, coeffs, |prepared| {
            read(&gpu, prepared)
        })
        .1
    };
    let clear = measure(0.0, air, None);
    let shadow = measure(4.0, air, None);
    let fog = measure(
        0.0,
        air.with_view_volume_medium(1),
        Some((
            "select(0.0, 1.0, x >= 25.3125 && x < 45.0)",
            [0.035, 0.002, 0.127],
        )),
    );
    assert_eq!(shadow[8][3].to_bits(), 1.0_f32.to_bits());
    assert!(shadow[8][2] < -50.0 && fog[8][3] < 0.6);
    for (index, distance) in DISTANCES.into_iter().enumerate() {
        let t = index * 2;
        let s = t + 1;
        for lane in 0..3 {
            assert_eq!(clear[t][lane].to_bits(), shadow[t][lane].to_bits());
            let deficit = shadow[s][lane] - clear[s][lane];
            assert!(
                (deficit - shadow[8][lane]).abs() < 1.0 + 0.004 * shadow[8][lane].abs(),
                "{distance} m lane {lane}: shadow deficit {deficit}, terminal {}",
                shadow[8][lane]
            );
            assert!((fog[t][lane] - fog[8][3] * clear[t][lane]).abs() < 0.0015);
            let expected = fog[8][3] * clear[s][lane] + fog[8][lane];
            assert!(
                (fog[s][lane] - expected).abs() < 1.0 + 0.004 * expected.abs(),
                "{distance} m lane {lane}: scatter {} vs {expected}",
                fog[s][lane]
            );
        }
        println!(
            "{distance} m: clear {:?}, shadow {:?}, medium {:?}",
            clear[s], shadow[s], fog[s]
        );
    }
    println!(
        "far aerial present; shadow terminal {:?}, medium terminal {:?}",
        shadow[8], fog[8]
    );
}
