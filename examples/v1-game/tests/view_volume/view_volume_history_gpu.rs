//! Submission-owned view-volume history on Metal.
mod fixture;

use thrum_render::{atmosphere::VIEW_VOLUME_SETTLE_FRAMES, synthetic::FrameLighting};

fn objects(device: &wgpu::Device) -> [isize; 5] {
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
fn metal_view_history_advances_with_submission_and_reseeds_on_reset() {
    let Some(gpu) = fixture::Gpu::open(7, "view volume history") else {
        return;
    };
    let mut scene = fixture::scene(false, true, false);
    let light = fixture::light(true, 1);
    let (mut prepared, mut target) = gpu.prepare(&scene, light);
    let before = objects(&gpu.device);
    let fresh = fixture::render(&gpu, &mut prepared, &mut target, &scene, light);
    assert_eq!(
        prepared
            .view_volume_witness()
            .expect("validated fixture data")
            .1,
        1
    );

    let mut discarded = scene.clone();
    discarded.view = fixture::view([40.0, 2.0, 0.0]);
    prepared
        .update_frame(
            &gpu.queue,
            &mut target,
            &discarded,
            FrameLighting::Atmosphere(light),
            1.0,
        )
        .expect("discarded pose");
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, gpu.caps.generation(), &target, None)
        .expect("unsubmitted volume");
    drop(encoder);
    assert_eq!(
        prepared
            .view_volume_witness()
            .expect("validated fixture data")
            .1,
        1
    );
    let equal = |expected: &fixture::Witness, actual: &fixture::Witness| {
        assert!(
            expected.scatter == actual.scatter
                && expected.transmission == actual.transmission
                && expected.image == actual.image,
            "reset must fill the same volume that a cold frame fills"
        );
    };
    let original_view = scene.view;
    scene.view = original_view.with_history_reset(true);
    equal(
        &fresh,
        &fixture::render(&gpu, &mut prepared, &mut target, &scene, light),
    );
    assert_eq!(
        prepared
            .view_volume_witness()
            .expect("validated fixture data")
            .1,
        2
    );
    scene.view = original_view;
    let next_epoch = fixture::light(true, 2);
    equal(
        &fresh,
        &fixture::render(&gpu, &mut prepared, &mut target, &scene, next_epoch),
    );
    let mut last = fresh.terminal();
    for frame in 1..=VIEW_VOLUME_SETTLE_FRAMES {
        let next = fixture::render(&gpu, &mut prepared, &mut target, &scene, next_epoch).terminal();
        if frame == VIEW_VOLUME_SETTLE_FRAMES {
            for (old, new) in last[..3].iter().zip(&next[..3]) {
                assert!(
                    (old - new).abs() <= 0.025 * new.abs(),
                    "held shadow settles within the declared frame count"
                );
            }
        }
        last = next;
    }
    assert_eq!(
        objects(&gpu.device),
        before,
        "live frames keep prepared GPU object counts"
    );

    let replacement =
        fixture::Gpu::open(8, "replacement view volume generation").expect("second Metal device");
    let (mut prepared, mut target) = replacement.prepare(&scene, next_epoch);
    assert_eq!(
        prepared
            .view_volume_witness()
            .expect("validated fixture data")
            .1,
        0
    );
    let replaced = fixture::render(&replacement, &mut prepared, &mut target, &scene, next_epoch);
    equal(&fresh, &replaced);
}
