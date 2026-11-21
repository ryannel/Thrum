//! Submitted SDR equality over odd/even extents, clips and whole texel scales.

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

const ART: [u8; 24] = [
    230, 60, 13, 255, 15, 232, 74, 255, 153, 21, 240, 255, 61, 22, 190, 255, 230, 225, 34, 255, 31,
    167, 204, 255,
];
const RECT: SpriteRect = SpriteRect {
    origin: [0; 2],
    size: [3, 2],
};

#[test]
fn clips_each_edge_and_keeps_asymmetric_texels_exact_on_both_recipes() {
    for recipe in fixture::RECIPES {
        let Some(thrum_test_support::Opened {
            device,
            queue,
            capabilities: caps,
            ..
        }) = fixture::open("sprite exact", recipe)
        else {
            return;
        };
        let mut host = fixture::host(
            &caps,
            &queue,
            [64, 46],
            fixture::Scene::new(vec![], 1),
            fixture::sheet([3, 2], &ART, &[RECT]),
        );
        for [width, height] in [[64_i32, 46_i32], [65, 47]] {
            let extent = [
                u32::try_from(width).expect("bounded fixture coordinate"),
                u32::try_from(height).expect("bounded fixture coordinate"),
            ];
            assert!(
                host.resize_scene_surface(extent)
                    .expect("resize should be admitted")
            );
            for scale in [1_u8, 2, 3, 4, 6] {
                let s = i32::from(scale);
                let corners = [
                    [-s, 3],
                    [width - 2 * s, 3],
                    [9, -s],
                    [9, height - s],
                    [width / 2 - s, height / 2 - s],
                ];
                let rows = corners
                    .iter()
                    .zip(0_u32..)
                    .map(|([x, y], order)| {
                        let mut row = SpriteRow::new(
                            0,
                            [
                                f64::from(x - width / 2) / f64::from(s),
                                f64::from(height / 2 - y) / f64::from(s),
                            ],
                        );
                        // Preserve the original test card's authored overlap order.
                        row.sort = Some(f64::from(order));
                        row
                    })
                    .collect();
                *host.source_mut() = fixture::Scene::new(rows, scale);
                let image = fixture::capture(&mut host, &device, extent, CaptureSource::Sdr);
                let mut cpu = Vec::new();
                for y in 0..height {
                    for x in 0..width {
                        let mut code = &[0, 0, 0, 255][..];
                        for [left, top] in corners {
                            let u = (x - left).div_euclid(s);
                            let v = (y - top).div_euclid(s);
                            if (0..3).contains(&u) && (0..2).contains(&v) {
                                let start = usize::try_from((v * 3 + u) * 4)
                                    .expect("bounded fixture coordinate");
                                code = &ART[start..start + 4];
                            }
                        }
                        cpu.extend_from_slice(code);
                    }
                }
                assert_eq!(image.rgba, cpu, "{recipe} {width}x{height} scale={scale}");
                println!(
                    "{recipe} {width}x{height} scale={scale}: {} exact submitted pixels",
                    cpu.len() / 4
                );
            }
        }
    }
}
