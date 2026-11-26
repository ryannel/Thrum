//! The submerged surface uses the same temporal attachments as the upper surface.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn underside_writes_depth_motion_and_reactivity_without_a_cut() {
    use glam::{Mat4, Vec4};
    use support::optics::{EXTENT, Rig, attach, camera, sun, water_with_profile};
    use thrum_render::synthetic::DisplayOverlay;

    let Some(gpu) = thrum_test_support::gpu("underwater frame attachments", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = |y: f32| camera([0.0, y, 0.0], [0.0, y + 0.5, 1.0], 0.8, 1.0 / 32768.0)
        .with_temporal_inputs(true)
        .with_display_overlay(DisplayOverlay::TemporalInputs);
    let above = view(0.2);
    let mut rig = Rig::new(&gpu, &above, sun(0.0, 0.0), &[], None);

    let disabled = water_with_profile(None, true, false);
    let disabled_report = disabled.report();
    attach(&mut rig, disabled);
    assert!(!disabled_report.lock().expect("prepared water")
        .resources.iter().any(|(name, _)| name.contains("lens")));
    rig.host.set_timing_requested(true);
    let dry = rig.frame();
    if let Some(timings) = dry.gpu_pass_ns {
        assert!(!timings.passes().iter().any(|pass| pass.name.contains("water gather")));
    }
    let dry_creations = rig.host.pipeline_creation_count();

    let enabled = water_with_profile(None, true, true);
    attach(&mut rig, enabled);
    rig.frame();
    let creations = rig.host.pipeline_creation_count();
    if let Some((dry, wet)) = dry_creations.zip(creations) {
        assert!(wet > dry, "preparing the enabled profile builds the lens pipeline");
    }

    let mut previous = above;
    for y in [-0.06, -0.2, -0.3] {
        let next = view(y);
        rig.host.continuous_source_mut().view = next;
        rig.frame();
        let witness = rig.witness.as_ref().expect("submitted attachments");
        let bytes = thrum_test_support::readback::texture(
            &rig.device, &rig.queue, witness, thrum_test_support::readback::Mip::level(0), ..
        );
        let [width, height] = EXTENT;
        let pixel = [width / 2, height / 2];
        let offset = ((pixel[1] * width + pixel[0]) * 8) as usize;
        let actual: [f32; 4] = std::array::from_fn(|i| {
            support::half_value(u16::from_le_bytes([bytes[offset + 2 * i], bytes[offset + 2 * i + 1]]))
        });
        let current_matrix = Mat4::from_cols_array_2d(&next.view_proj_columns());
        let previous_matrix = Mat4::from_cols_array_2d(&previous.view_proj_columns());
        let clip = Vec4::new(
            (pixel[0] as f32 + 0.5).mul_add(2.0 / width as f32, -1.0),
            (pixel[1] as f32 + 0.5).mul_add(-2.0 / height as f32, 1.0),
            1.0, 1.0,
        );
        let h = current_matrix.inverse() * clip;
        let eye = Vec4::new(0.0, y, 0.0, 1.0);
        let ray = h / h.w - eye;
        let intersection = eye - y * ray / ray.y;
        let now = current_matrix * intersection;
        let old = previous_matrix * intersection;
        let depth = now.z / now.w;
        let motion = (old / old.w - now / now.w) * Vec4::new(width as f32 * 0.5, -(height as f32) * 0.5, 0.0, 0.0);
        assert!((0.0..=1.0).contains(&depth));
        eprintln!("Metal at {y:.2} m: inputs {actual:?}, depth {depth:.5}, motion {motion:?}");
        assert!(actual[0] > 0.99);
        assert!((actual[1] - depth).abs() < 0.001);
        assert!((actual[2] - motion.x).abs() < 0.2 && (actual[3] - motion.y).abs() < (motion.y.abs() * 0.001).max(0.2));
        previous = next;
    }
    assert_eq!(rig.host.pipeline_creation_count(), creations);
}
}
