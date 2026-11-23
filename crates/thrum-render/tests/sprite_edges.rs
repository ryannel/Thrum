//! Verify premultiplied edges, frame isolation and shared tile edges in submitted pixels.

#[expect(
    dead_code,
    reason = "the fixtures are shared with sprite lifecycle and ordering tests"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;
use thrum_fixtures::sprite_edges::{self, CYAN, WHITE, YELLOW};

use thrum_render::{
    sprite::{EmbeddedSheet, SpriteRect, SpriteRow, SpriteSampling, SpriteScene},
    synthetic::CaptureSource,
};

const SCALES: [f32; 10] = [0.25, 0.375, 0.5, 0.75, 1.0, 1.25, 1.5, 2.37, 4.0, 6.0];

fn row(frame: u32, [x, y]: [f64; 2], scale: f32, rotation: f32, order: i32) -> SpriteRow {
    SpriteRow {
        scale: [scale; 2],
        rotation,
        layer: order,
        ..SpriteRow::new(frame, [x, -y])
    }
}

fn witness_rows(offset: u32, extent: [u32; 2]) -> Vec<SpriteRow> {
    let dx = f64::from(offset) / 16.0;
    let dy = f64::from((offset * 7) % 16) / 16.0;
    let [half_w, half_h] = extent.map(|v| f64::from(v) * 0.5);
    SCALES
        .into_iter()
        .zip(0_u32..)
        .flat_map(|(scale, i)| {
            let x = f64::from(i % 5 * 106) - half_w;
            let y = f64::from(i / 5 * 156) - half_h;
            let length = f64::from(scale) * 8.0;
            [
                row(9, [x, y], 6.0, 0.0, 0),
                row(
                    8,
                    [x + 14.0 + dx, y + 2.0 + dy],
                    scale * 0.75,
                    0.17 + offset as f32 * 0.079,
                    1,
                ),
                row(5, [x + 5.0 + dx, y + 52.0 + dy], scale, 0.0, 1),
                row(0, [x, y + 104.0], 12.0, 0.0, 0),
                row(10, [x + 2.0 + dx, y + 106.0 + dy], scale, 0.0, 1),
                row(11, [x + 2.0 + dx + length, y + 106.0 + dy], scale, 0.0, 1),
            ]
        })
        .collect()
}

fn check_frame(
    image: &thrum_render::runtime::SubmittedImage,
    offset: u32,
    sampling: SpriteSampling,
) {
    let dx = f64::from(offset) / 16.0;
    let dy = f64::from((offset * 7) % 16) / 16.0;
    let w = image.extent[0] as usize;
    let pixel = |x: usize, y: usize| image.rgba.as_chunks::<4>().0[y * w + x];
    for (i, scale) in SCALES.into_iter().enumerate() {
        let x = i % 5 * 106;
        let y = i / 5 * 156;
        for py in y..y + 48 {
            for px in x..x + 48 {
                assert_eq!(
                    pixel(px, py),
                    WHITE,
                    "{sampling:?} white outline at {scale}, {offset}, {px},{py}"
                );
            }
        }
        let mut visible = false;
        for py in y + 50..y + 102 {
            for px in x + 3..x + 56 {
                let [r, g, b, _] = pixel(px, py);
                assert_eq!(
                    r, 0,
                    "{sampling:?} neighbour bleed at {scale}, {offset}, {px},{py}"
                );
                assert_eq!(g, b);
                visible |= g > 0;
            }
        }
        assert!(
            visible,
            "{sampling:?} central frame never appeared at {scale}"
        );
        let start = [x as f64 + 2.0 + dx, y as f64 + 106.0 + dy];
        let span = f64::from(scale) * 8.0;
        let mut shared = 0;
        for py in y + 105..y + 155 {
            for px in x + 1..x + 100 {
                let u = px as f64 + 0.5 - start[0];
                let v = py as f64 + 0.5 - start[1];
                if (0.01..2.0 * span - 0.01).contains(&u) && (0.01..span - 0.01).contains(&v) {
                    let output = pixel(px, py);
                    assert!(
                        matches!(output, CYAN | YELLOW),
                        "{sampling:?} tile gap {output:?} at {scale}, {offset}, {px},{py}"
                    );
                    shared += usize::from((u - span).abs() <= 0.5);
                }
            }
        }
        assert!(shared > 0);
    }
}

#[test]
fn outlines_mips_and_tiles_remain_clean_on_metal() {
    let extent = [530, 312];
    let (rgba, origins) = sprite_edges::art();
    let art = EmbeddedSheet {
        extent: sprite_edges::EXTENT,
        rgba: rgba.into(),
        frames: origins
            .map(|origin| SpriteRect {
                origin,
                size: sprite_edges::FRAME_SIZE,
            })
            .into(),
        texels_per_unit: 1.0,
    };
    for recipe in fixture::RECIPES {
        let Some(gpu) = fixture::open("sprite edges", recipe) else {
            return;
        };
        for sampling in [SpriteSampling::Pixel, SpriteSampling::Smooth] {
            let mut config = SpriteScene::new(art.clone(), 60);
            config.sheets[0].sampling = sampling;
            config.sheets[0].tiles = vec![0, 9, 10, 11];
            let mut host = fixture::configured_host(
                &gpu.capabilities,
                &gpu.queue,
                extent,
                fixture::Scene::new(Vec::new(), 1),
                config,
            );
            for offset in 0..16 {
                host.source_mut().rows = witness_rows(offset, extent);
                let image = fixture::capture(&mut host, &gpu.device, extent, CaptureSource::Sdr);
                check_frame(&image, offset, sampling);
            }
        }
        println!(
            "{recipe}: white outlines, frame isolation and shared tile coverage, both kinds, ten scales and sixteen offsets"
        );
    }
}
