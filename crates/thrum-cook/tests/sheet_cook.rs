//! Original-frame texels, deterministic pages and last-good sheet publication.

#[path = "sheet_support/setup.rs"]
mod setup;

use image::{Rgba, RgbaImage};
use thrum_asset::{Ktx2Format, Profile, SheetKind, SheetProduct};
use thrum_test_support::Scratch;

const DESCRIPTION: &str = r#"{
    "kind":"pixel", "density":2.5, "page_size":24,
    "pivots":{"actor":[4.5,2.0], "walk.0":[1,2]},
    "grids":{"walk.png":[3,4]}, "tiles":["walk.1"]
}"#;

const RED: Rgba<u8> = Rgba([213, 39, 77, 150]);
const GREEN: Rgba<u8> = Rgba([48, 224, 67, 230]);
const BLUE: Rgba<u8> = Rgba([29, 91, 201, 255]);

#[test]
fn deterministic_profile_pages_match_original_texels_and_mip_gutters() {
    let scratch = Scratch::new("sheet-cook");
    let folder = setup::root(scratch.path());
    std::fs::write(folder.join("sheet.json"), DESCRIPTION).expect("description");
    let single = RgbaImage::from_fn(8, 8, |x, y| {
        if (2..5).contains(&x) && (1..6).contains(&y) {
            if x == 2 { RED } else { GREEN }
        } else {
            Rgba([33, 33, 33, 0])
        }
    });
    single.save(folder.join("actor.png")).expect("single");
    let grid = RgbaImage::from_fn(6, 4, |x, y| {
        if x >= 3 {
            BLUE
        } else if x == 1 && y >= 1 {
            GREEN
        } else {
            Rgba([0; 4])
        }
    });
    grid.save(folder.join("walk.png")).expect("grid");
    let mutate = setup::cook(scratch.path(), "first");
    let independently = setup::cook(scratch.path(), "second");
    verify_cooks(mutate, independently, &single, &grid);
}

fn verify_cooks(
    mut first: thrum_cook::Cooker,
    mut second: thrum_cook::Cooker,
    single: &RgbaImage,
    grid: &RgbaImage,
) {
    for cooker in [&mut first, &mut second] {
        let report = cooker.cook_all().expect("cook");
        assert!(report.failed.is_empty(), "{report:?}");
        assert_eq!(report.cooked, 3);
    }
    for profile in Profile::ALL {
        let bytes = setup::sheet_bytes(first.store(), profile);
        let other = setup::sheet_bytes(second.store(), profile);
        assert_eq!(&*bytes, &*other);
        let sheet = SheetProduct::open(bytes).expect("sheet product");
        assert_eq!(sheet.kind(), SheetKind::Pixel);
        assert_eq!(sheet.density().to_bits(), 2.5_f32.to_bits());
        let texture = sheet.texture();
        assert_eq!(texture.format(), Ktx2Format::Rgba8Srgb);
        assert_eq!(texture.array_layers(), 2);
        assert_eq!(texture.levels().len(), 3);
        let expected = [
            (
                "actor",
                [2, 1],
                [3, 5],
                [8, 8],
                [4.5_f32, 2.0_f32],
                0,
                single,
                0,
            ),
            ("walk.0", [1, 1], [1, 3], [3, 4], [1.0, 2.0], 0, grid, 0),
            ("walk.1", [0, 0], [3, 4], [3, 4], [1.5, 2.0], 1, grid, 3),
        ];
        assert_eq!(sheet.frames().len(), expected.len());
        let mut protected = std::collections::HashSet::new();
        for (index, ((name, frame), (authored, trim, size, source, pivot, tile, image, cell_x))) in
            sheet.frames().zip(expected).enumerate()
        {
            assert_eq!(name, authored);
            assert_eq!(frame.page, u32::from(index == 2));
            assert_eq!(
                (frame.trim, frame.size, frame.source_size, frame.tile),
                (trim, size, source, tile)
            );
            assert_eq!(frame.pivot.map(f32::to_bits), pivot.map(f32::to_bits));
            let at = |x, y| image.get_pixel(cell_x + trim[0] + x, trim[1] + y).0;
            for (mip, level) in texture.levels().iter().enumerate() {
                let bytes = texture.level_data(level);
                let [x0, y0] = frame.origin.map(|xy| xy >> mip);
                let step = 1 << mip;
                let [w, h] = size.map(|side| side.div_ceil(step));
                let page_offset = frame.page * level.width * level.height;
                for y in y0 - 1..=y0 + h {
                    for x in x0 - 1..=x0 + w {
                        assert!(x < level.width && y < level.height);
                        assert!(
                            protected.insert((mip, frame.page, x, y)),
                            "adjacent mip gutters overlap"
                        );
                        let texel = (page_offset + y * level.width + x) as usize * 4;
                        let observed = &bytes[texel..texel + 4];
                        let inside = (x0..x0 + w).contains(&x) && (y0..y0 + h).contains(&y);
                        if mip == 0 && inside {
                            assert_premultiplied(observed, at(x - x0, y - y0));
                        } else if !inside {
                            if tile == 1 {
                                assert_premultiplied(observed, BLUE.0);
                            } else {
                                assert_eq!(observed, [0; 4], "{name} mip {mip}");
                            }
                        }
                    }
                }
            }
        }
    }
}

fn assert_premultiplied(observed: &[u8], source: [u8; 4]) {
    let decode = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    for channel in 0..3 {
        let expected = decode(source[channel]) * f64::from(source[3]) / 255.0;
        let lower = decode(observed[channel].saturating_sub(1));
        let upper = decode(observed[channel].saturating_add(1));
        assert!(
            (lower..=upper).contains(&expected),
            "{observed:?} from {source:?}"
        );
    }
    assert_eq!(observed[3], source[3]);
}

#[test]
fn named_bad_edits_retain_the_last_good_manifest() {
    let scratch = Scratch::new("sheet-bad-save");
    let folder = setup::root(scratch.path());
    let description = folder.join("sheet.json");
    let actor = folder.join("actor.png");
    let duplicate = folder.join("actor.0.png");
    let png = RgbaImage::from_pixel(6, 6, BLUE);
    png.save(&actor).expect("actor");
    std::fs::write(
        &description,
        r#"{"kind":"pixel","density":1,"page_size":20}"#,
    )
    .expect("description");
    let mut cooker = setup::cook(scratch.path(), "store");
    let previous = cooker.cook_all().expect("initial cook").published;
    assert!(previous.is_some());
    let mut check = |reason: &str| {
        let report = cooker.cook_all().expect("bad edit reported");
        assert_eq!(report.failed.len(), 1, "{report:?}");
        assert_eq!(report.failed[0].0, "actors");
        assert!(report.failed[0].1.contains(reason), "{report:?}");
        assert!(report.published.is_none(), "{report:?}");
        assert_eq!(cooker.store().read_head().expect("head"), previous);
    };
    RgbaImage::from_pixel(13, 2, GREEN)
        .save(&actor)
        .expect("oversize image");
    check("actor: frame is larger than a sheet page");
    png.save(&actor).expect("reset actor");
    png.save(&duplicate).expect("duplicate");
    std::fs::write(
        &description,
        r#"{"kind":"pixel","density":1,"grids":{"actor.png":[3,3]}}"#,
    )
    .expect("grid");
    check("duplicate sheet frame name actor.0");
    std::fs::remove_file(&duplicate).expect("remove duplicate");
    std::fs::write(
        &description,
        r#"{"kind":"pixel","density":1,"grids":{"absent.png":[3,3]}}"#,
    )
    .expect("missing");
    check("missing image absent.png");
}

#[test]
fn smooth_sheet_profiles_keep_block_compression_sectioned_by_frame() {
    let scratch = Scratch::new("smooth-sheet-profiles");
    let folder = setup::root(scratch.path());
    std::fs::write(
        folder.join("sheet.json"),
        r#"{"kind":"smooth","density":4,"page_size":64}"#,
    )
    .expect("smooth description");
    for (name, colour) in [("a", RED), ("b", BLUE)] {
        RgbaImage::from_pixel(3, 5, colour)
            .save(folder.join(format!("{name}.png")))
            .expect("image");
    }
    let mut cooker = setup::cook(scratch.path(), "store");
    assert_eq!(cooker.cook_all().expect("cook").failed, []);
    for (profile, format) in [
        (Profile::Portable, Ktx2Format::Rgba8Srgb),
        (Profile::DesktopBc, Ktx2Format::Bc7Srgb),
        (Profile::AppleAstc, Ktx2Format::Astc4x4Srgb),
    ] {
        let product = SheetProduct::open(setup::sheet_bytes(cooker.store(), profile))
            .expect("smooth product");
        assert_eq!(product.kind(), SheetKind::Smooth);
        let texture = product.texture();
        assert_eq!(texture.format(), format);
        assert_eq!(texture.levels().len(), 3);
        let origins: Vec<_> = product.frames().map(|(_, frame)| frame.origin).collect();
        let [left, right] = origins.as_slice() else {
            panic!("two packed frames");
        };
        // At level 2, each 4×4 encoded block belongs to only one frame.
        assert_eq!((left[0] - 4) / 16 + 1, (right[0] - 4) / 16);
        assert!((right[0] - 4).is_multiple_of(16));
    }
}
