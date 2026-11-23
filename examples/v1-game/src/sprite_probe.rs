//! Authored scenes used to inspect the ordinary sprite path.

use thrum_render::{
    extract::SourceTick,
    sprite::{
        EmbeddedSheet, NamedSheet, SpriteFrame, SpriteRect, SpriteRow, SpriteSampling, SpriteScene,
        SpriteSink, SpriteView,
    },
};

/// Select a test scene in place of the V1 sprite registration card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteProbe {
    /// A grid of isolated texels translated through each of 256 subpixel positions.
    Coverage(f32),
    /// Pixel sprites, adjacent ground tiles, and the registration card moving together.
    Scroll {
        /// Output pixels per world unit.
        scale: f32,
        /// Distance travelled along X per simulation tick, in output pixels.
        speed: f32,
    },
    /// Comparison of pixel and smooth sprites over light and dark panels.
    Stills,
}

pub(crate) struct Scene {
    probe: SpriteProbe,
    rows: Vec<SpriteRow>,
}

impl Scene {
    pub(crate) fn new(probe: SpriteProbe) -> Self {
        let rows = match probe {
            SpriteProbe::Coverage(scale) => coverage_rows(scale),
            SpriteProbe::Scroll { .. } => scroll_rows(),
            SpriteProbe::Stills => still_rows(),
        };
        Self { probe, rows }
    }

    pub(crate) fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
    ) -> SpriteFrame<'_> {
        let (centre, pixels_per_unit) = match self.probe {
            SpriteProbe::Coverage(_) => (
                [f64::from(extent[0]) * 0.5, -f64::from(extent[1]) * 0.5],
                1.0,
            ),
            SpriteProbe::Scroll { scale, speed } => {
                let travelled = (source.tick.saturating_sub(1) as f64 + f64::from(alpha))
                    * f64::from(speed / scale);
                ([travelled, travelled * 0.53], scale)
            }
            SpriteProbe::Stills => ([0.0; 2], 1.0),
        };
        SpriteFrame {
            view: SpriteView {
                centre,
                pixels_per_unit,
            },
            rows: &self.rows,
        }
    }

    pub(crate) fn extract(&self, sink: &mut SpriteSink<'_>, tick: SourceTick) {
        if !matches!(self.probe, SpriteProbe::Scroll { .. }) {
            return;
        }
        for slot in 0..56_u32 {
            let x = f64::from(slot % 14) * 52.0 - 360.0;
            let y = f64::from(slot / 14) * -33.0 - 23.0;
            let speed = 0.006 + 0.002 * (slot % 5) as f32;
            let start = slot as f32 * 0.51;
            let pose = |tick: u64| (start + tick as f32 * speed).rem_euclid(std::f32::consts::TAU);
            let mut row = SpriteRow::new(3, [x, y]);
            row.rotation = if slot % 3 == 0 { pose(tick.tick) } else { 0.0 };
            row.layer = 2;
            let mut previous = thrum_render::sprite::SpritePose::from(row);
            if slot % 3 == 0 {
                previous.rotation = pose(tick.tick.saturating_sub(1));
            }
            sink.push_pair(slot, 1, u128::from(slot), row, Some(previous));
        }
    }
}

pub(crate) fn assets(probe: SpriteProbe) -> SpriteScene {
    if matches!(probe, SpriteProbe::Coverage(_)) {
        let art = EmbeddedSheet {
            extent: [1; 2],
            rgba: [255; 4].into(),
            frames: [SpriteRect {
                origin: [0; 2],
                size: [1; 2],
            }]
            .into(),
            texels_per_unit: 1.0,
        };
        let mut scene = SpriteScene::new(art.clone(), 512);
        scene.sheets.push(NamedSheet {
            sampling: SpriteSampling::NearestControl,
            ..NamedSheet::pixel("nearest control", art)
        });
        return scene;
    }
    let mut scene = SpriteScene::new(pixel_sheet(), 5000);
    scene.sheets[0].tiles = vec![1, 2, 4];
    scene.sheets.push(NamedSheet {
        sampling: SpriteSampling::Smooth,
        ..NamedSheet::pixel("smooth test art", painted_sheet())
    });
    if matches!(probe, SpriteProbe::Scroll { .. }) {
        scene.world_capacity = 56;
    }
    scene
}

fn coverage_rows(scale: f32) -> Vec<SpriteRow> {
    (0..2_u32)
        .flat_map(|sheet| {
            (0..256_u32).map(move |offset| {
                let x = f64::from((sheet * 16 + offset % 16) * 8 + 2) + f64::from(offset) / 256.0;
                let y = f64::from(offset / 16 * 8 + 2) + f64::from(offset * 73 % 256) / 256.0;
                SpriteRow {
                    sheet,
                    scale: [scale; 2],
                    ..SpriteRow::new(0, [x, -y])
                }
            })
        })
        .collect()
}

fn pixel_sheet() -> EmbeddedSheet {
    let mut bytes = crate::sprite_card::texels();
    for y in 0..32_i32 {
        for x in 0..256_i32 {
            let u = x % 32;
            let texel = match x / 32 {
                0 | 1 => {
                    let light = (u / 4 + y / 4) % 2 == 0;
                    match (x / 32, light) {
                        (0, true) => [33, 62, 72, 255],
                        (1, true) => [43, 82, 93, 255],
                        (0, false) => [26, 51, 59, 255],
                        _ => [33, 67, 77, 255],
                    }
                }
                2 => pixel_art(u - 16, y - 16),
                3 => [255; 4],
                4 => pixel_art((u - 16) * 2, (y - 16) * 2),
                _ => [0; 4],
            };
            bytes.extend(texel);
        }
    }
    let mut frames = vec![SpriteRect {
        origin: [0; 2],
        size: crate::sprite_card::SIZE,
    }];
    frames.extend((0..5).map(|i| SpriteRect {
        origin: [i * 32, 128],
        size: [32; 2],
    }));
    EmbeddedSheet {
        extent: [256, 160],
        rgba: bytes.into(),
        frames: frames.into(),
        texels_per_unit: 1.0,
    }
}

fn pixel_art(x: i32, y: i32) -> [u8; 4] {
    let radius = x.abs() + y.abs();
    match (radius, x, y) {
        (0..=3, _, _) => [255, 239, 183, 255],
        (4..=10, _, -12..=0) => [254, 190, 96, 255],
        (4..=10, _, _) => [221, 122, 72, 255],
        (11..=13, _, _) => [113, 67, 65, 255],
        _ => [255, 0, 255, 0],
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the authored colour expressions lie within the byte range"
)]
fn painted_sheet() -> EmbeddedSheet {
    let mut bytes = Vec::new();
    for y in 0..64 {
        for x in 0..64 {
            let [dx, dy] = [f64::from(x) - 31.5, f64::from(y) - 31.5];
            let r = (dx * 0.93).hypot(dy * 1.15);
            let alpha = ((26.5 - r) * 127.0).clamp(0.0, 255.0) as u8;
            let light = ((dx + dy + 56.0) / 112.0).clamp(0.0, 1.0);
            let ridge = if (dy - dx * 0.35).abs() < 1.4 { 35 } else { 0 };
            let blue = (92.0 + 116.0 * (1.0 - light)) as u8;
            bytes.extend([
                31 + ridge,
                (86.0 + 125.0 * (1.0 - light)) as u8,
                blue,
                alpha,
            ]);
        }
    }
    EmbeddedSheet {
        extent: [64; 2],
        rgba: bytes.into(),
        frames: [SpriteRect {
            origin: [0; 2],
            size: [64; 2],
        }]
        .into(),
        texels_per_unit: 1.0,
    }
}

fn scroll_rows() -> Vec<SpriteRow> {
    let mut rows = Vec::new();
    for y in -24..17_i32 {
        for x in -27..43_i32 {
            rows.push(SpriteRow::new(
                1 + u32::from((x + y).rem_euclid(2) != 0),
                [f64::from(x * 32), f64::from(y * 32 + 32)],
            ));
        }
    }
    let mut card = SpriteRow::new(0, [-128.0, 156.0]);
    card.layer = 1;
    rows.push(card);
    rows
}

fn still_rows() -> Vec<SpriteRow> {
    let mut rows = Vec::new();
    for (half, y) in [(0, 157.0), (1, -4.0)] {
        let tone = if half == 0 {
            [0.83, 0.87, 0.95, 1.0]
        } else {
            [0.022, 0.037, 0.054, 1.0]
        };
        rows.push(SpriteRow {
            tint: tone,
            scale: [18.0, 4.8],
            layer: -1,
            ..SpriteRow::new(4, [-288.0, y])
        });
        for (x, scale) in [-235.0, -171.0, -107.0, -37.0, 58.0]
            .into_iter()
            .zip([1.0, 1.5, 2.37, 4.0, 6.0])
        {
            rows.push(SpriteRow {
                layer: 1,
                scale: [scale; 2],
                ..SpriteRow::new(
                    5,
                    [
                        x - f64::from(scale) * 16.0,
                        y - 39.0 + f64::from(scale) * 16.0,
                    ],
                )
            });
        }
        for (x, scale) in [-150.0, -90.0, -10.0, 175.0]
            .into_iter()
            .zip([0.25, 0.5, 1.0, 2.0])
        {
            rows.push(SpriteRow {
                sheet: 1,
                layer: 1,
                scale: [scale; 2],
                ..SpriteRow::new(
                    0,
                    [
                        x - f64::from(scale) * 32.0,
                        y - 104.0 + f64::from(scale) * 32.0,
                    ],
                )
            });
        }
    }
    rows
}

pub(crate) fn parse(selection: &str) -> Result<SpriteProbe, String> {
    let number = |text: &str, bounds: std::ops::RangeInclusive<f32>| {
        let number = text.parse::<f32>().map_err(|error| error.to_string())?;
        bounds
            .contains(&number)
            .then_some(number)
            .ok_or_else(|| format!("invalid sprite proof value {text}"))
    };
    if selection == "stills" {
        return Ok(SpriteProbe::Stills);
    }
    if let Some(scale) = selection.strip_prefix("coverage:") {
        return Ok(SpriteProbe::Coverage(number(scale, 1.0..=6.0)?));
    }
    if let Some((scale, speed)) = selection
        .strip_prefix("scroll:")
        .and_then(|args| args.split_once(':'))
    {
        return Ok(SpriteProbe::Scroll {
            scale: number(scale, 1.0..=6.0)?,
            speed: number(speed, 0.01..=12.0)?,
        });
    }
    Err(format!("unknown sprite probe {selection}"))
}
