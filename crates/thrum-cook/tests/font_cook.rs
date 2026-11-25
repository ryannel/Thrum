//! Validate outline metrics through CoreText and pixel cells against authored texels.

mod font_support;

use thrum_asset::{FontProduct, FontStrike, SheetKind, layout::NO_OUTLINE};
use thrum_test_support::Scratch;

#[cfg(target_os = "macos")]
#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "reference pixel values round to table precision"
)]
fn outline_metrics_and_coverage_match_the_source() {
    let scratch = Scratch::new("font-cook-outline");
    let source = font_support::fixture("type");
    font_support::copy(&source, &scratch.join("source/type"));
    let mut cooker = font_support::cooker(scratch.join("source"), scratch.join("store"));
    assert_eq!(cooker.cook_all().expect("cook").failed, []);
    let font = FontProduct::open(font_support::product(cooker.store(), "type")).expect("font");
    assert_eq!(font.sizes().len(), 9);
    let core_text = std::process::Command::new("swift")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/font_probe.swift"))
        .arg(source.join("Roboto-Regular.ttf"))
        .output()
        .expect("run the independent CoreText reader");
    assert!(
        core_text.status.success(),
        "{}",
        String::from_utf8_lossy(&core_text.stderr)
    );
    let references: Vec<Reference> =
        serde_json::from_slice(&core_text.stdout).expect("CoreText metrics");
    let mut nonzero_kerns = 0;
    for reference in references {
        let strike = font
            .strike(reference.points, reference.scale)
            .expect("cooked size and scale");
        assert_eq!(strike.sheet.kind(), SheetKind::Pixel);
        assert_close(
            f64::from(strike.metrics.ascent),
            reference.line[0],
            "ascent",
        );
        assert_close(
            f64::from(strike.metrics.descent),
            reference.line[1],
            "descent",
        );
        assert_close(f64::from(strike.metrics.gap), reference.line[2], "line gap");
        assert_eq!(strike.glyphs[0].scalar, 0);
        assert!(strike.glyphs[0].advance > 0);
        assert_eq!(strike.glyphs.len(), reference.glyphs.len() + 1);
        let mut widest = 0;
        for (scalar, [advance, min_x, min_y, max_x, max_y]) in reference.glyphs {
            let character = char::from_u32(scalar).expect("scalar");
            let glyph = strike.glyph(character).expect("mapped glyph");
            assert_eq!(
                glyph.advance,
                advance.round() as i32,
                "{character:?} advance"
            );
            let frame = strike.frame(glyph.frame).expect("glyph frame");
            assert!(frame.pivot.iter().all(|pivot| pivot.fract() == 0.0));
            if max_x > min_x && max_y > min_y {
                let left = f64::from(-frame.pivot[0]);
                let top = f64::from(-frame.pivot[1]);
                assert!(
                    min_x >= left - 0.001
                        && max_x <= left + f64::from(frame.size[0]) + 0.001
                        && -max_y >= top - 0.001
                        && -min_y <= top + f64::from(frame.size[1]) + 0.001,
                    "{character} at {} x {}: {frame:?} vs {min_x},{min_y}..{max_x},{max_y}",
                    reference.points,
                    reference.scale
                );
            }
            if character.is_ascii_digit() {
                widest = widest.max(glyph.advance);
            }
            let outlined = reference.points.to_bits() == 16.0f32.to_bits();
            assert_eq!(glyph.outline != NO_OUTLINE, outlined);
            if outlined {
                assert!(strike.frame(glyph.outline).is_some());
            }
        }
        assert_eq!(strike.metrics.widest_digit, widest);
        for (pair, adjustment) in reference.kern {
            let pair = [u32::from(pair.as_bytes()[0]), u32::from(pair.as_bytes()[1])];
            let kerned = strike
                .kerning
                .iter()
                .find(|entry| entry.pair == pair)
                .map_or(0, |entry| entry.pixels);
            assert_eq!(
                kerned,
                adjustment.round() as i32,
                "pair {pair:?} size {} scale {}",
                reference.points,
                reference.scale
            );
            nonzero_kerns += usize::from(kerned != 0);
        }
        assert!(strike.glyph('Ł').is_some());
        let f = strike.glyph('F').expect("F");
        let frame = strike.frame(f.frame).expect("F frame");
        let alpha = |x, y| texel(strike, f.frame, x, y)[3];
        let rows: Vec<_> = (0..frame.size[1])
            .map(|y| {
                (0..frame.size[0])
                    .map(|x| u32::from(alpha(x, y)))
                    .sum::<u32>()
            })
            .collect();
        let columns: Vec<_> = (0..frame.size[0])
            .map(|x| {
                (0..frame.size[1])
                    .map(|y| u32::from(alpha(x, y)))
                    .sum::<u32>()
            })
            .collect();
        let half_h = rows.len() / 2;
        let half_w = columns.len() / 2;
        assert!(
            rows[..half_h].iter().sum::<u32>() > rows[rows.len() - half_h..].iter().sum(),
            "upright F"
        );
        assert!(
            columns[..half_w].iter().sum::<u32>() > columns[columns.len() - half_w..].iter().sum(),
            "unmirrored F"
        );
        let covered = (0..frame.size[1])
            .flat_map(|y| (0..frame.size[0]).map(move |x| texel(strike, f.frame, x, y)));
        for [r, g, b, a] in covered {
            assert_eq!([r, g], [b; 2]);
            let code = f64::from(b) / 255.0;
            let linear = if code < 0.04045 {
                code / 12.92
            } else {
                ((code + 0.055) / 1.055).powf(2.4)
            };
            assert!((linear - f64::from(a) / 255.0).abs() <= 0.005);
        }
    }
    assert!(nonzero_kerns > 0, "rounded kerned pairs were compared");
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
struct Reference {
    points: f32,
    scale: u32,
    glyphs: std::collections::BTreeMap<u32, [f64; 5]>,
    kern: std::collections::BTreeMap<String, f64>,
    line: [f64; 3],
}

#[cfg(target_os = "macos")]
fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "{label}: {actual} versus {expected}"
    );
}

#[test]
fn pixel_font_keeps_the_authored_grid_and_grows_one_texel() {
    let scratch = Scratch::new("font-cook-pixel");
    let source = font_support::fixture("pixel");
    font_support::copy(&source, &scratch.join("source/pixel"));
    let mut cooker = font_support::cooker(scratch.join("source"), scratch.join("store"));
    assert_eq!(cooker.cook_all().expect("cook").failed, []);
    let font = FontProduct::open(font_support::product(cooker.store(), "pixel")).expect("font");
    let strike = font.strike(8.0, 1).expect("one authored size");
    assert_eq!(font.sizes().len(), 1);
    assert!(font.sheet(2).is_none());
    let source_grid = image::open(source.join("grid.png"))
        .expect("grid")
        .into_rgba8();
    let description: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source.join("font.json")).expect("description"))
            .expect("JSON");
    for cell in description["characters"].as_array().expect("cells") {
        let character = cell["character"]
            .as_str()
            .expect("character")
            .chars()
            .next()
            .expect("scalar");
        let cell_x = u32::try_from(cell["cell"][0].as_u64().expect("column")).expect("u32");
        let glyph = strike.glyph(character).expect("cooked character");
        assert_eq!(
            i64::from(glyph.advance),
            cell["advance"].as_i64().expect("advance")
        );
        let source_texel = |x, y| source_grid.get_pixel(cell_x * 8 + x, y).0;
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(texel(strike, glyph.frame, x, y), source_texel(x, y));
            }
        }
        assert_eq!(strike.frame(glyph.outline).expect("outline").size, [10; 2]);
        for y in 0u32..10 {
            for x in 0u32..10 {
                let expected = (0u32..8)
                    .flat_map(|sy| (0u32..8).map(move |sx| (sx, sy)))
                    .any(|(sx, sy)| {
                        (sx + 1).abs_diff(x) <= 1
                            && (sy + 1).abs_diff(y) <= 1
                            && source_texel(sx, sy)[3] == 255
                    });
                assert_eq!(
                    texel(strike, glyph.outline, x, y),
                    [if expected { 255 } else { 0 }; 4]
                );
            }
        }
    }
}

fn texel(strike: FontStrike<'_>, frame: u32, x: u32, y: u32) -> [u8; 4] {
    let frame = strike.frame(frame).expect("frame");
    let sheet = strike.sheet.texture();
    let level = &sheet.levels()[0];
    let offset = ((frame.page * level.height + frame.origin[1] + y) * level.width
        + frame.origin[0]
        + x) as usize
        * 4;
    sheet.level_data(level)[offset..offset + 4]
        .try_into()
        .expect("texel")
}
