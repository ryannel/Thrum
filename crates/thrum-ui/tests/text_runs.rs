//! Pen positions calculated from the table, with separate cases for each fitting rule.

mod text_support;

use text_support::font;
use thrum_ui::{Alignment, Elision, PaintListBuilder, TextOptions, TextRun};

const fn opts(points: f32, extent: [u32; 2]) -> TextOptions {
    TextOptions {
        points,
        extent,
        scale: 1,
        alignment: Alignment::Left,
        wrap: false,
        elision: Elision::End,
        tabular_digits: false,
        outlined: false,
    }
}

fn pixel(value: &str, options: TextOptions) -> TextRun {
    TextRun::new(font("pixel"), value, options).expect("pixel run")
}

fn pens(run: &TextRun) -> Vec<[i64; 2]> {
    run.glyphs().iter().map(|glyph| glyph.pen).collect()
}

fn frames(run: &TextRun) -> Vec<u32> {
    run.glyphs().iter().map(|glyph| glyph.frame).collect()
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "font ascent in whole output pixels"
)]
fn offsets_add_advance_and_kerning_then_origin_snaps() {
    let font = font("type");
    let strike = font.strike(24.0, 1).expect("cooked");
    let baseline = strike.metrics.ascent.ceil() as i64;
    let a = strike.glyph('A').expect("A");
    let v = strike.glyph('V').expect("V");
    let k = strike
        .kerning
        .iter()
        .find(|entry| entry.pair == [65, 86])
        .expect("AV kern")
        .pixels;
    assert_ne!(k, 0);
    let x_first = i64::from(
        (-strike.frame(a.frame).expect("frame").pivot[0])
            .min(0.0)
            .abs() as i32,
    );
    let x_second = x_first + i64::from(a.advance + k);
    let x_third = x_second + i64::from(v.advance);
    let run = TextRun::new(font, "AV ", opts(24.0, [100, 40])).expect("run");
    assert_eq!(
        pens(&run),
        [
            [x_first, baseline],
            [x_second, baseline],
            [x_third, baseline]
        ]
    );
    let mut list = PaintListBuilder::new([120, 80]);
    run.paint(&mut list, 4, [16.51, 17.49], [1.0; 4], [0.0; 4]);
    let rows = list.finish().rows;
    assert_eq!(
        rows[0].position.map(f64::to_bits),
        [
            17.0 + f64::from(i32::try_from(x_first).expect("small x")),
            17.0 + f64::from(i32::try_from(baseline).expect("small baseline"))
        ]
        .map(f64::to_bits)
    );
}

#[test]
fn unsupported_size_and_scale_are_named() {
    let font = font("type");
    assert!(
        TextRun::new(font, "AV", opts(30.0, [100, 40]))
            .expect_err("size")
            .to_string()
            .contains("size 30")
    );
    assert!(
        TextRun::new(
            font,
            "AV",
            TextOptions {
                scale: 4,
                ..opts(24.0, [100, 40])
            }
        )
        .expect_err("scale")
        .to_string()
        .contains("scale 4")
    );
}

#[test]
fn missing_character_uses_a_fixed_box_and_counts() {
    let font = font("pixel");
    let missing = font.strike(8.0, 1).expect("strike").glyphs[0];
    let run = pixel("♥🦉A", opts(8.0, [80, 24]));
    assert_eq!(run.missing(), 2);
    assert_eq!(frames(&run)[..2], [missing.frame; 2]);
    assert_eq!(pens(&run), [[0, 7], [8, 7], [16, 7]]);
}

#[test]
fn base_and_mark_have_the_precomposed_frame() {
    let font = font("type");
    let options = opts(16.0, [150, 28]);
    let composed = TextRun::new(font, "Éž", options).expect("precomposed");
    let decomposed = TextRun::new(font, "E\u{301}z\u{30c}", options).expect("decomposed");
    assert_eq!(composed.glyphs(), decomposed.glyphs());
    assert_eq!(decomposed.missing(), 0);
}

#[test]
fn tabular_digits_all_take_the_widest_advance() {
    let font = font("type");
    let strike = font.strike(16.0, 1).expect("strike");
    let widest = ('0'..='9')
        .map(|ch| strike.glyph(ch).expect("digit").advance)
        .max()
        .expect("widest");
    let run = TextRun::new(
        font,
        "0123456789",
        TextOptions {
            tabular_digits: true,
            ..opts(16.0, [200, 24])
        },
    )
    .expect("digits");
    assert_eq!(
        pens(&run)
            .windows(2)
            .map(|pair| pair[1][0] - pair[0][0])
            .collect::<Vec<_>>(),
        [i64::from(widest); 9]
    );
}

#[test]
fn breaks_at_newline() {
    assert_eq!(
        pens(&pixel("AB\nBA", opts(8.0, [30, 16]))),
        [[0, 7], [8, 7], [0, 15], [8, 15]]
    );
}

#[test]
fn wraps_at_the_last_space_that_fits() {
    let options = TextOptions {
        wrap: true,
        ..opts(8.0, [32, 16])
    };
    assert_eq!(
        pens(&pixel("A B AB", options)),
        [[0, 7], [8, 7], [12, 7], [0, 15], [8, 15]]
    );
}

#[test]
fn aligns_the_line_in_the_available_width() {
    for (alignment, offset) in [
        (Alignment::Left, 0),
        (Alignment::Centre, 8),
        (Alignment::Right, 17),
    ] {
        assert_eq!(
            pens(&pixel(
                "AB",
                TextOptions {
                    alignment,
                    ..opts(8.0, [33, 8])
                }
            )),
            [[offset, 7], [offset + 8, 7]]
        );
    }
}

#[test]
fn elides_at_the_end_or_keeps_both_ends() {
    let options = opts(8.0, [24, 8]);
    let end = pixel("ABABAB", options);
    let middle = pixel(
        "ABABAB",
        TextOptions {
            elision: Elision::Middle,
            ..options
        },
    );
    assert_eq!(frames(&end), frames(&pixel("AB…", options)));
    assert_eq!(frames(&middle), frames(&pixel("A…B", options)));
    assert_eq!(pens(&end), [[0, 7], [8, 7], [16, 7]]);
}

#[test]
fn box_smaller_than_one_line_has_no_glyphs() {
    assert_eq!(pixel("A", opts(8.0, [80, 7])).glyphs(), []);
    assert_eq!(pixel("A", opts(8.0, [7, 8])).glyphs(), []);
}
