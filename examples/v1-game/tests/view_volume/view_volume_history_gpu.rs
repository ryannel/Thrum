//! Submission-owned view-volume history on Metal.
#![cfg(feature = "proving")]
#[expect(dead_code, reason = "history reads raw volume and image bytes")]
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

fn assert_cold_frame(transition: &str, expected: &fixture::Witness, actual: &fixture::Witness) {
    assert!(
        expected.scatter == actual.scatter,
        "{transition}: cold scatter, terminal {:?}, expected {:?}",
        actual.terminal(),
        expected.terminal()
    );
    assert!(
        expected.transmission == actual.transmission,
        "{transition}: cold transmission"
    );
    assert!(expected.image == actual.image, "{transition}: cold image");
}

#[test]
fn metal_view_history_advances_with_submission_and_reseeds_on_reset() {
    let Some(gpu) = fixture::Gpu::open(7, "view volume history") else {
        return;
    };
    let mut scene = fixture::scene(false, true, false);
    let light = fixture::daylight(true, 1);
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
    let original_view = scene.view;
    scene.view = original_view.with_history_reset(true);
    assert_cold_frame(
        "camera cut",
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
    let next_epoch = fixture::daylight(true, 2);
    assert_cold_frame(
        "profile epoch",
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
    let nightglow = next_epoch.with_nightglow();
    let cold_night = {
        let (mut cold, mut cold_target) = gpu.prepare(&scene, nightglow);
        fixture::render(&gpu, &mut cold, &mut cold_target, &scene, nightglow)
    };
    let day = prepared
        .atmosphere_witness()
        .expect("day environment")
        .0
        .epoch;
    let lit_night = fixture::render(&gpu, &mut prepared, &mut target, &scene, nightglow);
    let night = prepared
        .atmosphere_witness()
        .expect("night environment")
        .0
        .epoch;
    assert_ne!(night, day);
    assert_cold_frame("nightglow enabled", &cold_night, &lit_night);
    let lit_day = fixture::render(&gpu, &mut prepared, &mut target, &scene, next_epoch);
    assert_eq!(
        prepared
            .atmosphere_witness()
            .expect("day environment")
            .0
            .epoch,
        day
    );
    assert_cold_frame("nightglow disabled", &fresh, &lit_day);
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
    assert_cold_frame("replacement device", &fresh, &replaced);
}

#[test]
fn metal_view_history_resets_only_on_sun_path_source_epoch_change() {
    let Some(gpu) = fixture::Gpu::open(9, "view volume Sun-path history") else {
        return;
    };
    let mut scene = fixture::scene(false, true, false);
    let light = fixture::daylight(true, 1);
    let (mut prepared, mut target) = gpu.prepare(&scene, light);
    let mut replacement = scene.clone();
    replacement.sun_optical_depth = Some(fixture::sun_path(0.0, -660.0, 2));
    let cold = {
        let (mut cold, mut cold_target) = gpu.prepare(&replacement, light);
        fixture::render(&gpu, &mut cold, &mut cold_target, &replacement, light)
    };
    let before = objects(&gpu.device);
    let shadowed = fixture::render(&gpu, &mut prepared, &mut target, &scene, light);
    assert!(shadowed.terminal()[0] < -0.1, "initial cloud shadow");
    assert_eq!(cold.terminal(), [0.0, 0.0, 0.0, 1.0]);

    // Advection clears the centre ray but preserves the provider's source epoch.
    scene.sun_optical_depth = Some(
        scene
            .sun_optical_depth
            .take()
            .expect("Sun path")
            .with_displacement([0.0, 1000.0])
            .expect("finite advection"),
    );
    let advected = fixture::render(&gpu, &mut prepared, &mut target, &scene, light);
    let residual = advected.terminal()[0];
    assert!(
        shadowed.terminal()[0] < residual && residual < -0.01,
        "advection keeps reprojected history, terminal scatter {residual}"
    );

    let live = fixture::render(&gpu, &mut prepared, &mut target, &replacement, light);
    assert_cold_frame("Sun-path provider replacement", &cold, &live);
    assert_eq!(
        objects(&gpu.device),
        before,
        "provider replacement reuses prepared GPU objects"
    );
}
