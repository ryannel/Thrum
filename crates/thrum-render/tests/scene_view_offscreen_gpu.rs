//! View visibility is independent of both the main frustum and its wall.
mod support;
use std::{sync::Arc, time::Duration};
use support::*;
use thrum_render::{
    abi::InstanceRecord,
    capabilities::DeviceCapabilities,
    contribution::{SceneContributions, SecondaryViewRequest, ViewAir},
    synthetic::{
        CullMode, FrameLighting, SyntheticLight, SyntheticMeshChoice, SyntheticMeshes,
        SyntheticScene, SyntheticView,
    },
};

fn instance(slot: u32, material: u32, position: [f32; 3], half: [f32; 3]) -> InstanceRecord {
    let mut value = InstanceRecord::identity(slot, material);
    for axis in 0..3 {
        value.current[axis][axis] = half[axis];
        value.normal[axis][axis] = half[axis].recip();
        value.current[3][axis] = position[axis];
    }
    value.previous = value.current;
    value
}
#[test]
fn views_draw_behind_the_main_camera_and_its_occluding_wall_on_both_routes() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary offscreen instances",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        for bucket in [false, true] {
            for mode in [CullMode::Direct, CullMode::Frustum, CullMode::Occlusion] {
                for away in [true, false] {
                    let mut scene = SyntheticScene::grid(2).expect("scene");
                    scene.view = if away {
                        SyntheticView::perspective(
                            [0.0; 3],
                            [0.0, 0.0, -1.0],
                            [0.0, 1.0, 0.0],
                            1.0,
                            1.0,
                            0.1,
                            5000.0,
                        )
                        .expect("away")
                    } else {
                        camera(0.0, 0.0)
                    };
                    scene.camera_culling = mode;
                    scene.instances = vec![
                        instance(0, 0, [0.0, 0.0, 6.0], [1.0; 3]),
                        instance(1, 1, [0.0, 0.0, 3.0], [3.0, 3.0, 0.1]),
                    ];
                    scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
                    scene.materials[0].emissive_cutoff = [1024.0, 0.0, 0.0, 0.0];
                    scene.materials[1].base_rgba = [0.0, 0.0, 0.0, 1.0];
                    scene.materials[1].emissive_cutoff = [0.0, 1024.0, 0.0, 0.0];
                    scene.meshes = bucket.then(|| {
                        Arc::new(SyntheticMeshes {
                            updates: None,
                            meshes: Vec::new(),
                            slots: vec![SyntheticMeshChoice::Cube; 2],
                        })
                    });
                    let mut host = host_with_config(
                        &capabilities,
                        &queue,
                        scene,
                        FrameLighting::Diagnostic(SyntheticLight::default()),
                        thrum_render::runtime::FrameConfig {
                            render_extent: [64, 64],
                            surface: Some(thrum_render::present::SurfaceProfile {
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                extent: [64, 64],
                            }),
                        },
                    )
                    .await;
                    assert_eq!(host.camera_culling(), mode, "mode must be admitted");
                    let (observer, control) = Observer::new(1);
                    let (main, main_control) = MainObserver::new([64, 64]);
                    host.set_scene_contribution(Box::new(SceneContributions::new(vec![
                        Box::new(observer),
                        Box::new(main),
                    ])))
                    .expect("observers");
                    // Ordinary submitted frames prime Occlusion and expose its actual
                    // GPU list count, rather than mistaking a hidden pixel for culling.
                    let surface = device
                        .create_texture(&wgpu::TextureDescriptor {
                            label: Some("offscreen visibility surface"),
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
                    let mut counts = None;
                    for _ in 0..4 {
                        host.advance(Duration::ZERO)
                            .expect("advance")
                            .encode_scene()
                            .expect("encode")
                            .submit(Some(&thrum_render::runtime::SurfaceOutput {
                                view: &surface,
                                extent: [64, 64],
                            }))
                            .expect("submit");
                        device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: Some(Duration::from_secs(30)),
                            })
                            .expect("finish");
                        while let Some(sample) = host.poll_completed().expect("retire") {
                            counts = sample.camera_culling.or(counts);
                        }
                    }
                    if mode == CullMode::Occlusion && !away {
                        let counts = counts.expect("real Occlusion counts");
                        eprintln!("bucket={bucket}: wall occlusion counts={counts:?}");
                        assert_eq!(
                            counts.drawn, 1,
                            "the main Occlusion list must hide the subject"
                        );
                    }
                    let idle = host.capture_pixels().expect("idle");
                    let main_idle = output(
                        &device,
                        &queue,
                        &main_control.lock().expect("main").outputs[0],
                    );
                    control.lock().expect("control").requests[0] = Some(
                        SecondaryViewRequest::new(
                            &aimed([0.0, 8.0, 0.0], [0.0, -8.0, 6.0]),
                            [64, 64],
                            None,
                            ViewAir::Camera,
                        )
                        .expect("request"),
                    );
                    let asked = host.capture_pixels().expect("asked");
                    assert_eq!(idle, asked, "submitted main output");
                    assert_eq!(
                        main_idle,
                        output(
                            &device,
                            &queue,
                            &main_control.lock().expect("main").outputs[0]
                        ),
                        "stored main colour and depth"
                    );
                    let view = output(
                        &device,
                        &queue,
                        &control.lock().expect("control").outputs[0],
                    );
                    let red = (0..4096)
                        .filter(|&i| {
                            let (c, d) = pixel(&view, i);
                            c[0] > 0.9
                                && c[1] < 0.001
                                && c[3].to_bits() == 1.0_f32.to_bits()
                                && d > 0.0
                        })
                        .count();
                    eprintln!(
                        "view centre {:?}, main centre {:?}",
                        pixel(&view, 2080),
                        pixel(&main_idle, 2080)
                    );
                    assert!(
                        red > 100,
                        "own subject visible: bucket={bucket} mode={mode:?} away={away}: {red}"
                    );
                    assert!(
                        (0..4096).all(|i| pixel(&main_idle, i).0[0] < 0.9),
                        "main never sees the red subject"
                    );
                    eprintln!(
                        "bucket={bucket} mode={mode:?} away={away}: {red} subject pixels, main bytes unchanged"
                    );
                }
            }
        }
    });
}
