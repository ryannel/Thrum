//! Read the shared forward lookup and the real incident-light resolve on Metal.
#[expect(
    dead_code,
    reason = "the shared fixture also reads cumulative view volumes"
)]
mod fixture;

use std::f32::consts::LN_2;
use thrum_render::{
    atmosphere::{GroundBand, GroundBandMapping, SunOpticalDepth},
    synthetic::PreparedSynthetic,
};
use thrum_test_support::readback::{Mip, Readback};

const PROBE: &str = r"
const RECEIVERS = array<vec3<f32>, 8>(
    vec3(0.0,0.0,0.0), vec3(0.0,2.0,0.0), vec3(0.0,3.99,0.0), vec3(0.0,4.0,0.0),
    vec3(1.99,2.0,0.0), vec3(2.00,2.0,0.0), vec3(1.5,2.0,0.0), vec3(-3.0,1.0,0.0));
@vertex fn probe_vertex(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
    return vec4(array(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0))[id], 0.0, 1.0);
}
@fragment fn probe_factor(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4(cloud_factor(RECEIVERS[u32(pixel.x)]));
}
";

fn factors(gpu: &fixture::Gpu, prepared: &PreparedSynthetic) -> Vec<f32> {
    let device = &gpu.device;
    let (wgsl, light_layout, light) = prepared.forward_light_witness();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ground-band receivers"),
        source: wgpu::ShaderSource::Wgsl(format!("{wgsl}\n{PROBE}").into()),
    });
    let empty = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[],
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &empty,
        entries: &[],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&empty), Some(light_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ground-band receiver probe"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("probe_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("probe_factor"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba32Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ground-band factors"),
        size: wgpu::Extent3d {
            width: 8,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.set_bind_group(1, light, &[]);
        pass.draw(0..3, 0..1);
    }
    let readback = Readback::texture(device, &mut encoder, &texture, Mip::default(), ..);
    let submission = gpu.queue.submit([encoder.finish()]);
    readback
        .wait(device, submission)
        .as_chunks::<16>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(bytes[..4].try_into().expect("receiver factor")))
        .collect()
}

fn incident_lux(
    gpu: &fixture::Gpu,
    prepared: &PreparedSynthetic,
    target: &thrum_render::synthetic::OffscreenTarget,
) -> f32 {
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, gpu.caps.generation(), target, None)
        .expect("incident resolve");
    let readback = Readback::buffer(
        &gpu.device,
        &mut encoder,
        prepared.incident_meter_witness(),
        0..32,
    );
    let submission = gpu.queue.submit([encoder.finish()]);
    let bytes = readback.wait(&gpu.device, submission);
    f32::from_le_bytes(bytes[12..16].try_into().expect("incident lux"))
}

#[test]
fn metal_selects_one_whole_sun_path_at_the_receivers_height() {
    let Some(gpu) = fixture::Gpu::open(10, "ground-band receivers") else {
        return;
    };
    let plane = |tau| {
        SunOpticalDepth::new(
            [2, 2],
            &[tau; 4],
            40.0,
            [-1000.0, -1000.0, 1000.0, 1000.0],
            0.001,
            1,
        )
        .expect("upper cloud")
    };
    let mapping = GroundBandMapping {
        region: [-2.0, -2.0, 2.0, 2.0],
        top_m: 4.0,
        border_m: 1.0,
        padding: [0.0; 2],
    };
    let band = |base, middle, top| {
        GroundBand::new(
            [2, 2, 3],
            mapping,
            &[base, middle, top]
                .into_iter()
                .flat_map(|tau| [[tau, 0.0, 0.0]; 4])
                .collect::<Vec<_>>(),
        )
        .expect("ground band")
    };
    let light = fixture::daylight(false, 1);
    let mut scene = fixture::scene(false, false, false);
    scene.view = scene
        .view
        .with_incident_exposure(0.0, -12.0, 18.0)
        .expect("incident exposure");
    let (absent, _) = gpu.prepare(&scene, light);
    assert!(!absent.forward_light_witness().0.contains("fn cloud_factor"));
    scene.sun_optical_depth = Some(plane(LN_2).with_ground_band(band(LN_2, LN_2 * 2.0, LN_2)));
    let (prepared, target) = gpu.prepare(&scene, light);
    let result = factors(&gpu, &prepared);
    let expects = [0.5, 0.25, 0.5, 0.5, 0.5, 0.5, 0.375, 0.5];
    for (actual, expected) in result.iter().zip(expects) {
        assert!((actual - expected).abs() < 0.002, "factors {result:?}");
    }
    assert!(
        (result[0] - 0.25).abs() > 0.2,
        "adding the 2D field survived"
    );
    assert!(
        (result[0] - result[1]).abs() > 0.24,
        "wrong height survived"
    );
    let quarter = incident_lux(&gpu, &prepared, &target);
    scene.sun_optical_depth = Some(plane(LN_2).with_ground_band(band(LN_2, LN_2, LN_2)));
    let (half_scene, half_target) = gpu.prepare(&scene, light);
    let half = incident_lux(&gpu, &half_scene, &half_target);
    scene.sun_optical_depth = Some(plane(0.0).with_ground_band(band(0.0, 0.0, 0.0)));
    let (clear_scene, clear_target) = gpu.prepare(&scene, light);
    let clear = incident_lux(&gpu, &clear_scene, &clear_target);
    assert!(half > quarter && clear > half, "{quarter} {half} {clear}");
    assert!(((clear - half) / (half - quarter) - 2.0).abs() < 0.01);
    println!(
        "receiver factors={result:?}, incident lux quarter={quarter} half={half} clear={clear}, Metal sampled texture limit={}",
        gpu.caps.limits().max_sampled_textures_per_shader_stage
    );
}
