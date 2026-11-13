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
    samples(device, queue, prepared, false)
        .iter()
        .map(|v| v[0])
        .collect()
}

fn samples(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    shade: bool,
) -> Vec<[f32; 4]> {
    let target = crate::synthetic::OffscreenTarget::new(device, prepared, 8, 1).unwrap();
    let mut integration = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut integration, 151, &target, None)
        .unwrap();
    queue.submit([integration.finish()]);
    let probe = if shade {
        PROBE.replace(
            "return vec4(cloud_factor(CLOUD_TEST_POINTS[u32(pixel.x)]));",
            r"
            let index=u32(pixel.x);
            let selected=array<u32,4>(0u,1u,3u,5u);
            var input:VertexOutput;
            input.position=vec4(f32(index%2u),0.5,0.0,1.0);
            input.uv=vec2(0.5);input.material=0u;input.normal=vec3(0.0,1.0,0.0);
            input.world_position=CLOUD_TEST_POINTS[selected[index/2u]];
            input.clip_delta=vec4(0.0);input.current_clip=vec4(0.0,0.0,0.0,1.0);
            input.history_valid=0u;
            let receiver=ShadowReceiver(input.world_position,vec3(0.0),vec3(0.0),input.normal);
            return shade(input,receiver,true,false);
        ",
        )
    } else {
        PROBE.into()
    };
    // The paired pixels supply geometric Sun visibility 0/1 independently of
    // the cloud field, preserving the production material and cloud evaluation.
    let source = if shade {
        assert!(
            prepared
                .surface
                .shader_source
                .contains("var sun_factor = 1.0;")
        );
        prepared.surface.shader_source.replace(
            "var sun_factor = 1.0;",
            "var sun_factor = input.position.x;",
        )
    } else {
        prepared.surface.shader_source.to_string()
    };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production cloud-factor observer"),
        source: wgpu::ShaderSource::Wgsl(format!("{source}\n{probe}").into()),
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
        layout: Some(if shade {
            &prepared.surface.pipeline_layout
        } else {
            &layout
        }),
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
        if shade {
            pass.set_bind_group(0, &target.view_bind_group, &[0]);
            pass.set_bind_group(2, &prepared.materials, &[]);
            pass.set_bind_group(3, prepared.history.read_group(), &[]);
        } else {
            pass.set_bind_group(0, &group, &[]);
        }
        pass.set_bind_group(
            1,
            &prepared.light[prepared.contribution_frame().exposure_side],
            &[],
        );
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
        .map(|pixel| {
            std::array::from_fn(|i| {
                f32::from_le_bytes(pixel[i * 4..i * 4 + 4].try_into().expect("sample channel"))
            })
        })
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

#[test]
fn metal_production_surface_shade_dims_only_direct_sun_once_at_each_receiver() {
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
        let caps = DeviceCapabilities::capture(&adapter, &device, 151);
        let frame = AtmosphereFrame::new([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], 2.0, 1).unwrap();
        let make = |values: &[f32]| {
            SunOpticalDepth::new([2, 2], values, 1.0, [-1.0, -1.0, 1.0, 1.0], 0.01, 1).unwrap()
        };
        let mut baseline = None;
        for (name, field, expected) in [
            ("absent", None, [1.0; 4]),
            ("zero", Some(make(&[0.0; 4])), [1.0; 4]),
            (
                "ln2",
                Some(make(&[std::f32::consts::LN_2; 4])),
                [0.5, 0.5, 1.0, 1.0],
            ),
            (
                "spatial",
                Some(make(&[
                    0.0,
                    2.0 * std::f32::consts::LN_2,
                    0.0,
                    2.0 * std::f32::consts::LN_2,
                ])),
                [1.0, 0.25, 1.0, 1.0],
            ),
        ] {
            let mut scene = SyntheticScene::grid(1).unwrap();
            scene.materials[0].emissive_cutoff = [2_000.0, 1_000.0, 500.0, 0.0];
            scene.view = crate::synthetic::SyntheticView::perspective(
                [0.0, 2.0, 4.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                8.0,
                0.1,
                32_000.0,
            )
            .unwrap();
            scene.sun_optical_depth = field;
            let prepared = PreparedSynthetic::prepare_with_capabilities(
                &caps,
                &queue,
                &scene,
                FrameLighting::Atmosphere(frame),
                1.0,
            )
            .await
            .unwrap();
            let actual = samples(&device, &queue, &prepared, true);
            if let Some(base) = &baseline {
                let base: &Vec<[f32; 4]> = base;
                for (receiver, factor) in expected.iter().enumerate() {
                    for c in 0..3 {
                        let ambient = base[receiver * 2][c];
                        assert!(ambient > 1e-5, "positive sky and emission witness");
                        let direct = base[receiver * 2 + 1][c] - ambient;
                        assert!(
                            direct > 1e-8,
                            "positive direct witness {receiver} {c} baseline {base:?}"
                        );
                        assert!(
                            (actual[receiver * 2][c] - ambient).abs()
                                <= 4.0 * f32::EPSILON * ambient.max(1e-8),
                            "{name} changed geometric-shadow/sky/emission receiver {receiver}"
                        );
                        let wanted = ambient + direct * factor;
                        assert!(
                            (actual[receiver * 2 + 1][c] - wanted).abs()
                                <= 0.001 * direct.abs() + 4.0 * f32::EPSILON * wanted.abs(),
                            "{name} receiver {receiver} channel {c} actual {} expected {wanted}",
                            actual[receiver * 2 + 1][c]
                        );
                    }
                }
            } else {
                baseline = Some(actual);
            }
        }
    });
}
