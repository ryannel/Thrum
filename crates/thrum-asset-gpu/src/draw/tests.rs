//! The actual cooked march with controlled density, sky and independent f64 transport.
use wgpu::util::DeviceExt;

const FIXTURE: &str = r"
struct View { view_proj:mat4x4<f32>, camera_xyz_time:vec4<f32> }
@group(0) @binding(0) var<uniform> view:View;
struct Light { direction_to_light:vec4<f32>, irradiance:vec4<f32>, ambient:vec4<f32> }
@group(0) @binding(1) var<uniform> light:Light;
struct Aerial {sun_lux:vec4<f32>}
const aerial_frame=Aerial(vec4(0.0,1.0,0.0,1.0));
@group(1) @binding(6) var aerial_sampler:sampler;
fn ambient_irradiance(n:vec3<f32>)->vec3<f32>{return light.ambient.rgb;}
fn aerial_colour(colour:vec3<f32>,world:vec3<f32>,pixel:vec4<f32>)->vec3<f32>{return colour;}
fn scene_storage_scale()->f32{return 1.0;}
struct FragmentOutput { @location(0) colour:vec4<f32>, @location(1) motion:vec2<f32> }
@vertex fn fixture_vertex(@builtin(vertex_index) index:u32)->CookedVolumeOutput {
    let positions=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return CookedVolumeOutput(vec4(positions[index],0.0,1.0),view.camera_xyz_time.xyz+vec3(3.0,0.0,0.0));
}
";

#[test]
fn metal_cooked_march_uses_each_world_sample_and_preserves_sky() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("valid Metal qualification fixture");
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("valid Metal qualification fixture");
        let cloud = include_str!("../../../thrum-render/src/atmosphere/cloud.wgsl");
        let volume = super::cooked_source(true);
        let volume = volume
            .split_once("struct CookedVolume {")
            .expect("valid Metal qualification fixture")
            .1;
        let pipeline = pipeline(&device, cloud, volume);
        let density = texture(
            &device,
            wgpu::TextureFormat::R8Unorm,
            wgpu::TextureDimension::D3,
            [2, 2, 2],
        );
        queue.write_texture(
            density.as_image_copy(),
            &[255; 8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(2),
                rows_per_image: Some(2),
            },
            density.size(),
        );
        let tau = texture(
            &device,
            wgpu::TextureFormat::R16Float,
            wgpu::TextureDimension::D2,
            [2, 1, 1],
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let output = texture(
            &device,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureDimension::D2,
            [1, 1, 1],
        );
        let motion = texture(
            &device,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureDimension::D2,
            [1, 1, 1],
        );
        let views = [
            density.create_view(&wgpu::TextureViewDescriptor::default()),
            tau.create_view(&wgpu::TextureViewDescriptor::default()),
            output.create_view(&wgpu::TextureViewDescriptor::default()),
            motion.create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let mut baseline = None;
        for (name, depths, translation, shade) in [
            ("zero", [0_u16, 0], [0.0_f32, 0.0, 0.0], false),
            ("ln2", [0x398c, 0x398c], [0.0, 0.0, 0.0], true),
            ("spatial", [0, 0x3d8c], [0.0, 0.0, 0.0], true),
            ("above", [0x398c, 0x398c], [0.0, 3.0, 0.0], false),
            ("outside", [0x398c, 0x398c], [3.0, 0.0, 0.0], false),
        ] {
            queue.write_texture(
                tau.as_image_copy(),
                depths.map(u16::to_le_bytes).as_flattened(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                tau.size(),
            );
            let model = glam::Mat4::from_translation(translation.into());
            let mut volume = model.to_cols_array().to_vec();
            volume.extend(model.inverse().to_cols_array());
            volume.extend([1.0, 16.0, 8.0, 0.0]);
            let volume = uniform(&device, &volume);
            let mut view = glam::Mat4::IDENTITY.to_cols_array().to_vec();
            view.extend([-1.0 + translation[0], 0.5 + translation[1], 0.5, 0.0]);
            let view = uniform(&device, &view);
            let field = uniform(
                &device,
                &[0.0, 0.0, 1.0, 1.0, 2.0, 0.0, 0.0, 0.01, 1.0, 0.0, 0.0, 0.0],
            );
            let mut observed = Vec::new();
            for sun in [false, true] {
                let source = if sun { 4.0 * std::f32::consts::PI } else { 0.0 };
                let light = uniform(
                    &device,
                    &[
                        0.0,
                        1.0,
                        0.0,
                        0.0,
                        source,
                        source,
                        source,
                        0.0,
                        std::f32::consts::FRAC_PI_2,
                        std::f32::consts::FRAC_PI_2,
                        std::f32::consts::FRAC_PI_2,
                        0.0,
                    ],
                );
                let first = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[buffer(0, &view), buffer(1, &light)],
                });
                let second = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(1),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 13,
                            resource: wgpu::BindingResource::TextureView(&views[1]),
                        },
                        buffer(14, &field),
                    ],
                });
                let third = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(2),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 8,
                            resource: wgpu::BindingResource::TextureView(&views[0]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 9,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        buffer(10, &volume),
                    ],
                });
                observed.push(draw(
                    &device,
                    &queue,
                    &pipeline,
                    [&first, &second, &third],
                    [&views[2], &views[3]],
                    &output,
                ));
            }
            let actual = observed[1][0] - observed[0][0];
            let mut expected = 0.0_f64;
            let light_tau = 2.0 * (3.0_f64).sqrt() / 8.0;
            for i in 0..16 {
                let x = (f64::from(i) + 0.5) / 16.0;
                let tau = if !shade {
                    0.0
                } else if name == "spatial" {
                    f64::from(half_value(depths[1])) * ((x - 0.25) / 0.5).clamp(0.0, 1.0)
                } else {
                    f64::from(half_value(depths[0]))
                };
                expected += (-f64::from(i) / 16.0).exp()
                    * (1.0 - (-1.0_f64 / 16.0).exp())
                    * (-light_tau).exp()
                    * (-tau).exp();
            }
            let expected_sky = (1.0 - (-1.0_f64).exp()) * 0.25;
            assert!(
                (f64::from(observed[0][0]) - expected_sky).abs() < 1e-6,
                "{name} sky {observed:?}"
            );
            assert!(
                (f64::from(actual) - expected).abs() < 0.001 * expected,
                "{name} direct {actual} expected {expected}"
            );
            if name == "zero" {
                baseline = Some(actual);
            }
            if name == "ln2" {
                assert!(
                    (actual / baseline.expect("valid Metal qualification fixture") - 0.5).abs()
                        < 0.0003
                );
            }
        }
    });
}

fn half_value(bits: u16) -> f32 {
    let exponent = i32::from((bits >> 10) & 31) - 15;
    (1.0 + f32::from(bits & 1023) / 1024.0) * 2.0_f32.powi(exponent)
}

fn uniform(device: &wgpu::Device, values: &[f32]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(values),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}

fn buffer(binding: u32, value: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: value.as_entire_binding(),
    }
}

fn texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    dimension: wgpu::TextureDimension,
    size: [u32; 3],
) -> wgpu::Texture {
    let mut usage = wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::COPY_SRC;
    if matches!(
        format,
        wgpu::TextureFormat::Rgba32Float | wgpu::TextureFormat::Rg32Float
    ) {
        usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: size[2],
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension,
        format,
        usage,
        view_formats: &[],
    })
}

fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    groups: [&wgpu::BindGroup; 3],
    views: [&wgpu::TextureView; 2],
    output: &wgpu::Texture,
) -> [f32; 4] {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let attachments = views.map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &attachments,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        for (i, group) in groups.into_iter().enumerate() {
            pass.set_bind_group(
                u32::try_from(i).expect("valid Metal qualification fixture"),
                group,
                &[],
            );
        }
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        output.size(),
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result)
            .expect("valid Metal qualification fixture");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("valid Metal qualification fixture");
    receive
        .recv()
        .expect("valid Metal qualification fixture")
        .expect("valid Metal qualification fixture");
    let bytes = readback
        .slice(..)
        .get_mapped_range()
        .expect("valid Metal qualification fixture");
    std::array::from_fn(|i| {
        f32::from_le_bytes(
            bytes[i * 4..i * 4 + 4]
                .try_into()
                .expect("valid Metal qualification fixture"),
        )
    })
}

fn pipeline(device: &wgpu::Device, cloud: &str, volume: &str) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production cooked march controlled receiver"),
        source: wgpu::ShaderSource::Wgsl(
            format!("{FIXTURE}\n{cloud}\nstruct CookedVolume {{{volume}").into(),
        ),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("cooked march qualification"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("fixture_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("cooked_volume_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg32Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
