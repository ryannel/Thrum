//! Independent sRGB-decode and linear-compositing oracle over submitted pixels.

#[expect(
    dead_code,
    reason = "each sprite proof uses a subset of the shared host fixture"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use thrum_render::{
    sprite::{SpriteRect, SpriteRow},
    synthetic::CaptureSource,
};

const COLORS: [[u8; 4]; 3] = [
    [142, 79, 180, 255],
    [180, 130, 75, 149],
    [37, 179, 120, 110],
];
const TINTS: [[f32; 4]; 3] = [
    [0.80, 0.9, 0.71, 1.0],
    [1.3, 0.4, 0.7, 0.78],
    [1.0, 0.5, 0.88, 0.9],
];
const FLASHES: [[f32; 4]; 3] = [[0.0; 4], [0.23, 0.85, 0.71, 0.5], [1.0, 0.68, 0.12, 0.77]];
const ADDITIVE: [f32; 3] = [0.0, 1.0, 0.0];
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

fn linear(code: u8) -> f64 {
    let code = f64::from(code) / 255.0;
    if code <= 0.04045 {
        code / 12.92
    } else {
        ((code + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb(linear: f64) -> f64 {
    let channel = linear.clamp(0.0, 1.0);
    (255.0
        * if channel <= 0.003_130_8 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        })
    .round()
}

fn lay_over(dst: [f64; 3], index: usize) -> [f64; 3] {
    let coverage = f64::from(COLORS[index][3]) / 255.0 * f64::from(TINTS[index][3]);
    let flash_amount = f64::from(FLASHES[index][3]);
    let covered = coverage * (1.0 - f64::from(ADDITIVE[index]));
    [0, 1, 2].map(|c| {
        let tinted = linear(COLORS[index][c]) * f64::from(TINTS[index][c]);
        let lit = tinted * (1.0 - flash_amount) + f64::from(FLASHES[index][c]) * flash_amount;
        lit * coverage + dst[c] * (1.0 - covered)
    })
}

#[test]
fn all_orders_in_one_pass_match_the_cpu_blend_with_an_additive_middle() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("interleaved sprite blend", "portable")
    else {
        return;
    };
    let extent = [48, 16];
    let frames = [0, 1, 2].map(|x| SpriteRect {
        origin: [x, 0],
        size: [1; 2],
    });
    let pixels: Vec<_> = COLORS.into_iter().flatten().collect();
    let mut rows = Vec::new();
    for (column, order) in (0_u32..).zip(ORDERS) {
        for (layer, index) in (0..).zip(order) {
            rows.push(SpriteRow {
                tint: TINTS[index],
                flash: FLASHES[index],
                additive: ADDITIVE[index],
                layer,
                scale: [4.0; 2],
                ..SpriteRow::new(
                    u32::try_from(index).expect("three frames"),
                    [-20.0 + f64::from(column) * 7.0, 2.0],
                )
            });
        }
    }
    // Deliberately supply rows in reverse order. Layer restores each blend stack.
    rows.reverse();
    let mut host = fixture::host(
        &caps,
        &queue,
        extent,
        fixture::Scene::new(rows, 1),
        fixture::sheet([3, 1], &pixels, &frames),
    );
    let image = fixture::capture(&mut host, &device, extent, CaptureSource::Sdr);
    assert_eq!(image.sample.sprites.expect("sprite draw stats").draws, 1);
    let mut results = Vec::new();
    for (column, order) in ORDERS.into_iter().enumerate() {
        let expected = order.into_iter().fold([0.0; 3], lay_over).map(srgb);
        let address = (7 * 48 + 5 + 7 * column) * 4;
        let pixel = &image.rgba[address..address + 4];
        for c in 0..3 {
            assert!(
                (f64::from(pixel[c]) - expected[c]).abs() <= 1.0,
                "{order:?}: {pixel:?}, CPU {expected:?}"
            );
        }
        assert_eq!(pixel[3], 255);
        results.push(pixel.to_vec());
    }
    assert_ne!(
        results[0], results[1],
        "the additive middle really changes the composite"
    );
}
