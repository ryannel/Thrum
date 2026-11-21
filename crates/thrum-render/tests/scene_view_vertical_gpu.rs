//! Admission is exact in declared coordinates and leaves refused slots idle.
mod support;
#[cfg(feature = "creation-counters")]
mod admission {
    use super::support::*;
    use std::time::Duration;
    use thrum_render::{
        capabilities::DeviceCapabilities,
        contribution::{SecondaryViewError, SecondaryViewRequest, ViewAir},
        runtime::{FrameConfig, FrameSample},
        synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
    };

    fn frame(host: &mut Host, device: &wgpu::Device, surface: &wgpu::TextureView) -> FrameSample {
        assert!(
            host.advance(Duration::ZERO)
                .expect("advance")
                .encode_scene()
                .expect("encode")
                .submit(Some(&thrum_render::runtime::SurfaceOutput {
                    view: surface,
                    extent: [64, 64]
                }))
                .expect("submit")
                .submitted
        );
        thrum_test_support::wait::until("frame retirement", Duration::from_secs(30), || {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .expect("complete");
            host.poll_completed().expect("retire")
        })
    }

    fn objects(device: &wgpu::Device) -> [isize; 11] {
        let h = device.get_internal_counters().hal;
        [
            h.buffers.read(),
            h.textures.read(),
            h.texture_views.read(),
            h.samplers.read(),
            h.render_pipelines.read(),
            h.compute_pipelines.read(),
            h.shader_modules.read(),
            h.query_sets.read(),
            h.bind_groups.read(),
            h.bind_group_layouts.read(),
            h.pipeline_layouts.read(),
        ]
    }

    #[test]
    fn exact_declared_vertical_admits_all_heights_and_refuses_horizontal_motion() {
        let Some(thrum_test_support::Opened {
            gpu, device, queue, ..
        }) = thrum_test_support::open(
            "vertical admission",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        )
        else {
            return;
        };
        pollster::block_on(async {
            let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
            let surface = device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("vertical admission surface"),
                    size: wgpu::Extent3d {
                        width: 64,
                        height: 64,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut scene = SyntheticScene::grid(1).expect("scene");
            scene.view = aimed([16.0, 10.0, 0.0], [-2.0, -1.7, -1.0]);
            let mut host = host_with_config(
                &cap,
                &queue,
                scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                FrameConfig {
                    render_extent: [64, 64],
                    surface: Some(thrum_render::present::SurfaceProfile {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        extent: [64, 64],
                    }),
                },
            )
            .await;
            let (observer, control) = Observer::new(1);
            control.lock().expect("control").allow_refusal = true;
            host.set_scene_contribution(Box::new(observer))
                .expect("slot");
            for main in [[16.0, 10.0, 0.0], [32.0, 10.0, 64.0]] {
                host.continuous_source_mut().view = aimed(main, [-2.0, -1.7, -1.0]);
                control.lock().expect("control").requests[0] = None;
                let idle = host.capture_pixels().expect("idle output");
                frame(&mut host, &device, &surface);
                let before = objects(&device);
                let pipelines = host.pipeline_creation_count().expect("counters");
                let mut cases = Vec::new();
                for height in [-500.0, -70.0, 0.0, 10.0, 90.0, 9000.0] {
                    for direction in [[0.0, 0.0, -1.0], [-2.0, 1.7, -1.0], [1.0, -0.4, 2.0]] {
                        cases.push(([main[0], height, main[2]], direction, true));
                    }
                }
                for axis in [0, 2] {
                    for delta in [None, Some(40_000.0)] {
                        let mut eye = main;
                        eye[axis] = delta.map_or_else(
                            || f32::from_bits(main[axis].to_bits() + 1),
                            |d| main[axis] + d,
                        );
                        cases.push((eye, [-2.0, 1.7, -1.0], false));
                    }
                }
                for (eye, direction, admitted) in cases {
                    let request = SecondaryViewRequest::new(
                        &aimed(eye, direction),
                        [64, 64],
                        None,
                        ViewAir::Camera,
                    )
                    .expect("checked camera");
                    control.lock().expect("control").requests[0] = Some(request);
                    let sample = frame(&mut host, &device, &surface);
                    let result = control.lock().expect("control").results[0];
                    assert_eq!(
                        result,
                        if admitted {
                            Ok(())
                        } else {
                            Err(SecondaryViewError::CameraOffVertical)
                        },
                        "declared={eye:?}"
                    );
                    let timings = sample.gpu_pass_ns.expect("Metal pass timings");
                    let names = timings
                        .passes()
                        .iter()
                        .filter(|p| p.name.starts_with("secondary view "))
                        .map(|p| p.name.to_string())
                        .collect::<Vec<_>>();
                    assert_eq!(!names.is_empty(), admitted, "{eye:?}: {names:?}");
                    assert_eq!(objects(&device), before, "ask creates no GPU object");
                    assert_eq!(host.pipeline_creation_count().expect("count"), pipelines);
                    assert_eq!(
                        idle,
                        host.capture_pixels().expect("asked output"),
                        "main bytes unchanged"
                    );
                    eprintln!(
                        "main={main:?} request={eye:?} direction={direction:?}: {result:?}, view passes={}",
                        names.len()
                    );
                }
            }
            assert_eq!(host.creation_report().violations, 0);
        });
    }
}
