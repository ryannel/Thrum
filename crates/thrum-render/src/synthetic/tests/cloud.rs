//! Exercise the production cloud variant and its shared point lookup on Metal.
use crate::synthetic::{FrameLighting, PreparedSynthetic, SyntheticScene};
use crate::{
    atmosphere::{AtmosphereFrame, SunOpticalDepth},
    capabilities::DeviceCapabilities,
};
use std::{sync::mpsc, time::Duration};

const PROBE: &str = r"
const CLOUD_TEST_POINTS=array<vec3<f32>,8>(
    vec3(-0.5,0.0,0.0),vec3(0.5,0.0,0.0),
    vec3(0.0,1.0,0.0),vec3(0.0,2.0,0.0),
    vec3(-2.0,0.0,0.0),vec3(2.0,0.0,0.0),
    vec3(-0.5,0.5,0.0),vec3(0.0,0.0,0.0));
@vertex fn cloud_test_vertex(@builtin(vertex_index) index:u32)->@builtin(position) vec4<f32> {
    let positions=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.0,1.0);
}
@fragment fn cloud_test_fragment(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
    return vec4(cloud_factor(CLOUD_TEST_POINTS[u32(pixel.x)]));
}
";

fn factors(device: &wgpu::Device, queue: &wgpu::Queue, prepared: &PreparedSynthetic) -> Vec<f32> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production cloud-factor observer"),
        source: wgpu::ShaderSource::Wgsl(
            format!("{}\n{PROBE}", prepared.surface.shader_source).into(),
        ),
    });
    let empty = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&empty), Some(&prepared.surface.light_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shared cloud lookup in fragment pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("cloud_test_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("cloud_test_fragment"),
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
        label: Some("unexposed cloud factors"),
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
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &empty,
        entries: &[],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
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
        pass.set_bind_group(1, &prepared.light[0], &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        texture.size(),
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("factor receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("factor draw completes");
    receive
        .recv()
        .expect("factor map callback")
        .expect("factor mapped");
    readback
        .slice(..)
        .get_mapped_range()
        .expect("mapped factor pixels")[..128]
        .as_chunks::<16>()
        .0
        .iter()
        .map(|pixel| f32::from_le_bytes(pixel[..4].try_into().expect("factor channel")))
        .collect()
}

#[test]
fn metal_cloud_factor_uses_receiver_ray_and_returns_exact_clear_support() {
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
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 151);
        eprintln!(
            "Metal cloud-factor qualification, device sampled-texture limit {}",
            capabilities.limits().max_sampled_textures_per_shader_stage
        );
        let uniform = SunOpticalDepth::new(
            [2, 2],
            &[std::f32::consts::LN_2; 4],
            1.0,
            [-1.0, -1.0, 1.0, 1.0],
            0.01,
            1,
        )
        .unwrap();
        let gradient = SunOpticalDepth::new(
            [2, 2],
            &[
                0.0,
                std::f32::consts::LN_2 * 2.0,
                0.0,
                std::f32::consts::LN_2 * 2.0,
            ],
            1.0,
            [-1.0, -1.0, 1.0, 1.0],
            0.01,
            2,
        )
        .unwrap();
        for (label, field, sun, expected) in [
            (
                "zero optical depth",
                SunOpticalDepth::new([2, 2], &[0.0; 4], 1.0, [-1.0, -1.0, 1.0, 1.0], 0.01, 3)
                    .unwrap(),
                [0.0, 1.0, 0.0],
                [1.0; 8],
            ),
            (
                "uniform ln2",
                uniform,
                [0.0, 1.0, 0.0],
                [0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5],
            ),
            (
                "two points",
                gradient.clone(),
                [0.0, 1.0, 0.0],
                [1.0, 0.25, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5],
            ),
            (
                "project each height",
                gradient.clone(),
                [1.0, 1.0, 0.0],
                [0.25, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.25],
            ),
            (
                "advected field",
                gradient.clone().with_displacement([0.5, 0.0]).unwrap(),
                [0.0, 1.0, 0.0],
                [1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            ),
            (
                "invalid sample",
                gradient.clone().with_validity(false),
                [0.0, 1.0, 0.0],
                [1.0; 8],
            ),
            ("unsupported Sun", gradient, [1.0, 0.0, 0.0], [1.0; 8]),
        ] {
            let mut scene = SyntheticScene::grid(1).unwrap();
            scene.sun_optical_depth = Some(field);
            let frame = AtmosphereFrame::new(sun, 127_000.0, [1.0; 3], 2.0, 1).unwrap();
            let prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                FrameLighting::Atmosphere(frame),
                1.0,
            )
            .await
            .unwrap();
            let actual = factors(&device, &queue, &prepared);
            eprintln!("{label}: {actual:?}");
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                let expected: f32 = expected;
                if expected.to_bits() == 1.0_f32.to_bits() {
                    assert_eq!(*actual, 1.0, "{label} receiver {index}");
                } else {
                    assert!(
                        (actual - expected).abs() < 0.0003,
                        "{label} receiver {index}: {actual} expected {expected}"
                    );
                }
            }
        }
    });
}
