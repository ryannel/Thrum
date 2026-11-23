use std::sync::{Arc, Mutex};

use thrum_controls::Controls;
use thrum_render::{
    capabilities::{DeviceProfile, DeviceRecipe, OpenedDevice},
    runtime::FrameConfig,
};

#[cfg(not(target_arch = "wasm32"))]
use super::cooked_store;
use super::{
    Camera, DEFAULT_GROUND, Sky, SkyEdits, SkyGame,
    capture::{crc32, encode_png},
    ground_scene, kept_errors,
};
use thrum_asset_gpu::CookedStats;

const CAMERA: Camera = Camera::look_at([0.0, 1.7, 6.0], [0.0, 1.0, 0.0], 40.0);

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "compare the exact pre-change f64 to f32 alpha expression"
)]
fn a_ticking_sky_world_states_its_clock() {
    use std::time::Duration;
    use thrum_render::runtime::FrameWorld;
    let mut world = super::StillWorld {
        tick_duration: Some(Duration::from_millis(20)),
        ..super::StillWorld::default()
    };
    // 50 Hz, at tick 2 + half an interval. Literal phases include the bound.
    for (observation_ns, tick, phases) in [
        (50_000_000, 2, [0.5_f32, 1.0, 2.0, 3.0]),
        (60_000_000, 3, [0.0, 0.5, 1.5, 2.5]),
        // One nanosecond below a tick, the 50 ms lead rounds to the f32 bound.
        (79_999_999, 3, [0.999_999_94, 1.5, 2.5, 3.5]),
    ] {
        let observation = Duration::from_nanos(observation_ns);
        world.advance(observation).expect("scripted observation");
        assert!(world.is_interpolating());
        assert_eq!(world.source().tick, tick);
        assert_eq!(
            world.tick_due_time(),
            Some(Duration::from_millis(tick * 20))
        );
        for (lead_ms, expected) in [0, 10, 30, 50].into_iter().zip(phases) {
            let lead = Duration::from_millis(lead_ms);
            let phase = world.display_phase_at(lead);
            assert!((phase - expected).abs() < 1e-6);
            // Preserve the exact expression used before this change.
            let old_alpha =
                ((observation + lead).as_secs_f64() / 0.020 - tick as f64).clamp(0.0, 1.0) as f32;
            assert_eq!(
                world.presentation_alpha_at(lead).to_bits(),
                old_alpha.to_bits()
            );
            assert_eq!(phase.min(1.0).to_bits(), old_alpha.to_bits());
        }
    }
    let still = super::StillWorld::default();
    assert!(!still.is_interpolating());
    assert_eq!(still.tick_due_time(), None);
    assert_eq!(still.display_phase_at(Duration::from_millis(50)), 1.0);
    assert_eq!(still.presentation_alpha_at(Duration::ZERO), 1.0);
}

#[test]
fn a_game_owns_its_clock_and_the_default_kit_stays_still() {
    use std::time::Duration;
    use thrum_render::runtime::FrameWorld;
    let mut still = super::StillWorld::default();
    still.advance(Duration::from_secs(10)).expect("still");
    assert_eq!(still.source().tick, 0);
    assert_eq!(still.presentation_alpha_at(Duration::ZERO), 1.0);
    let mut world = super::StillWorld {
        tick_duration: Some(Duration::from_millis(20)),
        ..super::StillWorld::default()
    };
    assert_eq!(
        world
            .advance(Duration::from_millis(50))
            .expect("tick")
            .completed_ticks,
        2
    );
    assert_eq!(world.publication().expect("page").page.tick, 2);
    assert!((world.presentation_alpha_at(Duration::ZERO) - 0.5).abs() < 1e-6);
    assert!((world.presentation_alpha_at(Duration::from_millis(5)) - 0.75).abs() < 1e-6);
    assert_eq!(
        world
            .advance(Duration::from_millis(50))
            .expect("repeat")
            .completed_ticks,
        0
    );
    let mut other = super::StillWorld {
        tick_duration: Some(Duration::from_millis(20)),
        ..super::StillWorld::default()
    };
    other
        .advance(Duration::from_millis(20))
        .expect("independent world");
    assert_eq!(other.source().tick, 1);
    assert_eq!(world.source().tick, 2);
}

#[test]
fn a_hidden_ticking_kit_world_discards_the_partial_tick_and_holds_until_commit() {
    use std::time::Duration;
    use thrum_render::runtime::FrameWorld;
    let mut world = super::StillWorld {
        tick_duration: Some(Duration::from_millis(20)),
        ..super::StillWorld::default()
    };
    world.advance(Duration::from_millis(50)).expect("visible");
    world.pause();
    world.advance(Duration::from_secs(5)).expect("hidden");
    assert_eq!(world.source().tick, 2);
    assert_eq!(world.tick_due_time(), None);
    for lead in [Duration::ZERO, Duration::from_millis(50)] {
        assert_eq!(world.presentation_alpha_at(lead), 1.0);
        assert_eq!(world.display_phase_at(lead), 1.0);
    }
    assert!(!world.is_interpolating());
    world.resume(Duration::from_secs(6));
    for millis in [6000, 6005, 6010, 6019] {
        world.advance(Duration::from_millis(millis)).expect("shown");
        assert_eq!(world.source().tick, 2);
        assert!(!world.is_interpolating());
        assert_eq!(world.presentation_alpha_at(Duration::ZERO), 1.0);
        assert_eq!(world.display_phase_at(Duration::from_millis(50)), 1.0);
        assert_eq!(world.tick_due_time(), Some(Duration::from_secs(6)));
    }
    world
        .advance(Duration::from_millis(6020))
        .expect("new commit");
    assert_eq!(world.source().tick, 3);
    assert!(world.is_interpolating());
    assert_eq!(world.presentation_alpha_at(Duration::ZERO), 0.0);
    assert_eq!(world.tick_due_time(), Some(Duration::from_millis(6020)));
}

fn sky(elevation: f32, azimuth: f32) -> Sky {
    Sky {
        exposure_compensation_ev: 0.0,
        temporal_inputs: false,
        sun_elevation_degrees: elevation,
        sun_azimuth_degrees: azimuth,
        camera: CAMERA,
        exposure_zones: false,
    }
}

fn close(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(a, b)| (a - b).abs() < 1e-6)
}

#[test]
fn sun_bearing_is_clockwise_from_north_with_y_up() {
    assert!(close(sky(90.0, 0.0).toward_sun(), [0.0, 1.0, 0.0]));
    assert!(close(sky(0.0, 0.0).toward_sun(), [0.0, 0.0, -1.0]));
    assert!(close(sky(0.0, 90.0).toward_sun(), [1.0, 0.0, 0.0]));
    assert!(close(sky(0.0, 180.0).toward_sun(), [0.0, 0.0, 1.0]));
    let low = sky(-2.0, 270.0).toward_sun();
    assert!(low[1] < 0.0 && low[0] < -0.99);
}

#[test]
fn undrawable_skies_are_refused_by_field() {
    assert!(sky(25.0, 150.0).check().is_ok());
    assert!(
        sky(f32::NAN, 0.0)
            .check()
            .unwrap_err()
            .contains("sun_elevation")
    );
    assert!(sky(91.0, 0.0).check().is_err());
    assert!(
        sky(10.0, f32::INFINITY)
            .check()
            .unwrap_err()
            .contains("azimuth")
    );
    let mut buried = sky(10.0, 0.0);
    buried.camera.position[1] = -1.0;
    assert!(buried.check().unwrap_err().contains("sea level"));
    let mut vertical = sky(10.0, 0.0);
    vertical.camera.target = [0.0, 0.0, 6.0];
    assert!(vertical.check().unwrap_err().contains("vertically"));
    let mut fisheye = sky(10.0, 0.0);
    fisheye.camera.vertical_fov_degrees = 179.0;
    assert!(fisheye.check().is_err());
}

#[test]
fn update_sends_only_changed_drawable_skies() {
    let elevation = Arc::new(Mutex::new(25.0_f32));
    let shared = Arc::clone(&elevation);
    let mut game = SkyGame::new(Controls::new(), CAMERA, move |_, sky| {
        sky.sun_elevation_degrees = *shared.lock().unwrap();
    });
    let first = game.update();
    assert_eq!(first.0.map(|sky| sky.sun_elevation_degrees), Some(25.0));
    assert!(game.update().no_changes(), "an unchanged sky is not resent");
    *elevation.lock().unwrap() = f32::NAN;
    assert!(game.update().no_changes(), "a refused sky is not sent");
    assert_eq!(
        game.sky.sun_elevation_degrees, 25.0,
        "the previous sky is kept"
    );
    assert!(game.refused.is_some());
    *elevation.lock().unwrap() = 5.0;
    assert_eq!(
        game.update().0.map(|sky| sky.sun_elevation_degrees),
        Some(5.0)
    );
    assert!(game.refused.is_none());
    let merged = SkyEdits(Some(sky(1.0, 0.0))).merge_edits(SkyEdits(None));
    assert_eq!(merged.0, Some(sky(1.0, 0.0)));
    let merged = merged.merge_edits(SkyEdits(Some(sky(2.0, 0.0))));
    assert_eq!(merged.0, Some(sky(2.0, 0.0)));
}

#[test]
fn png_is_a_valid_stored_rgb_image() {
    // The CRC of an empty IEND chunk is fixed by the PNG specification.
    assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    let rgba: Vec<u8> = (0_u8..3 * 2 * 4).map(|value| value * 10).collect();
    let png = encode_png([3, 2], &rgba).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&png[12..16], b"IHDR");
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 3);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 2);
    assert_eq!(&png[24..29], &[8, 2, 0, 0, 0]);
    assert_eq!(&png[png.len() - 12..], b"\0\0\0\0IEND\xAE\x42\x60\x82");
    // Inflate the single stored block and compare each filtered scanline with the input.
    let idat = 8 + 25;
    assert_eq!(&png[idat + 4..idat + 8], b"IDAT");
    let zlib = &png[idat + 8..];
    assert_eq!((zlib[0], zlib[1], zlib[2]), (0x78, 0x01, 1));
    let length = usize::from(u16::from_le_bytes([zlib[3], zlib[4]]));
    assert_eq!(length, 2 * (1 + 3 * 3));
    let raw = &zlib[7..7 + length];
    for (row, line) in raw.chunks(10).enumerate() {
        assert_eq!(line[0], 0);
        for column in 0..3 {
            let source = &rgba[(row * 3 + column) * 4..][..3];
            assert_eq!(&line[1 + column * 3..][..3], source);
        }
    }
    assert!(encode_png([3, 3], &rgba).is_err());
    assert!(encode_png([0, 2], &[]).is_err());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn missing_or_uncooked_stores_name_the_next_step() {
    let unset = cooked_store(None).unwrap_err();
    assert!(unset.contains("THRUM_ASSET_STORE is not set"), "{unset}");
    assert!(unset.contains("./dev scene"), "{unset}");
    let scratch = thrum_test_support::Scratch::new("sky-store");
    let root = scratch.path().to_path_buf();
    let empty = cooked_store(Some(root.clone().into_os_string())).unwrap_err();
    assert!(empty.contains("has no HEAD"), "{empty}");
    std::fs::write(root.join("HEAD"), b"1").unwrap();
    assert_eq!(
        cooked_store(Some(root.clone().into_os_string())),
        Ok(root.clone())
    );
}

#[test]
fn the_ground_is_one_wide_slab_with_its_top_at_zero() {
    let scene = ground_scene(Some(DEFAULT_GROUND), &[]).unwrap();
    let slab = &scene.instances[0];
    assert!(slab.is_active(), "the ground row is active");
    let top = slab.current[3][1] + slab.current[1][1];
    assert!(top.abs() < 1e-6, "top face at {top}");
    assert!(
        slab.current[0][0] >= 1_000.0,
        "the ground reaches toward the horizon"
    );
    assert!(
        (slab.current[2][2] - slab.current[0][0]).abs() < 1e-3,
        "and is square"
    );
    assert_eq!(scene.materials[0].base_rgba[..3], DEFAULT_GROUND);
    assert!(!ground_scene(None, &[]).unwrap().instances[0].is_active());
    let message = ground_scene(Some([0.2, 1.5, 0.1]), &[]).unwrap_err();
    assert!(message.contains("within 0–1"), "{message}");
}

/// Render `game` offscreen at 160×90 for three frames; fail on any wgpu validation error.
fn render(gpu: &thrum_test_support::Gpu, mut game: SkyGame) -> Vec<u8> {
    pollster::block_on(async {
        let OpenedDevice {
            queue,
            capabilities,
            watch,
            ..
        } = DeviceRecipe {
            profile: DeviceProfile::Portable,
            ..Default::default()
        }
        .open_admitted(&gpu.instance, &gpu.adapter)
        .await
        .expect("device");
        let extent = [160, 90];
        let mut host = game
            .build_host(
                &capabilities,
                &queue,
                FrameConfig {
                    render_extent: extent,
                    surface: None,
                },
            )
            .await
            .unwrap();
        let mut pixels = Vec::new();
        for _ in 0..3 {
            pixels = host.capture_pixels().unwrap();
        }
        assert_eq!(pixels.len(), 160 * 90 * 4);
        assert_eq!(watch.uncaptured_error(), None);
        pixels
    })
}

/// Mean of one channel over rows `rows` of a 160-pixel-wide RGBA image.
fn mean(pixels: &[u8], rows: std::ops::Range<usize>, channel: usize) -> f64 {
    let band = &pixels[rows.start * 160 * 4..rows.end * 160 * 4];
    let pixels = band.as_chunks::<4>().0;
    pixels
        .iter()
        .map(|pixel| f64::from(pixel[channel]))
        .sum::<f64>()
        / pixels.len() as f64
}

/// The sky above and the ground below the horizon: with no ground the scene has one inactive
/// instance row, so every pass (Sun shadow cascades, light clusters, forward, history and
/// display) also runs with zero active rows.
#[test]
fn the_sky_and_ground_render_through_every_pass() {
    let Some(gpu) = thrum_test_support::gpu(
        "the_sky_and_ground_render_through_every_pass",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let sun = |_: &Controls, sky: &mut Sky| {
        sky.sun_elevation_degrees = 25.0;
        sky.sun_azimuth_degrees = 150.0;
    };
    let bare = render(
        &gpu,
        SkyGame::new(Controls::new(), CAMERA, sun).with_ground(None),
    );
    // The upper rows are daylit sky: bright and bluer than red.
    assert!(mean(&bare, 0..10, 2) > 60.0, "sky is lit");
    assert!(
        mean(&bare, 0..10, 2) > mean(&bare, 0..10, 0),
        "daylit sky is bluer than red"
    );
    let grounded = render(
        &gpu.fresh_adapter(),
        SkyGame::new(Controls::new(), CAMERA, sun),
    );
    assert!(
        mean(&grounded, 0..10, 2) > 60.0,
        "the sky stays lit above the ground"
    );
    // The lowest rows look down at the ground, lit and warmer than the blue sky.
    let (red, blue) = (mean(&grounded, 80..90, 0), mean(&grounded, 80..90, 2));
    assert!(red > 30.0, "the ground is lit: red mean {red}");
    assert!(
        red > blue,
        "the default ground is earth-coloured: {red} vs {blue}"
    );
    assert!(
        (red - mean(&bare, 80..90, 0)).abs() > 5.0,
        "the ground replaces the sky's planet surface"
    );
}

#[test]
fn loader_failures_after_the_first_draw_are_reported_once_with_the_kept_generation() {
    let mut stats = CookedStats::default();
    let mut shown = 0;
    let fail = |stats: &mut CookedStats, error: &str| {
        stats.errors.push(error.to_owned());
        stats.failures += 1;
    };
    fail(&mut stats, "generation 1: unreadable");
    assert!(
        kept_errors(&stats, &mut shown).is_empty(),
        "the status reports it"
    );
    stats.drawn = Some(2);
    assert_eq!(kept_errors(&stats, &mut shown), []);
    fail(&mut stats, "generation 3: asset unavailable");
    fail(&mut stats, "generation 4: asset unavailable");
    assert_eq!(
        kept_errors(&stats, &mut shown),
        [
            (2, "generation 3: asset unavailable".to_owned()),
            (2, "generation 4: asset unavailable".to_owned())
        ]
    );
    assert!(
        kept_errors(&stats, &mut shown).is_empty(),
        "each is reported once"
    );
    // A failure in the same poll as the first draw, but before it, is not reported as kept.
    let mut late = CookedStats::default();
    let mut late_shown = 0;
    fail(&mut late, "generation 1: unreadable");
    late.failures_before_first_draw = late.failures;
    late.drawn = Some(2);
    assert_eq!(kept_errors(&late, &mut late_shown), []);
    // The bounded list dropped its oldest entry: only the new one is reported.
    stats.errors.remove(0);
    fail(&mut stats, "generation 5: asset unavailable");
    assert_eq!(
        kept_errors(&stats, &mut shown),
        [(2, "generation 5: asset unavailable".to_owned())]
    );
}
