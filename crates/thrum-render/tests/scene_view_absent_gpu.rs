//! Missing declarations add nothing; idle slots and changed requests allocate
//! nothing in ordinary submitted frames, and their timings report actual work.
mod support;

/// A scene with no view in use composes the same main shader whether its
/// contribution declares no view slot or declares one and leaves it idle, and
/// neither composition carries view text. Compared at one commit, for each of
/// the four air and shadow variants.
#[test]
fn an_idle_view_slot_leaves_every_main_shader_variant_unchanged() {
    use thrum_render::{
        atmosphere::AtmosphereFrame,
        capabilities::DeviceCapabilities,
        synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
    };
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "unchanged main shader",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let mut variants = Vec::new();
    for air in [false, true] {
        for shadow in [false, true] {
            let main_shader = |slots: usize| {
                pollster::block_on(async {
                    let mut scene = SyntheticScene::grid(1).expect("scene");
                    scene.view = support::camera(0.0, 2.0);
                    scene.sun_shadows = shadow;
                    let lighting = if air {
                        FrameLighting::Atmosphere(
                            AtmosphereFrame::new([0.5, 0.707, 0.5], 100_000.0, [1.0; 3], 0.0, 1)
                                .expect("air"),
                        )
                    } else {
                        FrameLighting::Diagnostic(SyntheticLight::default())
                    };
                    let mut host =
                        support::host_with_lighting(&capabilities, &queue, scene, lighting).await;
                    let (observer, control) = support::Observer::new(slots);
                    host.set_scene_contribution(Box::new(observer))
                        .expect(if slots == 0 {
                            "no declaration"
                        } else {
                            "idle slot declaration"
                        });
                    control.lock().expect("control").main_shader.clone()
                })
            };
            let empty = main_shader(0);
            let idle = main_shader(1);
            assert!(
                !empty.is_empty(),
                "air={air} shadow={shadow}: no main shader composed"
            );
            assert!(
                !empty.contains("secondary_view") && !idle.contains("secondary_view"),
                "air={air} shadow={shadow}: view text reached the main shader"
            );
            assert!(
                empty == idle,
                "air={air} shadow={shadow}: an idle view slot changed the main shader"
            );
            variants.push(empty);
        }
    }
    variants.sort();
    variants.dedup();
    assert_eq!(
        variants.len(),
        4,
        "the four air and shadow variants compose apart"
    );
}
#[cfg(feature = "creation-counters")]
use std::time::Duration;
#[cfg(feature = "creation-counters")]
use support::Host;
#[cfg(feature = "creation-counters")]
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir},
    runtime::FrameSample,
    synthetic::SyntheticScene,
};

#[cfg(feature = "creation-counters")]
fn frame(host: &mut Host, device: &wgpu::Device, surface: &wgpu::TextureView) -> FrameSample {
    assert!(
        host.advance(Duration::ZERO)
            .expect("advance")
            .encode_scene()
            .expect("ordinary serial frame")
            .submit(Some(&thrum_render::runtime::SurfaceOutput {
                view: surface,
                extent: [64, 64]
            }))
            .expect("submit scene")
            .submitted
    );
    thrum_test_support::wait::until(
        "submitted frame retirement",
        Duration::from_secs(30),
        || {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .expect("frame completion");
            host.poll_completed().expect("completed sample")
        },
    )
}
#[cfg(feature = "creation-counters")]
fn objects(device: &wgpu::Device) -> [isize; 11] {
    let hal = device.get_internal_counters().hal;
    [
        hal.buffers.read(),
        hal.textures.read(),
        hal.texture_views.read(),
        hal.samplers.read(),
        hal.render_pipelines.read(),
        hal.compute_pipelines.read(),
        hal.shader_modules.read(),
        hal.query_sets.read(),
        hal.bind_groups.read(),
        hal.bind_group_layouts.read(),
        hal.pipeline_layouts.read(),
    ]
}

#[cfg(feature = "creation-counters")]
#[test]
fn declarations_and_asks_leave_the_default_shader_and_main_frame_unchanged() {
    use support::*;
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary views absent",
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
        scene.sun_shadows = true;
        scene.camera_culling = thrum_render::synthetic::CullMode::Frustum;
        scene.meshes = Some(std::sync::Arc::new(
            thrum_render::synthetic::SyntheticMeshes {
                updates: None,
                meshes: Vec::new(),
                slots: vec![thrum_render::synthetic::SyntheticMeshChoice::Cube],
            },
        ));
        scene.view = camera(0.0, 2.0);
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
        scene.instances[0].current[3] = [0.0, 0.0, 0.5, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        let mut host = host_with_config(
            &capabilities,
            &queue,
            scene,
            thrum_render::synthetic::FrameLighting::Diagnostic(
                thrum_render::synthetic::SyntheticLight::default(),
            ),
            thrum_render::runtime::FrameConfig {
                render_extent: [64, 64],
                surface: Some(thrum_render::present::SurfaceProfile {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    extent: [64, 64],
                }),
            },
        )
        .await;
        let surface = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("submitted view absence surface"),
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
        let base_graph = format!("{:?}", host.scene_graph().expect("base graph"));
        let base_count = host.pipeline_creation_count().expect("creation counters");
        let (empty, empty_control) = Observer::new(0);
        host.set_scene_contribution(Box::new(empty))
            .expect("no declaration");
        assert_eq!(
            base_graph,
            format!("{:?}", host.scene_graph().expect("no-view graph")),
            "passes, resources and pipeline keys remain identical"
        );
        assert_eq!(
            base_count,
            host.pipeline_creation_count().expect("creation counters")
        );
        let base_shader = empty_control
            .lock()
            .expect("empty observer")
            .main_shader
            .clone();
        assert!(
            !base_shader.contains("secondary_view"),
            "no view shader is added to the main source"
        );
        let base = host.capture_pixels().expect("base pixels");
        let (observer, control) = Observer::new(1);
        host.set_scene_contribution(Box::new(observer))
            .expect("slot declaration");
        assert_eq!(base_shader, control.lock().expect("observer").main_shader);
        assert_eq!(base, host.capture_pixels().expect("declared idle pixels"));
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(
                &aimed([0.0, -2.0, 2.0], [0.0, 2.0, -1.5]),
                [64, 64],
                None,
                ViewAir::Camera,
            )
            .expect("ask"),
        );
        assert_eq!(base, host.capture_pixels().expect("asked pixels"));
        // Captures allocate their own detached targets. Use the prepared frame
        // slots below when asserting the absence of runtime object creation.
        control.lock().expect("control").requests[0] = None;
        frame(&mut host, &device, &surface);
        let before = objects(&device);
        assert!(
            before[0] > 0 && before[1] > 0,
            "Metal counters must be live"
        );
        let pipelines = host.pipeline_creation_count().expect("pipeline count");
        for request in [
            None,
            Some(aimed([0.0, -2.0, 2.0], [0.0, 2.0, -1.5])),
            Some(aimed([0.0, -3.0, 2.0], [0.2, 3.0, -1.5])),
            None,
        ] {
            control.lock().expect("control").requests[0] = request.map(|view| {
                SecondaryViewRequest::new(&view, [64, 64], None, ViewAir::Camera).expect("ask")
            });
            let sample = frame(&mut host, &device, &surface);
            let timings = sample.gpu_pass_ns.expect("Metal pass timings");
            let names = timings
                .passes()
                .iter()
                .filter(|pass| pass.name.starts_with("secondary view "))
                .map(|pass| pass.name.to_string())
                .collect::<Vec<_>>();
            assert_eq!(
                names.is_empty(),
                request.is_none(),
                "only asked slots have timings: {names:?}"
            );
            assert_eq!(
                names
                    .iter()
                    .any(|name| name.starts_with("secondary view Sun caster cull")),
                request.is_some(),
                "only an asked view compacts casters: {names:?}"
            );
            assert_eq!(
                before,
                objects(&device),
                "changing or idling the request creates no GPU object"
            );
            assert_eq!(
                pipelines,
                host.pipeline_creation_count().expect("pipeline count")
            );
        }
        assert_eq!(host.creation_report().violations, 0);
    });
}
