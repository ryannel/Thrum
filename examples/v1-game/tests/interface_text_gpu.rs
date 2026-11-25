//! Qualify indexed glyph drawing and transactional font residency on submitted Metal frames.

mod font_gpu_fixture;
use font_gpu_fixture as rig;

#[expect(
    dead_code,
    reason = "reuse the small simulated world from the sheet publication fixture"
)]
mod sprite_sheet_fixture;

mod metal {
    use std::time::Duration;

    use thrum_render::sprite::SpriteRow;
    use thrum_test_support::Scratch;
    use thrum_ui::TextRun;

    use crate::rig::{self, EXTENT, Reference, Rig};

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "requires Metal for its submitted image comparison"
    )]
    fn cooked_texels_match_at_all_scales_and_fractional_origins() {
        let scratch = Scratch::new("interface-text-reference");
        let cooker = rig::cook(scratch.path(), false);
        let outline = rig::font(&cooker, "type");
        let pixel = rig::font(&cooker, "pixel");
        for recipe in ["portable", "apple-tile"] {
            let gpu = rig::open(recipe);
            for scale in 1..=3 {
                let mut host = Rig::new(&gpu, &cooker, scale);
                host.ready();
                host.host.source_mut().world_rows.push(SpriteRow {
                    scale: [80.0, 48.0],
                    tint: [0.045, 0.21, 0.43, 1.0],
                    ..SpriteRow::new(0, [-40.0, 24.0])
                });
                host.host.source_mut().drawing = false;
                let background = host.draw().rgba;
                host.host.source_mut().drawing = true;
                let mut observed_starts = [false; 2];
                for subpixel in 0..64 {
                    let x = 14.0 + f64::from(subpixel) / 64.0;
                    host.host.source_mut().origin = [x, 4.0];
                    host.host.source_mut().value = 0;
                    let frame = host.draw();
                    let mut expected = Reference::new(background.clone());
                    let strike = outline.strike(12.0, scale).expect("exact scale");
                    let snapped = 14 + i32::from(subpixel >= 32);
                    observed_starts[usize::from(subpixel >= 32)] = true;
                    rig::outline_av(&mut expected, strike, [snapped, 4]);
                    rig::grid_rows(&mut expected, &pixel, scale, false, 0);
                    if let Some(at) = expected.first_difference(&frame.rgba) {
                        panic!(
                            "byte {at}: actual {}, expected {}",
                            frame.rgba[at], expected.rgba[at]
                        );
                    }
                    if subpixel == 0 {
                        let opaque = expected
                            .rgba
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(background.as_chunks::<4>().0)
                            .position(|(glyph, background)| {
                                *glyph == [255; 4] && glyph != background
                            })
                            .expect("opaque white glyph texel")
                            * 4;
                        let mut bad = expected.rgba.clone();
                        bad[opaque] -= 1;
                        assert_eq!(expected.first_difference(&bad), Some(opaque));
                    }
                    if scale != 1 && subpixel == 0 {
                        let mut wrong = Reference::new(background.clone());
                        rig::outline_at_wrong_scale(&mut wrong, &outline, scale);
                        rig::grid_rows(&mut wrong, &pixel, scale, false, 0);
                        assert!(
                            wrong.first_difference(&frame.rgba).is_some(),
                            "a resampled strike must fail"
                        );
                    }
                }
                assert_eq!(observed_starts, [true; 2]);
                println!("{recipe} scale {scale}: 64 subpixel origins and authored pixel grid");
            }
        }
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "requires Metal for its submitted image comparison"
    )]
    fn world_frame_rows_use_pixel_blocks_and_sort_between_sprites() {
        let scratch = Scratch::new("interface-text-world");
        let cooker = rig::cook(scratch.path(), false);
        let gpu = rig::open("portable");
        let mut host = Rig::new(&gpu, &cooker, 1);
        host.ready();
        host.host.source_mut().drawing = false;
        let background = host.draw().rgba;
        let font = rig::font(&cooker, "pixel");
        let run = TextRun::new(&font, "AB", rig::options(8.0, 1, [24, 8])).expect("world run");
        // At two output pixels per world unit, world [0,0] is at the output centre.
        let mut rows: Vec<_> = run
            .rows(1, [0.0, 0.0], [1.0; 4], [0.0; 4])
            .map(|mut row| {
                row.position[1] = -row.position[1];
                row.layer = 2;
                row.sort = Some(1.0);
                row
            })
            .collect();
        let panel = |position, layer, sort, tint| SpriteRow {
            layer,
            sort: Some(sort),
            tint,
            scale: [8.0; 2],
            ..SpriteRow::new(0, position)
        };
        rows.insert(0, panel([0.0, 0.0], 2, 2.0, [1.0, 0.0, 0.0, 1.0]));
        rows.push(panel([0.0, 0.0], 1, 5.0, [0.0, 0.0, 1.0, 1.0]));
        host.host.source_mut().world_rows = rows;
        let image = host.draw();
        let [cx, cy] = EXTENT.map(|side| usize::try_from(side / 2).expect("extent"));
        let mut expected = Reference::new(background);
        let strike = font.strike(8.0, 1).expect("pixel strike");
        let glyph = strike.glyph('B').expect("B frame");
        rig::glyph(
            &mut expected,
            strike,
            glyph.frame,
            [
                i32::try_from(cx + 16).expect("centre"),
                i32::try_from(cy + 14).expect("centre"),
            ],
            2,
        );
        for y in cy..cy + 16 {
            for x in cx..cx + 16 {
                let offset = (y * EXTENT[0] as usize + x) * 4;
                expected.rgba[offset..offset + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
        assert_eq!(image.rgba, expected.rgba);
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "requires Metal for its submitted image comparison"
    )]
    fn submitted_scale_switch_waits_for_copies_and_retires_the_old_sheet() {
        let scratch = Scratch::new("interface-text-scale");
        let cooker = rig::cook(scratch.path(), false);
        let gpu = rig::open("portable");
        let mut host = Rig::new(&gpu, &cooker, 1);
        let references = [1, 2, 3].map(|scale| {
            let mut fresh = Rig::new(&gpu, &cooker, scale);
            fresh.ready();
            fresh.host.source_mut().value = 0;
            fresh.draw()
        });
        host.ready();
        for (old, new) in [(1, 2), (2, 3), (3, 1)] {
            assert!(host.scale.set(new));
            let old_index = (old - 1) as usize;
            let new_index = (new - 1) as usize;
            thrum_test_support::wait::until(
                "last scale copy is recorded and discarded",
                Duration::from_secs(8),
                || {
                    let image = host.record_when(|stats| stats.recorded_last_copy.is_some());
                    assert_eq!(host.drawn_scale(), old);
                    image.map_or(Some(()), |image| {
                        assert_eq!(image.rgba, references[old_index].rgba);
                        None
                    })
                },
            );
            let last_copy = host.draw();
            assert_eq!(last_copy.rgba, references[old_index].rgba);
            assert_eq!(host.drawn_scale(), old);
            let switch = host.draw();
            assert_eq!(switch.rgba, references[new_index].rgba);
            assert_ne!(switch.rgba, references[old_index].rgba);
            assert_eq!(host.drawn_scale(), new);
            let bytes = rig::resident_bytes(&rig::font(&cooker, "type"), new)
                + rig::resident_bytes(&rig::font(&cooker, "pixel"), 1);
            assert_eq!(host.stats.lock().expect("stats").resident_font_bytes, bytes);
            println!("scale {old}→{new}, {bytes} committed texture bytes");
        }
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "requires Metal for its submitted image comparison"
    )]
    fn edits_switch_whole_on_submission_and_invalid_edits_keep_the_last_layout() {
        let scratch = Scratch::new("interface-text-edit");
        let mut live = rig::cook(scratch.path(), false);
        let edited = rig::cook(scratch.path(), true);
        let gpu = rig::open("portable");
        let references = [&live, &edited].map(|cooker| {
            let mut fresh = Rig::new(&gpu, cooker, 1);
            fresh.ready();
            [0, 1].map(|value| {
                fresh.host.source_mut().value = value;
                fresh.draw().rgba
            })
        });
        assert_ne!(references[0], references[1]);
        let mut host = Rig::new(&gpu, &live, 1);
        host.ready();
        rig::edit_pixel(&scratch.join("live"), 12, true);
        assert_eq!(live.cook_all().expect("saved font").failed, []);
        let mut value = 0usize;
        let mut before = 0;
        thrum_test_support::wait::until(
            "new font input is offered",
            Duration::from_secs(8),
            || {
                host.host.source_mut().value = value;
                let image = host.draw();
                assert_eq!(image.rgba, references[0][value]);
                assert_ne!(image.rgba, references[1][value]);
                before += 1;
                value ^= 1;
                (host.lease.sheet_publication().expect("offer").generation == 2).then_some(())
            },
        );
        host.record(true);
        assert_eq!(host.stats.lock().expect("stats").drawn, Some(1));
        assert!(host.lease.font_candidate().is_some());
        for value in [value, value ^ 1] {
            host.host.source_mut().value = value;
            let image = host.draw();
            assert_eq!(image.rgba, references[1][value]);
            assert_ne!(image.rgba, references[0][value]);
        }
        assert_eq!(host.stats.lock().expect("stats").drawn, Some(2));
        assert!(host.lease.font_candidate().is_none());
        let description = scratch.join("live/type/font.json");
        std::fs::write(&description, b"{bad json").expect("uncookable edit");
        let report = live.cook_all().expect("report source refusal");
        assert!(report.failed[0].1.contains("font.json"));
        assert!(report.published.is_none());
        assert_eq!(host.draw().rgba, references[1][value ^ 1]);
        let type_template =
            std::fs::read_to_string(rig::fixture().join("type/font.json")).expect("type source");
        let stripped = type_template.replace("{\"points\": 12},", "");
        std::fs::write(description, stripped).expect("remove a live size");
        assert!(
            live.cook_all()
                .expect("publish product without live size")
                .published
                .is_some()
        );
        thrum_test_support::wait::until("named refusal", Duration::from_secs(8), || {
            host.host.source_mut().value = value;
            assert_eq!(host.draw().rgba, references[1][value]);
            value ^= 1;
            host.stats
                .lock()
                .expect("stats")
                .errors
                .iter()
                .any(|reason| reason.contains("font size 12"))
                .then_some(())
        });
        host.draw();
        assert!(host.lease.font_candidate().is_none());
        assert!(host.host.source().next.is_none());
        let missing_scale = type_template.replace(r#""sizes""#, r#""scales": [2,3], "sizes""#);
        std::fs::write(scratch.join("live/type/font.json"), missing_scale)
            .expect("remove the live scale");
        assert!(
            live.cook_all()
                .expect("publish product without live scale")
                .published
                .is_some()
        );
        thrum_test_support::wait::until("named scale refusal", Duration::from_secs(8), || {
            host.host.source_mut().value = value;
            assert_eq!(host.draw().rgba, references[1][value]);
            value ^= 1;
            host.stats
                .lock()
                .expect("stats")
                .errors
                .iter()
                .any(|reason| reason.contains("font scale 1"))
                .then_some(())
        });
        assert!(host.lease.font_candidate().is_none());
        println!(
            "{before} submissions before edit, two after, one discarded switch, and named source, size and scale refusals"
        );
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "requires Metal for its submitted image comparison"
    )]
    fn newly_visible_run_refuses_the_ready_edit_before_submit() {
        let scratch = Scratch::new("interface-text-late-refusal");
        let mut cooker = rig::cook(scratch.path(), false);
        let gpu = rig::open("portable");
        let mut host = Rig::new(&gpu, &cooker, 1);
        host.ready();
        let old = host.draw();
        let committed = host.lease.sheet_publication().expect("original sheets");
        host.host.source_mut().drawing = false;
        let description = scratch.join("live/type/font.json");
        let original = std::fs::read_to_string(&description).expect("font description");
        let stripped = original.replace("{\"points\": 12},", "");
        std::fs::write(&description, stripped).expect("remove hidden run's size");
        assert!(cooker.cook_all().expect("cook edit").published.is_some());
        thrum_test_support::wait::until(
            "candidate sheets are ready",
            Duration::from_secs(8),
            || {
                host.draw();
                assert_eq!(host.stats.lock().expect("stats").drawn, Some(1));
                (host
                    .lease
                    .sheet_publication()
                    .expect("offered sheets")
                    .generation
                    == 2)
                    .then_some(())
            },
        );
        assert!(host.host.source().next.is_some());

        host.host.source_mut().drawing = true;
        let refused = host.draw();
        assert_eq!(refused.rgba, old.rgba);
        assert_eq!(
            refused.sample.interface.unwrap().stamp,
            committed.paint_stamp
        );
        let restored = host.lease.sheet_publication().expect("restored sheets");
        assert!(std::sync::Arc::ptr_eq(&restored, &committed));
        assert!(host.lease.font_candidate().is_none());
        assert!(host.host.source().next.is_none());
        for _ in 0..4 {
            let submitted = host.draw();
            assert_eq!(submitted.rgba, old.rgba);
            assert_eq!(
                submitted.sample.interface.unwrap().stamp,
                Some(committed.paint_stamp.unwrap())
            );
        }
        let stats = host.stats.lock().expect("stats");
        assert_eq!(stats.drawn, Some(1));
        assert_eq!(stats.failures, 1);
        assert!(
            stats
                .errors
                .iter()
                .any(|reason| reason.contains("font size 12"))
        );
        assert!(!stats.swaps.iter().any(|swap| swap.generation == 2));
        println!(
            "ready generation 2 refused for newly visible size 12; generation 1 sheets and pixels retained"
        );
    }

    use thrum_render::sprite::SpriteSheetSource as _;
}
