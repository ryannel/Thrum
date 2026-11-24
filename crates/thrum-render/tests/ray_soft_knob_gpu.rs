//! The runtime Sun penumbra knob uses prepared GPU objects on Metal.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn penumbra_toggle_preserves_the_hard_frame_and_allocations() {
    use glam::Vec3;
    use thrum_render::{synthetic::SunShadowMethod, timing::GpuTimings};
    use ray_shadow_support as fixture;
    use ray_support::support;

    let Some(gpu) = support::gate("soft Sun runtime toggle") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let mut scene = fixture::scene(Vec3::ZERO, &[
        fixture::box_at(1, Vec3::new(-0.75, 0.5, 1.0), Vec3::new(0.2, 0.5, 0.2)),
    ]);
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    let lighting = fixture::sun(Vec3::new(-0.4, 0.5, 0.7), 8000.0);
    let (mut hard, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
    assert_eq!(hard.shadow_selection().sun, SunShadowMethod::Traced);
    hard.update_frame(&queue, &mut target, &scene, lighting, 1.0).expect("hard update");
    let reference = support::submit_bytes(&device, &queue, &hard, &target);
    scene.sun_penumbra = true;
    let (mut soft, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
    assert_eq!(soft.shadow_selection().sun, SunShadowMethod::Traced);
    assert!(target.sun_penumbra_history_bytes() > 0);
    let mut timing = GpuTimings::new(&caps, &queue, 64).expect("timestamp batch");
    assert!(timing.is_enabled(), "Metal timestamp queries");
    let counts = || {
        let hal = device.get_internal_counters().hal;
        [hal.textures.read(), hal.texture_views.read(), hal.buffers.read(), hal.bind_groups.read(),
            hal.shader_modules.read(), hal.compute_pipelines.read(), hal.render_pipelines.read()]
    };
    let mut baseline = None;
    for enabled in [false, true, false, true] {
        soft.set_sun_penumbra_for_probe(enabled).expect("apply runtime knob");
        soft.update_frame(&queue, &mut target, &scene, lighting, 1.0).expect("update");
        let mut readback = target.readback(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        soft.encode_with_readback(&mut encoder, 1, &target, Some(&mut timing), &mut readback).expect("encode");
        timing.finish_render(1, &encoder).expect("finish timestamps");
        queue.submit([encoder.finish()]);
        soft.commit_frame();
        support::wait(&device);
        let bytes = readback.try_collect(1).expect("colour readback").expect("complete");
        let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(timing.resolve_if_ready(1, &mut resolve).expect("resolve timestamps"));
        queue.submit([resolve.finish()]);
        support::wait(&device);
        let passes = timing.try_collect(1).expect("valid timestamps").expect("complete timestamps");
        let visibility = passes.iter().filter(|pass| pass.name.starts_with("traced Sun visibility")).count();
        assert_eq!(visibility, if enabled { 4 } else { 0 });
        if !enabled { assert_eq!(bytes, reference, "penumbra off must match the hard frame"); }
        drop(readback);
        if let Some(baseline) = baseline {
            assert_eq!(counts(), baseline, "the toggle must not allocate");
        } else {
            baseline = Some(counts());
        }
    }
}
}
