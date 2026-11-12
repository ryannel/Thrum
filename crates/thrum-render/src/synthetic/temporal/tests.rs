use super::super::*;
use crate::capabilities::DeviceCapabilities;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("witness.json")).expect("shared witness definition")
}

async fn metal() -> (DeviceCapabilities, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("M2 Metal adapter");
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("M2 Metal device");
    (DeviceCapabilities::capture(&adapter, &device, 41), queue)
}

fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    bytes_per_pixel: u32,
) -> Vec<u8> {
    let width = texture.width();
    let stride = (width * bytes_per_pixel).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("M2 attachment readback"),
        size: u64::from(stride * texture.height()),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    let (send, recv) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |r| {
        send.send(r).expect("M2 mapping reply");
    });
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("M2 read completion");
    recv.recv()
        .expect("M2 read callback")
        .expect("M2 mapped bytes");
    let view = buffer
        .slice(..)
        .get_mapped_range()
        .expect("M2 mapped range");
    view.chunks_exact(stride as usize)
        .flat_map(|row| row[..(width * bytes_per_pixel) as usize].iter().copied())
        .collect()
}

fn submit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    lighting: FrameLighting,
) {
    prepared
        .update_frame(queue, target, scene, lighting, 1.0)
        .expect("M2 frame update");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 41, target, None)
        .expect("M2 real graph encode");
    queue.submit([encoder.finish()]);
    prepared.commit_frame();
}

#[test]
fn metal_temporal_reader_storage_and_invalid_geometry_w1_w2() {
    pollster::block_on(async {
        let (capabilities, queue) = metal().await;
        let device = capabilities.device();
        let expected = fixture();
        let mut baseline = None;
        for enabled in [false, true] {
            let mut scene = SyntheticScene::grid(1).expect("one geometry subject");
            scene.view = scene.view.with_temporal_inputs(enabled);
            let capabilities = capabilities
                .clone()
                .with_recipe_override("apple-tile")
                .expect("M3 Max tile recipe");
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .await
            .expect("M2 pipelines");
            let mut target =
                OffscreenTarget::new(device, &prepared, 64, 64).expect("M2 graph target");
            let depth = target.textures.texture(&target.ids.depth).expect("depth");
            assert_eq!(
                depth.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING),
                enabled
            );
            assert_eq!(
                crate::capabilities::apple::storage_mode(depth),
                Some(if enabled { "Private" } else { "Memoryless" })
            );
            assert_eq!(target.temporal_witness().is_some(), enabled);
            println!("M2 D2.1 enabled={enabled}\n{}", target.storage_report());
            if !enabled {
                for _ in 0..2 {
                    submit(
                        device,
                        &queue,
                        &mut prepared,
                        &mut target,
                        &scene,
                        FrameLighting::Diagnostic(SyntheticLight::default()),
                    );
                }
                baseline = Some(read_texture(
                    device,
                    &queue,
                    target
                        .textures
                        .texture(&target.ids.output)
                        .expect("normal output"),
                    4,
                ));
                continue;
            }
            for row in ["W1", "W2"] {
                submit(
                    device,
                    &queue,
                    &mut prepared,
                    &mut target,
                    &scene,
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                );
                let reactive = read_texture(
                    device,
                    &queue,
                    target
                        .textures
                        .texture(&target.ids.reactive)
                        .expect("reactive attachment"),
                    1,
                );
                let value = expected["rows"][row]["reactive"]
                    .as_f64()
                    .expect("numeric row");
                assert_eq!(
                    f64::from(reactive[32 * 64 + 32]) / 255.0,
                    value,
                    "{row} geometry pixel"
                );
                assert_eq!(reactive[0], 0, "uncovered clear");
                println!(
                    "M2 {row}: reactive={} uncovered={}",
                    reactive[32 * 64 + 32],
                    reactive[0]
                );
            }
            assert_eq!(
                baseline.as_ref().expect("disabled control"),
                &read_texture(
                    device,
                    &queue,
                    target
                        .textures
                        .texture(&target.ids.output)
                        .expect("enabled output"),
                    4
                ),
                "stored inputs must not change ordinary colour"
            );
        }
    });
}

#[test]
fn metal_sky_and_star_reactive_edit_then_steady_w4_w5() {
    pollster::block_on(async {
        let (capabilities, queue) = metal().await;
        let device = capabilities.device();
        let expected = fixture();
        for night in [false, true] {
            let mut scene = SyntheticScene::grid(1).expect("scene");
            scene.instances[0].ids[3] = 0;
            scene.star_catalogue = Some(&[[0.0, 0.0, -1.0, 9.6e-6]]);
            scene.view = SyntheticView::perspective(
                [0.0, 2.0, 3.0],
                [0.0, 2.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                20.0,
            )
            .expect("view")
            .with_temporal_inputs(true);
            if night {
                scene.view = scene
                    .view
                    .with_celestial_basis(glam::Mat3::IDENTITY.to_cols_array_2d())
                    .expect("star basis");
            }
            let lighting = FrameLighting::Atmosphere(
                AtmosphereFrame::new(
                    if night {
                        [0.0, -1.0, 0.0]
                    } else {
                        [0.0, 0.0, -1.0]
                    },
                    127_000.0,
                    [1.0; 3],
                    2.0,
                    1,
                )
                .expect("Earth"),
            );
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                lighting,
                1.0,
            )
            .await
            .expect("sky/star pipelines");
            let mut target =
                OffscreenTarget::new(device, &prepared, 64, 64).expect("sky/star target");
            for row in ["W5", "W4", "W5"] {
                if night {
                    if row == "W4" {
                        scene.view = scene.view.with_celestial_epoch(1);
                    }
                } else {
                    scene.view = scene.view.with_history_reset(row == "W4");
                }
                submit(device, &queue, &mut prepared, &mut target, &scene, lighting);
                let reactive = read_texture(
                    device,
                    &queue,
                    target
                        .textures
                        .texture(&target.ids.reactive)
                        .expect("reactive"),
                    1,
                );
                let value = expected["rows"][row]["reactive"]
                    .as_f64()
                    .expect("row reactive");
                assert_eq!(
                    f64::from(reactive[32 * 64 + 32]) / 255.0,
                    value,
                    "{row} night={night}"
                );
                assert_eq!(target.draw_stars, night);
                println!("M2 {row} night={night} centre={}", reactive[32 * 64 + 32]);
            }
        }
    });
}

#[expect(
    clippy::print_stdout,
    reason = "the M2 proof log records GPU storage-domain conversion"
)]
pub(in crate::synthetic) fn assert_history_rescale(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
    current_inverse: f32,
    previous_side: usize,
    previous_p: f32,
    current_p: f32,
) {
    let resolved = target
        .textures
        .texture(&target.ids.temporal.exposure)
        .expect("resolved exposure texture");
    assert_eq!(
        resolved.size(),
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1
        }
    );
    assert_eq!(resolved.format(), wgpu::TextureFormat::R16Float);
    let bytes = read_texture(device, queue, resolved, 2);
    let actual_exposure = half::f16::from_bits(u16::from_le_bytes(
        bytes[..2].try_into().expect("R16 exposure"),
    ))
    .to_f32();
    let expected_exposure = 1.0 / (current_p * current_inverse);
    assert!(
        (actual_exposure - expected_exposure).abs() < expected_exposure * 0.002,
        "resolved exposure {actual_exposure} vs {expected_exposure}"
    );
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor { label:Some("M2 retained exposure rescale witness"),source:wgpu::ShaderSource::Wgsl(format!("{}\n@group(0) @binding(4) var<uniform> previous: ExposureSample;\n@fragment fn history_rescale() -> @location(0) f32 {{ return rescale_storage(previous.values.x * 2.0, previous.values.x, exposure.values.x); }}",include_str!("../temporal.wgsl")).into()) });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("M2 history storage conversion"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("history_rescale"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: prepared.exposure.samples[prepared.history.write_side()]
                    .as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: prepared.exposure.samples[previous_side].as_entire_binding(),
            },
        ],
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("M2 rescaled retained colour"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..wgpu::RenderPassDescriptor::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
    let bytes = read_texture(device, queue, &texture, 2);
    let actual = half::f16::from_bits(u16::from_le_bytes(
        bytes[..2].try_into().expect("one R16Float texel"),
    ))
    .to_f32();
    let expected = 2.0 * current_p;
    assert!(
        (actual - expected).abs() < expected * 0.002 + 1e-7,
        "history rescaled to current p: {actual} vs {expected}"
    );
    assert!(
        (actual - 2.0 * previous_p).abs() > expected * 0.1,
        "unscaled reinterpretation must fail"
    );
    println!(
        "M2 D2.3 retained p={previous_p}, current p={current_p}, GPU-rescaled retained colour={actual}, unscaled={}",
        2.0 * previous_p
    );
}

#[test]
fn metal_history_key_uses_output_grid_across_render_extent_changes() {
    pollster::block_on(async {
        let (capabilities, queue) = metal().await;
        let device = capabilities.device();
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = scene.view.with_output_extent([128, 72]);
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .await
        .expect("scene");
        let mut prior = 0;
        for render in [[64, 36], [32, 18]] {
            let mut target =
                OffscreenTarget::new(device, &prepared, render[0], render[1]).expect("render grid");
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                    1.0,
                )
                .expect("update");
            assert_eq!(
                prepared.temporal_history_key().expect("key").extent,
                [128, 72]
            );
            assert_eq!(
                prepared.history_identity()[1],
                prior,
                "scale must preserve submitted history"
            );
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode(&mut encoder, 41, &target, None)
                .expect("encode");
            queue.submit([encoder.finish()]);
            prior = prepared.history_identity()[0];
            prepared.commit_frame();
        }
        let mut target =
            OffscreenTarget::new(device, &prepared, 32, 18).expect("cancelled key target");
        scene.view = scene
            .view
            .with_output_extent([256, 144])
            .with_history_epoch(1);
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .expect("prepare cancelled key");
        let mut cancelled =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode(&mut cancelled, 41, &target, None)
            .expect("encode cancelled key");
        drop(cancelled);
        scene.view = scene
            .view
            .with_output_extent([128, 72])
            .with_history_epoch(0);
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .expect("restore submitted key");
        assert_eq!(
            prepared.history_identity()[1],
            prior,
            "an unsubmitted key/cut must not erase the retained sample"
        );
        println!(
            "M2 D2.4 HistoryKey output=[128,72] retained across render [64,36] -> [32,18]; abandoned output/correspondence key leaves prior sample intact"
        );
    });
}

#[test]
fn temporal_shader_validates_and_retains_exposure_sample_abi() {
    let module = wgpu::naga::front::wgsl::parse_str(include_str!("../temporal.wgsl"))
        .expect("temporal WGSL parses");
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("temporal WGSL validates");
    let ty = module
        .types
        .iter()
        .find_map(|(_, t)| (t.name.as_deref() == Some("ExposureSample")).then_some(&t.inner))
        .expect("matching sample ABI");
    assert!(matches!(ty, wgpu::naga::TypeInner::Struct { span: 32, .. }));
}
