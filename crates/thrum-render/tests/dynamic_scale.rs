//! Metal witnesses for the drawn rectangle and preparation-only allocation.
#![cfg(target_os = "macos")]
#![expect(
    clippy::unwrap_used,
    reason = "a failed fixture operation fails the witness"
)]
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::{DeviceCapabilities, DeviceRecipe},
    synthetic::{
        FrameLighting, OffscreenTarget, PreparedSynthetic, RenderScale, SyntheticScene,
        SyntheticView,
    },
};
use thrum_test_support::readback::{Mip, Readback};

const EXTENT: [u32; 2] = [64, 64];
const GENERATION: u64 = 1;

async fn device() -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let gpu = thrum_test_support::gpu("render scale", Some(wgpu::Backend::Metal))
        .expect("the milestone requires Metal");
    let opened = DeviceRecipe::default()
        .open_admitted(&gpu.instance, &gpu.adapter)
        .await
        .unwrap();
    assert_eq!(opened.adapter.get_info().backend, wgpu::Backend::Metal);
    assert_eq!(
        opened.capabilities.recipe().unwrap(),
        thrum_render::capabilities::Recipe::Portable
    );
    (opened.device, opened.queue, opened.capabilities)
}

fn camera(yaw: f32, scale: RenderScale) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, 2.0, 0.0],
        [yaw.sin(), 2.0, -yaw.cos()],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        100.0,
    )
    .unwrap()
    .with_reconstruction(true)
    .with_render_scale(scale)
}

fn light() -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.0, 0.6, 0.8], 127_500.0, [1.0; 3], 2.0, 1).unwrap(),
    )
}

async fn prepared(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    scene: &SyntheticScene,
) -> (PreparedSynthetic, OffscreenTarget) {
    assert_eq!(caps.generation(), GENERATION);
    let prepared = PreparedSynthetic::prepare_with_capabilities(caps, queue, scene, light(), 1.0)
        .await
        .unwrap();
    let target = OffscreenTarget::new(device, &prepared, EXTENT[0], EXTENT[1]).unwrap();
    (prepared, target)
}

fn counters(device: &wgpu::Device) -> [isize; 5] {
    let hal = device.get_internal_counters().hal;
    [
        hal.textures.read(),
        hal.texture_views.read(),
        hal.bind_groups.read(),
        hal.compute_pipelines.read(),
        hal.render_pipelines.read(),
    ]
}

#[test]
fn scale_steps_create_no_gpu_objects() {
    pollster::block_on(async {
        let (device, queue, caps) = device().await;
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = camera(0.0, RenderScale::Native);
        let (mut renderer, mut target) = prepared(&device, &queue, &caps, &scene).await;
        let record = |renderer: &mut PreparedSynthetic,
                      target: &mut OffscreenTarget,
                      scene: &SyntheticScene| {
            renderer
                .update_frame(&queue, target, scene, light(), 1.0)
                .unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            renderer
                .encode(&mut encoder, GENERATION, target, None)
                .unwrap();
            queue.submit([encoder.finish()]);
            renderer.commit_frame();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        };
        record(&mut renderer, &mut target, &scene);
        let before = counters(&device);
        assert!(
            before.into_iter().all(|value| value > 0),
            "counter support: {before:?}"
        );
        for scale in RenderScale::STEPS
            .into_iter()
            .chain(RenderScale::STEPS.into_iter().rev())
        {
            scene.view = camera(0.0, scale);
            record(&mut renderer, &mut target, &scene);
            assert_eq!(target.scale_attachment_extents(), [EXTENT; 4]);
            assert_eq!(counters(&device), before, "creation at {scale:?}");
        }
        // Deliberately reallocate a target on another thread at the same step.
        scene.view = camera(0.0, RenderScale::Half);
        let mut replacement = std::thread::scope(|scope| {
            scope
                .spawn(|| OffscreenTarget::new(&device, &renderer, EXTENT[0], EXTENT[1]).unwrap())
                .join()
                .unwrap()
        });
        record(&mut renderer, &mut replacement, &scene);
        let after = counters(&device);
        assert!(
            after[..3].iter().zip(before).all(|(&a, b)| a > b),
            "control: {before:?} -> {after:?}"
        );
    });
}

fn marker(caps: &DeviceCapabilities) -> wgpu::RenderPipeline {
    let site = caps.site();
    let shader = site.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("attachment border marker"),
        source: wgpu::ShaderSource::Wgsl(
            "
            @vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
                let p = array(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
                return vec4(p[i],0.75,1.);
            }
            struct Marker {
                @location(0) colour: vec4<f32>,
                @location(1) motion: vec2<f32>,
                @location(2) reactive: f32,
            }
            @fragment fn fragment() -> Marker {
                return Marker(vec4(40.,0.,30.,1.),vec2(41.,-29.),1.);
            }
        "
            .into(),
        ),
    });
    let colours = [
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::Rg16Float,
        wgpu::TextureFormat::R8Unorm,
    ]
    .map(|format| {
        Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })
    });
    site.render(&wgpu::RenderPipelineDescriptor {
        label: Some("attachment border marker"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            targets: &colours,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
) -> (Vec<u8>, Vec<[f32; 2]>, f32) {
    renderer
        .update_frame(queue, target, scene, light(), 1.0)
        .unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut sdr = target.readback(device);
    renderer
        .encode_with_readback(&mut encoder, GENERATION, target, None, &mut sdr)
        .unwrap();
    let motion = Readback::texture(
        device,
        &mut encoder,
        target.scale_motion_witness(),
        Mip::level(0),
        ..,
    );
    let (view, offset) = renderer.scale_view_witness(target);
    let bias = Readback::buffer(device, &mut encoder, view, offset + 224..offset + 228);
    let submitted = queue.submit([encoder.finish()]);
    renderer.commit_frame();
    let bias_bytes = bias.wait(device, submitted.clone());
    let motion = motion
        .wait(device, submitted)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| {
            std::array::from_fn(|axis| {
                half::f16::from_le_bytes([pixel[axis * 2], pixel[axis * 2 + 1]]).to_f32()
            })
        })
        .collect();
    (
        sdr.try_collect(GENERATION).unwrap().unwrap(),
        motion,
        f32::from_le_bytes(bias_bytes.try_into().unwrap()),
    )
}

fn pixel(motion: &[[f32; 2]], x: u32, y: u32) -> [f32; 2] {
    motion[(y * EXTENT[0] + x) as usize]
}

#[test]
fn drawn_rectangle_protects_reconstruction_inputs_and_motion() {
    pollster::block_on(async {
        let (device, queue, caps) = device().await;
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.instances[0].current[3] = [0.0, 2.0, -3.0, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        scene.view = camera(0.0, RenderScale::Native);
        let (mut renderer, mut target) = prepared(&device, &queue, &caps, &scene).await;
        let (mut marked_renderer, mut marked) = prepared(&device, &queue, &caps, &scene).await;
        marked.set_border_witness(marker(&caps));
        frame(&device, &queue, &mut renderer, &mut target, &scene);
        frame(&device, &queue, &mut marked_renderer, &mut marked, &scene);
        for scale in RenderScale::STEPS {
            let [w, h] = scale.extent(EXTENT);
            for yaw in [0.0, 0.0, 0.02] {
                scene.view = camera(yaw, scale);
                let (clean, motion, bias) =
                    frame(&device, &queue, &mut renderer, &mut target, &scene);
                let (painted, marked_motion, _) =
                    frame(&device, &queue, &mut marked_renderer, &mut marked, &scene);
                assert_eq!(clean.len(), (EXTENT[0] * EXTENT[1] * 4) as usize);
                assert!(
                    clean == painted,
                    "{scale:?}: the marker altered the full output"
                );
                if w < EXTENT[0] {
                    assert_eq!(
                        pixel(&marked_motion, w, h / 2).map(f32::to_bits),
                        [41.0_f32, -29.0].map(f32::to_bits)
                    );
                    assert_eq!(
                        pixel(&marked_motion, w / 2, h).map(f32::to_bits),
                        [41.0_f32, -29.0].map(f32::to_bits)
                    );
                    assert_ne!(
                        pixel(&motion, w, h / 2).map(f32::to_bits),
                        [41.0_f32, -29.0].map(f32::to_bits)
                    );
                }
                let expected_bias = (f64::from(w) / f64::from(EXTENT[0])).log2() - 1.0;
                assert!((f64::from(bias) - expected_bias).abs() < 1e-6);
            }
            let (_, stationary, _) = frame(&device, &queue, &mut renderer, &mut target, &scene);
            let _marked = frame(&device, &queue, &mut marked_renderer, &mut marked, &scene);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(pixel(&stationary, x, y).map(|v| v.abs().to_bits()), [0; 2]);
                }
            }
            scene.view = camera(0.0, scale);
            let (_, panned, _) = frame(&device, &queue, &mut renderer, &mut target, &scene);
            let _marked = frame(&device, &queue, &mut marked_renderer, &mut marked, &scene);
            // Compare a sky pixel against independent pinhole projection at yaw +0.02
            // in the preceding view. Work in units of the actual drawn rectangle.
            let (x, y) = (w / 8, h / 8);
            let tangent = 0.5_f64.tan();
            let nx = 2.0 * (f64::from(x) + 0.5) / f64::from(w) - 1.0;
            let ny = 1.0 - 2.0 * (f64::from(y) + 0.5) / f64::from(h);
            let (sin, cos) = 0.02_f64.sin_cos();
            let ray_x = nx * tangent;
            let old_z = -sin * ray_x - cos;
            let expected = [
                ((cos * ray_x - sin) / (-old_z * tangent) - nx) * f64::from(w) / 2.0,
                (ny / -old_z - ny) * -f64::from(h) / 2.0,
            ];
            let actual = pixel(&panned, x, y);
            for axis in 0..2 {
                assert!(
                    (f64::from(actual[axis]) - expected[axis]).abs() < 0.04,
                    "{scale:?}: sky motion {actual:?} vs {expected:?}"
                );
            }
        }
    });
}
