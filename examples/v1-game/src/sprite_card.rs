//! Original asymmetric registration card for 2D output.

use thrum_render::{
    runtime::FrameError,
    sprite::{EmbeddedSheet, SpriteFrame, SpriteRect, SpriteRow, SpriteScene, SpriteView},
};

pub(crate) const SIZE: [u32; 2] = [256, 128];
const ROWS: [SpriteRow; 1] = [SpriteRow::new(0, [-128.0, 64.0])];

pub(crate) fn scene(scale: u16) -> Result<SpriteScene, FrameError> {
    if !(1..=6).contains(&scale) {
        return Err(FrameError::InvalidState(
            "sprite card scale must be an integer from 1 to 6",
        ));
    }
    Ok(SpriteScene::new(
        EmbeddedSheet {
            extent: SIZE,
            rgba: texels().into(),
            frames: [SpriteRect {
                origin: [0, 0],
                size: SIZE,
            }]
            .into(),
            texels_per_unit: 1.0,
        },
        1,
    ))
}

pub(crate) fn frame(scale: u16, extent: [u32; 2]) -> SpriteFrame<'static> {
    let scale = f32::from(scale);
    let [odd_x, odd_y] = extent.map(|side| f64::from(side % 2));
    SpriteFrame {
        view: SpriteView {
            centre: [
                odd_x / (2.0 * f64::from(scale)),
                -odd_y / (2.0 * f64::from(scale)),
            ],
            pixels_per_unit: scale,
        },
        rows: &ROWS,
    }
}

#[cfg(target_os = "ios")]
const MOBILE_DENSITY: f32 = 128.0;
#[cfg(target_os = "ios")]
const MOBILE_SCALE: f32 = 2.0;

#[cfg(target_os = "ios")]
pub(crate) fn add_to_mobile_field(field: &mut SpriteScene) -> Result<(), FrameError> {
    let mut card = scene(2)?.sheets.remove(0);
    card.name = "registration card".into();
    card.sheet.texels_per_unit = MOBILE_DENSITY;
    field.sheets.push(card);
    Ok(())
}

#[cfg(target_os = "ios")]
pub(crate) fn mobile_field(extent: [u32; 2]) -> SpriteFrame<'static> {
    const ROWS: [SpriteRow; 1] = [SpriteRow {
        sheet: 1,
        layer: 2,
        ..SpriteRow::new(0, [-1.0, 0.5])
    }];
    let pixels_per_unit = MOBILE_SCALE * MOBILE_DENSITY;
    let [odd_x, odd_y] = extent.map(|side| f64::from(side % 2));
    SpriteFrame {
        view: SpriteView {
            centre: [
                odd_x / (2.0 * f64::from(pixels_per_unit)),
                -odd_y / (2.0 * f64::from(pixels_per_unit)),
            ],
            pixels_per_unit,
        },
        rows: &ROWS,
    }
}

pub(crate) fn texels() -> Vec<u8> {
    const BORDER: [u8; 3] = [78, 107, 118];
    const PAPER: [u8; 3] = [21, 31, 43];
    const CREAM: [u8; 3] = [244, 229, 187];
    const GOLD: [u8; 3] = [252, 175, 62];
    const AQUA: [u8; 3] = [55, 224, 201];
    const SWATCHES: [[u8; 3]; 8] = [
        [252, 77, 90],
        [253, 144, 68],
        GOLD,
        [173, 226, 117],
        AQUA,
        [69, 155, 250],
        [121, 112, 250],
        [220, 118, 232],
    ];
    let mut rgba = Vec::with_capacity((SIZE[0] * SIZE[1] * 4) as usize);
    for y in 0..128_u8 {
        for x in 0..=255_u8 {
            let colour = match (x, y) {
                (0 | 255, _) | (_, 0 | 127) => BORDER,
                (8..=10, 8..=27) | (8..=22, 8..=10) => GOLD,
                (233..=247, 117..=119)
                | (245..=247, 100..=119)
                | (213..=234, 21..=22)
                | (223..=224, 11..=31) => AQUA,
                (31..=35, 15..=28) | (27..=39, 18..=21) | (29..=37, 16..=17) | (33, 13..=14) => {
                    CREAM
                }
                (48..=207, 16..=27) => SWATCHES[usize::from((x - 48) / 20)],
                (16..=239, 39..=50) => {
                    let value = ((x - 16) / 7).saturating_mul(8);
                    [value; 3]
                }
                (16..=239, 54..=61) => {
                    let value = (x - 16).saturating_add(32);
                    [value, 46, 80]
                }
                (16..=239, 62..=69) => {
                    let value = (x - 16).saturating_add(32);
                    [27, value, 101]
                }
                (16..=239, 70..=77) => {
                    let value = (x - 16).saturating_add(32);
                    [48, 68, value]
                }
                (16..=75, 92..=110) => {
                    if x % 2 == 0 {
                        CREAM
                    } else {
                        PAPER
                    }
                }
                (90..=149, 92..=110) => {
                    if y % 2 == 0 {
                        CREAM
                    } else {
                        PAPER
                    }
                }
                (164..=223, 92..=110) => {
                    if (x + y.wrapping_sub(92)) % 2 == 0 {
                        CREAM
                    } else {
                        PAPER
                    }
                }
                _ => PAPER,
            };
            rgba.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
        }
    }
    rgba
}
