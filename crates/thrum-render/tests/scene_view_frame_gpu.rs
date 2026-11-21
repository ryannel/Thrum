//! A view belongs to this recording: shared exposure and jitter, no sky or
//! late effects, and no temporal dependence through discard or replacement.
mod support;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use support::*;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectCadence,
        EffectGraph, SceneContribution, SceneContributions, SceneSurface, SecondaryViewRecord,
        SecondaryViewRequest, ViewAir,
    },
    present::SurfaceProfile,
    runtime::{FrameConfig, SurfaceOutput},
    synthetic::{FrameLighting, SyntheticScene},
};

struct Tag {
    pipeline: Option<wgpu::RenderPipeline>,
    draws: Arc<AtomicUsize>,
}
fn tag_pass() -> AfterOpaquePass {
    AfterOpaquePass {
        name: "frame colour tag".into(),
        kind: AfterOpaqueKind::Render,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: false,
        write_depth: false,
        refraction: false,
    }
}
impl SceneContribution for Tag {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: false,
            compute: false,
        }
    }
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            after_opaque: vec![tag_pass()],
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        assert!(surface.secondary_surfaces.is_empty());
        let device = surface.device;
        let input = tag_pass().input_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frame colour tag"),
            bind_group_layouts: &[Some(surface.view_layout), Some(&input)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frame colour tag"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("support/scene_view/frame_tag.wgsl").into(),
            ),
        });
        let targets = surface.color_formats.map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        self.pipeline = Some(device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("frame colour tag"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        }));
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
    fn draw_after_opaque(&self, _: usize, pass: &mut wgpu::RenderPass<'_>, _: &ContributionFrame) {
        self.draws.fetch_add(1, Ordering::Relaxed);
        pass.set_pipeline(self.pipeline.as_ref().expect("tag pipeline"));
        pass.draw(0..3, 0..1);
    }
}
fn surface(device: &wgpu::Device, extent: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("view lifecycle surface"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}
fn submit(host: &mut Host, device: &wgpu::Device, output: &wgpu::TextureView, extent: [u32; 2]) {
    assert!(
        host.advance(Duration::ZERO)
            .expect("advance")
            .encode_scene()
            .expect("encode")
            .submit(Some(&SurfaceOutput {
                view: output,
                extent
            }))
            .expect("submit")
            .submitted
    );
    thrum_test_support::wait::until("frame completion", Duration::from_secs(30), || {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .expect("completion");
        host.poll_completed().expect("sample")
    });
}
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the copied viewport and allocation dimensions are admitted positive integer extents"
)]
fn observed(
    host: &Host,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    control: &Arc<Mutex<Control>>,
    extent: [u32; 2],
    exposure: f32,
) -> SecondaryViewRecord {
    let state = control.lock().expect("control");
    let bytes = output(device, queue, &state.outputs[0]);
    let record: SecondaryViewRecord = bytemuck::pod_read_unaligned(&bytes[..RECORD_BYTES]);
    let main = state.main_frame.expect("upload frame");
    let base = host.source().view.view_proj_columns();
    let dx = (main.view_proj[2][0] - base[2][0]) / base[2][3];
    let dy = (main.view_proj[2][1] - base[2][1]) / base[2][3];
    assert!(
        dx != 0.0 || dy != 0.0,
        "the main frame must actually jitter"
    );
    let shift = glam::Mat4::from_cols(
        glam::Vec4::X,
        glam::Vec4::Y,
        glam::Vec4::Z,
        glam::Vec4::new(
            dx * (extent[0] as f32 / record.extent[0]),
            dy * (extent[1] as f32 / record.extent[1]),
            0.0,
            1.0,
        ),
    );
    let expected = shift * glam::Mat4::from_cols_array_2d(&record.unjittered_view_proj);
    assert_eq!(
        record.view_proj,
        expected.to_cols_array_2d(),
        "the same phase is scaled at the view's own viewport"
    );
    assert_ne!(record.view_proj, record.unjittered_view_proj);
    let mut covered = 0;
    let mut clear = 0;
    for index in 0..(bytes.len() - RECORD_BYTES) / 32 {
        let (colour, depth) = pixel(&bytes, index);
        if colour[3] == 0.0 {
            assert_eq!(colour, [0.0; 4], "a view has no sky, stars or late tag");
            clear += 1;
            continue;
        }
        assert_eq!(colour[3], 1.0);
        assert!(depth > 0.0 && depth < 1.0);
        covered += 1;
        for (channel, expected) in [exposure, exposure * 0.5, exposure * 0.25]
            .into_iter()
            .enumerate()
        {
            let half = half::f16::from_f32(expected);
            let step = half::f16::from_bits(half.to_bits() + 1).to_f32() - expected;
            assert!(
                (colour[channel] - expected).abs() <= step,
                "this frame's exposure {exposure}, colour {colour:?}"
            );
        }
    }
    assert!(
        covered > 3 && clear > 100,
        "whole opaque content and clear background in its first frame"
    );
    let centre =
        (record.extent[1] as usize / 2) * record.extent[2] as usize + record.extent[0] as usize / 2;
    assert_eq!(
        pixel(&bytes, centre).0[3],
        1.0,
        "the requested viewport is used immediately"
    );
    record
}

#[test]
fn the_first_asked_frame_is_whole_after_discard_capture_cut_resize_and_device_replacement() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary frame lifetime",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = camera(0.0, 2.0)
            .with_reconstruction(true)
            .with_celestial_basis([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
            .expect("celestial rotation");
        scene.instances[0].current[3] = [0.0, 0.0, 0.5, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
        scene.star_catalogue = Some(&[[0.2, 0.0, 0.979_795_9, 0.001]]);
        let light = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.0, 1.0, 0.0], 0.0, [1.0; 3], 0.0, 1)
                .expect("night")
                .with_nightglow(),
        );
        let profile = |extent| SurfaceProfile {
            format: wgpu::TextureFormat::Rgba8Unorm,
            extent,
        };
        let mut host = host_with_config(
            &capabilities,
            &queue,
            scene,
            light,
            FrameConfig {
                render_extent: [64, 64],
                surface: Some(profile([64, 64])),
            },
        )
        .await;
        let (observer, control) = Observer::new(1);
        let draws = Arc::new(AtomicUsize::new(0));
        host.set_scene_contribution(Box::new(SceneContributions::new(vec![
            Box::new(observer),
            Box::new(Tag {
                pipeline: None,
                draws: Arc::clone(&draws),
            }),
        ])))
        .expect("attach");
        let request_camera = aimed([0.0, -2.0, 2.0], [0.0, 2.0, -1.5])
            .with_exposure(16.0)
            .expect("ignored view exposure")
            .with_jitter_phase(31, [1, 1]);
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&request_camera, [16, 16], None, ViewAir::Camera)
                .expect("ask"),
        );
        let target = surface(&device, [64, 64]);
        submit(&mut host, &device, &target, [64, 64]);
        observed(&host, &device, &queue, &control, [64, 64], 1.0);
        assert!(
            draws.load(Ordering::Relaxed) > 0,
            "the main late effect actually runs"
        );
        let before = host.last_submission();
        let discarded = host
            .advance(Duration::ZERO)
            .expect("advance")
            .encode_scene()
            .expect("discarded encoding");
        let discarded_matrix = control
            .lock()
            .expect("control")
            .main_frame
            .expect("discard frame")
            .view_proj;
        drop(discarded);
        assert_eq!(before, host.last_submission());
        submit(&mut host, &device, &target, [64, 64]);
        assert_eq!(
            control
                .lock()
                .expect("control")
                .main_frame
                .expect("next frame")
                .view_proj,
            discarded_matrix,
            "discard preserves the jitter phase"
        );
        observed(&host, &device, &queue, &control, [64, 64], 1.0);
        host.capture_pixels().expect("detached capture");
        let captured = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        let captured: SecondaryViewRecord = bytemuck::pod_read_unaligned(&captured[..RECORD_BYTES]);
        assert_eq!(
            captured.view_proj, captured.unjittered_view_proj,
            "capture is unjittered"
        );
        submit(&mut host, &device, &target, [64, 64]);
        observed(&host, &device, &queue, &control, [64, 64], 1.0);
        host.source_mut().view = host
            .source()
            .view
            .with_history_epoch(9)
            .with_incident_exposure(0.0, 6.0, 6.0)
            .expect("frame storage exposure");
        submit(&mut host, &device, &target, [64, 64]);
        let storage_gain = 1024.0 * (0.25 / (1.2 * 64.0));
        observed(&host, &device, &queue, &control, [64, 64], storage_gain);
        let inputs = host.extent_prepare_inputs([96, 48]).expect("resize inputs");
        let candidate = std::thread::spawn(move || {
            inputs
                .build(&std::sync::atomic::AtomicBool::new(false))
                .expect("resize preparation")
                .expect("replacement extent")
        })
        .join()
        .expect("preparation thread");
        assert_eq!(
            control.lock().expect("control").handles[0].maximum_extent(),
            [64, 64],
            "preparation leaves the old view bound"
        );
        assert!(host.publish_extent(candidate));
        assert!(host.contribution_error().is_none());
        assert_eq!(
            control.lock().expect("control").handles[0].maximum_extent(),
            [96, 48],
            "resizing replaces the declared maximum"
        );
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&request_camera, [96, 48], None, ViewAir::Camera)
                .expect("resized ask"),
        );
        let target = surface(&device, [96, 48]);
        submit(&mut host, &device, &target, [96, 48]);
        observed(&host, &device, &queue, &control, [96, 48], storage_gain);
        let inputs = host.device_prepare_inputs().expect("replacement inputs");
        let (replacement, replacement_queue) = gpu.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let capabilities =
            selected_capabilities(&DeviceCapabilities::capture(&gpu.adapter, &replacement, 2));
        let mut candidate = inputs
            .prepare(&capabilities, &replacement_queue, Some(profile([96, 48])))
            .await
            .expect("replacement preparation");
        host.publish_device(&mut candidate).expect("publish device");
        assert_eq!(control.lock().expect("control").handles[0].generation(), 2);
        let target = surface(&replacement, [96, 48]);
        submit(&mut host, &replacement, &target, [96, 48]);
        observed(
            &host,
            &replacement,
            &replacement_queue,
            &control,
            [96, 48],
            storage_gain,
        );
    });
}
