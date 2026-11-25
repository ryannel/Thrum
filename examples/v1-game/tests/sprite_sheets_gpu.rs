//! Submitted Metal pixels for named frames, transactional residency and device replacement.

mod sprite_sheet_fixture;
use sprite_sheet_fixture as fixture;

use std::time::Duration;

use fixture::{ACTOR, BLUE, GREEN, RED, Rig, WHITE, art, open, reference, row, save};
use image::{Rgba, RgbaImage};
use thrum_asset_gpu::{CookedAssets, CookedConfig, MeshPlacement};

#[test]
fn last_copy_must_submit_and_world_frames_anchor_through_trim() {
    let Some(opened) = open("sprite sheets pivot and last copy") else {
        return;
    };
    let mut rig = Rig::new(opened);
    let absent = reference(None, false);
    thrum_test_support::wait::until(
        "record last copy without submitting it",
        Duration::from_secs(10),
        || {
            rig.record(true);
            assert!(rig.lease.offered().is_none());
            assert!(rig.stats.lock().expect("stats").drawn.is_none());
            (rig.stats.lock().expect("stats").recorded_last_copy == Some(1)).then_some(())
        },
    );
    let copy_frame = rig.draw();
    assert_eq!(copy_frame.rgba, absent);
    assert_eq!(
        copy_frame
            .sample
            .sprites
            .expect("sprite stats")
            .pending_sheet_rows,
        2
    );
    assert_eq!(
        rig.lease.offered().expect("offered after copy").generation,
        1
    );
    assert!(rig.stats.lock().expect("stats").drawn.is_none());

    let product = rig.product();
    let trims: Vec<_> = product
        .frames()
        .take(2)
        .map(|(_, frame)| frame.trim)
        .collect();
    assert_ne!(trims[0], trims[1]);
    for quarter in [false, true] {
        for flip in [false, true] {
            for (index, handle) in rig.handles.into_iter().enumerate() {
                let mut actor = row(rig.slot, handle, ACTOR);
                if quarter {
                    actor.rotation = std::f32::consts::FRAC_PI_2;
                }
                if flip {
                    actor.scale[0] = -1.0;
                }
                rig.host.source_mut().actor = actor;
                let image = rig.draw();
                assert_eq!(
                    image.rgba,
                    reference(Some((&art(RED, index == 1), quarter, flip)), true),
                    "frame {index}, quarter={quarter}, flip={flip}"
                );
                assert_eq!(image.sample.sprites.expect("stats").world_rows, 1);
            }
        }
    }
    let mut config = CookedConfig::new(
        rig.cooker.store().root().to_path_buf(),
        vec![MeshPlacement::new("arch", "sand")],
    );
    config.lease_sheet("actors");
    let assets = CookedAssets::new(config).expect("valid 3D config");
    let error = rig
        .host
        .set_scene_contribution(Box::new(assets))
        .expect_err("2D placement refused");
    assert!(
        error
            .to_string()
            .contains("2D scene refuses mesh placement 'arch'"),
        "{error}"
    );
}

#[test]
fn one_submitted_frame_switches_pixels_and_packing_without_recreating_the_graph() {
    let Some(opened) = open("sprite sheet atomic switch") else {
        return;
    };
    let mut rig = Rig::new(opened);
    let old = reference(Some((&art(RED, false), false, false)), true);
    let new = reference(Some((&art(GREEN, true), false, false)), true);
    assert_eq!(rig.ready(1).rgba, old);
    let old_offer = rig.lease.offered().expect("old offer");
    let graph = rig.host.scene_graph().expect("graph");
    let created = rig.host.creation_report().pipelines;
    let previous = rig.product();
    let old_rectangle = previous
        .frames()
        .find(|(name, _)| *name == "a")
        .expect("a")
        .1
        .origin;
    save(
        &rig.folder.join("0.png"),
        &RgbaImage::from_pixel(24, 6, Rgba(BLUE)),
    );
    save(&rig.folder.join("a.png"), &art(GREEN, true));
    assert_eq!(rig.publish(), 2);
    let edited = rig.product();
    let new_rectangle = edited
        .frames()
        .find(|(name, _)| *name == "a")
        .expect("a")
        .1
        .origin;
    assert_ne!(
        old_rectangle, new_rectangle,
        "edit repacks the sampled frame"
    );
    thrum_test_support::wait::until(
        "old pixels until copy submits",
        Duration::from_secs(10),
        || {
            assert_eq!(rig.draw().rgba, old);
            (rig.lease.offered().expect("offer").generation == 2).then_some(())
        },
    );
    rig.record(true);
    assert_eq!(rig.stats.lock().expect("stats").drawn, Some(1));
    assert_eq!(rig.draw().rgba, new);
    assert_eq!(rig.draw().rgba, new);
    let new_offer = rig.lease.offered().expect("new offer");
    assert_ne!(old_offer.products[0].texture, new_offer.products[0].texture);
    assert_eq!(old_offer.products[1].texture, new_offer.products[1].texture);
    let stats = rig.stats.lock().expect("stats");
    let [first, edit] = stats.swaps.as_slice() else {
        panic!("expected two submitted swaps")
    };
    assert!(edit.uploaded_bytes > 0);
    let old_actor_bytes = upload_bytes(previous.texture());
    let old_retained_bytes = upload_bytes(
        old_offer.products[1]
            .sheet
            .as_ref()
            .expect("retained sheet")
            .texture(),
    );
    assert_eq!(first.uploaded_bytes, old_actor_bytes + old_retained_bytes);
    assert_eq!(
        edit.uploaded_bytes,
        upload_bytes(edited.texture()),
        "only the changed sheet uploads"
    );
    assert_eq!(rig.host.creation_report().pipelines, created);
    let after = rig.host.scene_graph().expect("same graph");
    assert_eq!(
        graph.attachment_storage[0].resource,
        after.attachment_storage[0].resource
    );
}

fn upload_bytes(texture: &thrum_asset::Ktx2) -> u64 {
    texture
        .levels()
        .iter()
        .map(|level| {
            let row = level
                .bytes_per_row(texture.format())
                .next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
            u64::from(row)
                * u64::from(level.rows_per_image(texture.format()))
                * u64::from(texture.array_layers())
        })
        .sum()
}

#[test]
fn a_retained_handle_survives_repack_absence_and_restoration() {
    let Some(opened) = open("sprite sheet stable names") else {
        return;
    };
    let mut rig = Rig::new(opened);
    let alive = reference(Some((&art(RED, false), false, false)), true);
    assert_eq!(rig.ready(1).rgba, alive);
    save(
        &rig.folder.join("0.png"),
        &RgbaImage::from_pixel(20, 3, Rgba(WHITE)),
    );
    std::fs::remove_file(rig.folder.join("b.png")).expect("remove other frame");
    let description = rig.folder.join("sheet.json");
    std::fs::write(
        &description,
        r#"{"kind":"pixel","density":1,"pivots":{"a":[4,4]}}"#,
    )
    .expect("updated pivots");
    assert_eq!(rig.publish(), 2);
    assert_eq!(rig.ready(2).rgba, alive);
    std::fs::remove_file(rig.folder.join("a.png")).expect("remove requested frame");
    std::fs::write(&description, r#"{"kind":"pixel","density":1}"#)
        .expect("no missing source references");
    assert_eq!(rig.publish(), 3);
    let absent = rig.ready(3);
    assert_eq!(absent.rgba, reference(None, true));
    assert_eq!(absent.sample.sprites.expect("stats").missing_frame_rows, 1);
    save(&rig.folder.join("a.png"), &art(RED, false));
    // The default pivot of an 8×8 image is the same as the original authored pivot.
    assert_eq!(rig.publish(), 4);
    let restored = rig.ready(4);
    assert_eq!(restored.rgba, alive);
    assert_eq!(
        restored.sample.sprites.expect("stats").missing_frame_rows,
        0
    );
}

fn cooker_refusal(test: &str, reason: &str, inject: impl FnOnce(&Rig)) {
    let Some(opened) = open(test) else { return };
    let mut rig = Rig::new(opened);
    let previous = rig.ready(1).rgba;
    inject(&rig);
    let report = rig.cooker.cook_all().expect("failed save reported");
    assert!(report.published.is_none(), "{report:?}");
    assert_eq!(report.failed.len(), 1, "{report:?}");
    assert_eq!(report.failed[0].0, "actors");
    assert!(report.failed[0].1.contains(reason), "{report:?}");
    assert_eq!(rig.draw().rgba, previous);
}

#[test]
fn corrupt_png_keeps_the_last_good_sheet() {
    cooker_refusal("corrupt PNG", "a.png", |rig| {
        std::fs::write(rig.folder.join("a.png"), b"invalid PNG").expect("bad image");
    });
}

#[test]
fn oversize_frame_keeps_the_last_good_sheet() {
    cooker_refusal(
        "oversize frame",
        "a: frame is larger than a sheet page",
        |rig| {
            save(
                &rig.folder.join("a.png"),
                &RgbaImage::from_pixel(65, 2, Rgba(RED)),
            );
        },
    );
}

#[test]
fn missing_source_keeps_the_last_good_sheet() {
    cooker_refusal("missing image", "missing frame a", |rig| {
        std::fs::remove_file(rig.folder.join("a.png")).expect("missing image");
    });
}

fn residency_refusal(rig: &mut Rig, previous: &[u8], reason: &str) {
    assert_eq!(rig.publish(), 2);
    thrum_test_support::wait::until(
        "residency refuses the publication",
        Duration::from_secs(10),
        || {
            assert_eq!(rig.draw().rgba, previous);
            rig.stats
                .lock()
                .expect("stats")
                .errors
                .iter()
                .any(|error| error.contains(reason))
                .then_some(())
        },
    );
    assert_eq!(rig.stats.lock().expect("stats").drawn, Some(1));
}

#[test]
fn sheet_that_exceeds_the_device_layers_keeps_the_last_good_sheet() {
    let Some(opened) = open("device layer limit") else {
        return;
    };
    let mut rig = Rig::new(opened);
    let previous = rig.ready(1).rgba;
    let layers = rig.opened.device.limits().max_texture_array_layers;
    save(
        &rig.folder.join("grid.png"),
        &RgbaImage::from_pixel(layers + 1, 1, Rgba(RED)),
    );
    std::fs::write(
        rig.folder.join("sheet.json"),
        r#"{"kind":"pixel","density":1,"page_size":16,"grids":{"grid.png":[1,1]}}"#,
    )
    .expect("single frame per page");
    residency_refusal(
        &mut rig,
        &previous,
        "lease actors: unsupported KTX2 texture: extent or page count exceeds this device's texture limits",
    );
}

#[test]
fn failed_staging_fill_keeps_the_last_good_sheet() {
    let Some(opened) = open("failed staging fill") else {
        return;
    };
    let mut rig = Rig::new(opened);
    let previous = rig.ready(1).rgba;
    rig.fault.fail_next_fill();
    save(&rig.folder.join("a.png"), &art(GREEN, false));
    residency_refusal(&mut rig, &previous, "injected fill failure");
}

#[test]
fn replacement_device_draws_embedded_art_before_restaging_cooked_pages() {
    let Some(opened) = open("sprite sheet device replacement") else {
        return;
    };
    let mut rig = Rig::new(opened);
    let expected = reference(Some((&art(RED, false), false, false)), true);
    assert_eq!(rig.ready(1).rgba, expected);
    let old = rig.lease.offered().expect("old lease");
    rig.replacement();
    assert!(rig.lease.offered().is_none());
    let pending = rig.draw();
    assert_eq!(pending.rgba, reference(None, false));
    assert_eq!(pending.sample.sprites.expect("stats").pending_sheet_rows, 2);
    assert_eq!(rig.ready(1).rgba, expected);
    let current = rig.lease.offered().expect("restaged lease");
    for (old, current) in old.products.iter().zip(&current.products) {
        assert_eq!(current.device_generation, 2);
        assert_ne!(old.texture, current.texture);
        assert_eq!(old.hash, current.hash);
    }
}
