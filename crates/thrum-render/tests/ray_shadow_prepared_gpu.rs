//! Day, night and capture use only their generation's prepared pipelines.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod support;

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn shadow_pipelines_are_closed_when_each_generation_is_ready() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support as metal;
    use std::time::Duration;
    use thrum_render::{
        atmosphere::AtmosphereFrame,
        present::SurfaceProfile,
        runtime::{CreationPhase, FrameConfig, FrameHost, SurfaceOutput},
        synthetic::{FrameLighting, SunShadowMethod, SyntheticPointLight},
    };

    let Some(gpu) = metal::gate("traced preparation across generations") else {
        return;
    };
    let (first, queue, first_caps) = metal::device(&gpu, "ray-query", 1);
    let (second, second_queue, second_caps) = metal::device(&gpu, "ray-query", 2);
    let mut scene = fixture::scene(Vec3::ZERO, &[]);
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene.lamp_shadows = true;
    scene.local_lights = vec![SyntheticPointLight {
        position: [0.8, 1.8, 1.0],
        radius: 5.0,
        candela: [600.0; 3],
    }];
    scene.star_catalogue = Some(&[[0.0, 1.0, 0.0, 0.001]]);
    let lighting = |elevation| {
        FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.4, elevation, 0.8], 127_000.0, [1.0; 3], 4.0, 1)
                .expect("atmosphere")
                .with_nightglow(),
        )
    };
    let extent = [64; 2];
    let surface = SurfaceProfile {
        format: wgpu::TextureFormat::Rgba8Unorm,
        extent,
    };
    let source = support::Source {
        view: scene.view,
        lighting: lighting(0.55),
        instances: vec![([0.0, -0.1, 0.0], 0)],
        lights: scene.local_lights.clone(),
    };
    let mut host = pollster::block_on(FrameHost::new(
        &first_caps,
        &queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(surface),
        },
        support::World,
        source,
        scene,
        1.0,
    ))
    .expect("prepared scene");
    for (generation, device) in [(1, &first), (2, &second)] {
        if generation == 2 {
            let mut prepared = pollster::block_on(
                host.device_prepare_inputs()
                    .expect("replacement inputs")
                    .prepare(&second_caps, &second_queue, Some(surface)),
            )
            .expect("prepare replacement");
            host.publish_device(&mut prepared)
                .expect("publish replacement");
        }
        let ready = host.creation_report();
        assert!(ready.ready);
        assert_eq!(ready.generation, generation);
        let preparation = if generation == 1 {
            CreationPhase::Build
        } else {
            CreationPhase::Recovery
        };
        assert!(ready.pipelines.iter().all(|row| row.phase == preparation));
        assert!(ready.pipelines.iter().any(|row| row.render > 0));
        let view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("replacement shadow frame"),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent,
        };
        for (frame, elevation) in [0.55, -0.55, 0.30].into_iter().enumerate() {
            host.continuous_source_mut().lighting = lighting(elevation);
            assert!(
                host.drive(
                    Duration::from_millis((frame as u64 + 4 * generation) * 17),
                    Some(&output)
                )
                .expect("submit")
                .submitted
            );
            metal::wait(device);
            let sample = thrum_test_support::wait::until(
                "frame sample",
                Duration::from_secs(20),
                || host.poll_completed().expect("poll"),
            );
            assert_eq!(sample.shadow_selection.sun, SunShadowMethod::Traced);
            assert!(sample.shadow_selection.lamps && sample.shadow_selection.fallback.is_none());
            assert_eq!(
                host.capture_pixels().expect("capture variant").len(),
                64 * 64 * 4
            );
            let report = host.creation_report();
            assert_eq!(report.pipelines, ready.pipelines);
            assert_eq!(report.violations, 0);
        }
        println!(
            "Generation {generation}: {} creation sites at ready, unchanged after day, night and capture",
            ready.pipelines.len()
        );
    }
}
}
