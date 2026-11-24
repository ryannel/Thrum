//! Plant incompatible values into the real Metal visibility history.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
fn replace_colour(queue: &wgpu::Queue, texture: &wgpu::Texture, texel: [u16; 4]) {
    let [width, height] = [texture.width(), texture.height()];
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(&vec![texel; (width * height) as usize]),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: Some(height),
        },
        texture.size(),
    );
}

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
fn replace_depth(queue: &wgpu::Queue, texture: &wgpu::Texture, depth: u32) {
    let [width, height] = [texture.width(), texture.height()];
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(&vec![[0, 0, depth, 33]; (width * height) as usize]),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 16),
            rows_per_image: Some(height),
        },
        texture.size(),
    );
}

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn published_soft_visibility_rejects_invalid_history_on_metal() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::synthetic::{OffscreenTarget, SunShadowMethod};

    let Some(gpu) = support::gate("submitted soft Sun history") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let mut scene = fixture::scene(Vec3::ZERO, &[]);
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene.sun_penumbra = true;
    let lighting = fixture::sun(Vec3::new(0.0, 1.0, 0.2), 8000.0);
    let (mut prepared, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
    assert_eq!(prepared.shadow_selection().sun, SunShadowMethod::Traced);
    let update = |prepared: &mut thrum_render::synthetic::PreparedSynthetic, target: &mut OffscreenTarget, scene: &thrum_render::synthetic::SyntheticScene| {
        prepared.update_frame(&queue, target, scene, lighting, 1.0).expect("update");
    };
    let sample = |bytes: &[u8]| fixture::sample(&scene.view, bytes, Vec3::ZERO);
    update(&mut prepared, &mut target, &scene);
    let hard = support::submit_bytes(&device, &queue, &prepared, &target);
    let lit = sample(&hard);
    assert!(lit[0] > 32);
    prepared.commit_frame();
    let read = prepared.sun_penumbra_side_for_probe();
    let [values, _] = target.sun_penumbra_textures_for_probe(read).expect("soft history");

    // A soft edge with distinct passed and stopped values. The flat floor is
    // outside the band and must take the darker passed value.
    replace_colour(&queue, values, [0, 0x3a00, 0, 0]);
    update(&mut prepared, &mut target, &scene);
    let before = sample(&support::submit_bytes(&device, &queue, &prepared, &target));
    assert!(before[0] < lit[0], "must consume the planted soft edge, got {before:?} vs {lit:?}");
    assert_eq!(prepared.sun_penumbra_side_for_probe(), read, "rendering alone cannot commit");
    let other = target.sun_penumbra_textures_for_probe(1 - read).expect("alternate history")[0];
    let settled = thrum_test_support::readback::texture(&device, &queue, other,
        thrum_test_support::readback::Mip::level(0), ..);
    assert_eq!(settled.as_chunks::<8>().0[96 * 192 + 96], [0, 60, 0, 60, 0, 60, 0, 60]);
    prepared.commit_frame();
    update(&mut prepared, &mut target, &scene);
    assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), lit,
        "the pass settles the previously soft soil in one submission");
    prepared.commit_frame();

    let read = prepared.sun_penumbra_side_for_probe();
    let [values, _] = target.sun_penumbra_textures_for_probe(read).expect("soft history");
    replace_colour(&queue, values, [0, 0, 0x3c00, 0x3c00]);
    update(&mut prepared, &mut target, &scene);
    assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), lit,
        "a settled dark predecessor cannot override a clear centre ray");
    prepared.commit_frame();

    let read = prepared.sun_penumbra_side_for_probe();
    let [values, depths] = target.sun_penumbra_textures_for_probe(read).expect("soft history");
    replace_colour(&queue, values, [0, 0, 0, 0]);
    replace_depth(&queue, depths, 0);
    update(&mut prepared, &mut target, &scene);
    assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), lit,
        "the traced visibility pass left out this predecessor texel");
    prepared.commit_frame();

    let read = prepared.sun_penumbra_side_for_probe();
    let [values, depths] = target.sun_penumbra_textures_for_probe(read).expect("soft history");
    replace_colour(&queue, values, [0, 0, 0, 0]);
    let clip = glam::Mat4::from_cols_array_2d(&scene.view.view_proj_columns()) * Vec3::ZERO.extend(1.0);
    let floor_depth = (clip.z / clip.w).to_bits();
    replace_depth(&queue, depths, floor_depth.wrapping_add(0x0020_0000));
    update(&mut prepared, &mut target, &scene);
    assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), lit,
        "a mismatching nonzero predecessor depth rejects the planted edge");
    prepared.commit_frame();

    let read = prepared.sun_penumbra_side_for_probe();
    let [values, _] = target.sun_penumbra_textures_for_probe(read).expect("soft history");
    replace_colour(&queue, values, [0, 0, 0, 0]);
    let mut cut = scene.clone();
    cut.view = cut.view.with_history_reset(true);
    update(&mut prepared, &mut target, &cut);
    assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), lit,
        "a cut rejects an otherwise matching predecessor");

    // An abandoned command may write the alternate side only if it submits.
    // Reprepare the regular view but drop the encoder before submitting it.
    update(&mut prepared, &mut target, &scene);
    let mut discarded = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared.encode(&mut discarded, 1, &target, None).expect("encode unsubmitted frame");
    drop(discarded);
    update(&mut prepared, &mut target, &scene);
    assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), before,
        "the abandoned frame did not replace the planted predecessor");

    let mut resized = OffscreenTarget::new(&device, &prepared, 208, 208).expect("new target extent");
    for side in [0, 1] {
        let [values, depths] = resized.sun_penumbra_textures_for_probe(side).expect("new history");
        replace_colour(&queue, values, [0, 0, 0, 0]);
        replace_depth(&queue, depths, floor_depth);
    }
    update(&mut prepared, &mut resized, &scene);
    let resized_bytes = support::submit_bytes(&device, &queue, &prepared, &resized);
    let new_index = (104 * 208 + 104) * 4;
    assert!(resized_bytes[new_index] > 32, "an extent change cannot show the planted edge");

    let (new_device, new_queue, new_caps) = support::device(&gpu, "ray-query", 2);
    let (mut replacement, mut new_target) = fixture::prepare(&new_device, &new_caps, &new_queue, &scene, lighting);
    assert_eq!(replacement.shadow_selection().sun, SunShadowMethod::Traced);
    for side in [0, 1] {
        let [values, depths] = new_target.sun_penumbra_textures_for_probe(side).expect("replacement history");
        replace_colour(&new_queue, values, [0, 0, 0, 0]);
        replace_depth(&new_queue, depths, floor_depth);
    }
    replacement.update_frame(&new_queue, &mut new_target, &scene, lighting, 1.0).expect("replacement update");
    assert_eq!(sample(&support::submit_bytes(&new_device, &new_queue, &replacement, &new_target)), lit,
        "device replacement rejects the planted predecessor");

}
}

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn terminator_uses_the_planted_passage_value_and_settled_mark() {
    use glam::Vec3;
    use ray_shadow_support::{self as fixture, terminator::Sheet};
    use ray_support::support;
    use thrum_render::synthetic::SunShadowMethod;

    let Some(gpu) = support::gate("soft Sun terminator history channels") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let receiver = Vec3::new(15.0_f32.to_radians().cos(), 0.23, 0.0);
    let (y, x) = 94.0_f32.to_radians().sin_cos();
    let sun = Vec3::new(x, y, 0.0);
    let lighting = fixture::sun(sun, 8000.0);
    let joined = Sheet { distance: 3.0, joined: true, reverse: true };
    let mut clear_centre: Option<[u8; 3]> = None;
    for (panel, open_rim) in [(None, false), (Some(joined), true)] {
        let mut scene = fixture::terminator::scene(receiver, sun, panel, open_rim);
        scene.sun_penumbra = true;
        let (mut prepared, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
        assert_eq!(prepared.shadow_selection().sun, SunShadowMethod::Traced);
        let sample = |bytes: &[u8]| fixture::sample(&scene.view, bytes, receiver);
        prepared.update_frame(&queue, &mut target, &scene, lighting, 1.0).expect("first update");
        let centre = sample(&support::submit_bytes(&device, &queue, &prepared, &target));
        prepared.commit_frame();
        if open_rim {
            assert!(centre[0] < clear_centre.expect("clear centre")[0], "joined sheet refuses the passage");
        } else {
            clear_centre = Some(centre);
        }

        let read = prepared.sun_penumbra_side_for_probe();
        let [values, _] = target.sun_penumbra_textures_for_probe(read).expect("submitted history");
        replace_colour(&queue, values, [0, 0x3c00, 0, 0]);
        prepared.update_frame(&queue, &mut target, &scene, lighting, 1.0).expect("planted update");
        let selected = sample(&support::submit_bytes(&device, &queue, &prepared, &target));
        if open_rim {
            assert!(selected[0] > centre[0], "refused passage reads stopped even when it rounded to one");
            let [values, _] = target.sun_penumbra_textures_for_probe(read).expect("same history");
            replace_colour(&queue, values, [0, 0x3c00, 0, 0x3c00]);
            prepared.update_frame(&queue, &mut target, &scene, lighting, 1.0).expect("settled update");
            assert_eq!(sample(&support::submit_bytes(&device, &queue, &prepared, &target)), centre,
                "a settled lit value yields to the blocked centre ray");
        } else {
            assert!(selected[0] < centre[0], "the granted passage reads passed");
        }
    }
}
}
