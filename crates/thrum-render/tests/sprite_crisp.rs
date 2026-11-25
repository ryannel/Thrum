//! Verify the submitted camera sweep, aligned texel bytes and smooth-sheet minification.

#[expect(
    dead_code,
    reason = "the fixtures are shared with sprite lifecycle and ordering tests"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use serde_json::json;
use thrum_render::{
    sprite::{SpriteRect, SpriteRow, SpriteSampling, SpriteScene},
    synthetic::CaptureSource,
};

const OFFSETS: u32 = 256;
const CHECKPOINTS: [u32; 5] = [0, 64, 128, 192, OFFSETS - 1];

fn subpixel(index: u32) -> [f64; 2] {
    [index, (index * 73) % OFFSETS].map(|offset| f64::from(offset) / f64::from(OFFSETS))
}

fn srgb_to_linear(code: u8) -> f64 {
    let signal = f64::from(code) / 255.0;
    if signal <= 0.04045 {
        signal / 12.92
    } else {
        ((signal + 0.055) / 1.055).powf(2.4)
    }
}

fn marker_centroid(bytes: &[u8], width: usize) -> [f64; 2] {
    let mut mass = 0.0;
    let mut moment = [0.0; 2];
    for (index, pixel) in bytes.as_chunks::<8>().0.iter().enumerate() {
        let red = half::f16::from_le_bytes([pixel[0], pixel[1]]).to_f64();
        mass += red;
        moment[0] += red * (index % width) as f64;
        moment[1] += red * (index / width) as f64;
    }
    assert!(mass > 0.0, "the submitted marker must be visible");
    moment.map(|axis| axis / mass)
}

fn moves_diagonally(positions: &[[f64; 2]; 5]) -> bool {
    positions
        .windows(2)
        .all(|pair| pair[1][0] > pair[0][0] && pair[1][1] > pair[0][1])
}

#[expect(
    clippy::print_stdout,
    reason = "records submitted centroids for the milestone"
)]
fn verify_scroll(recipe: &str) {
    let Some(gpu) = fixture::open("submitted sprite camera sweep", recipe) else {
        return;
    };
    let extent = [16, 16];
    let art = fixture::sheet(
        [1; 2],
        &[255; 4],
        &[SpriteRect {
            origin: [0; 2],
            size: [1; 2],
        }],
    );
    for scale in [1.25_f32, 1.5, 2.37] {
        let row = SpriteRow {
            scale: [scale; 2],
            ..SpriteRow::new(0, [0.0; 2])
        };
        let mut host = fixture::host(
            &gpu.capabilities,
            &gpu.queue,
            extent,
            fixture::Scene::new(vec![row], 1),
            art.clone(),
        );
        let mut previous_submission = None;
        let mut positions = [[0.0; 2]; CHECKPOINTS.len()];
        let mut frozen = [[0.0; 2]; CHECKPOINTS.len()];
        let mut first = Vec::new();
        for step in 0..OFFSETS {
            let offset = f64::from(step) / f64::from(OFFSETS);
            host.continuous_source_mut().centre = [-offset, offset];
            let image =
                fixture::capture(&mut host, &gpu.device, extent, CaptureSource::SceneLinear);
            assert!(previous_submission.is_none_or(|old| image.sample.submission_order > old));
            previous_submission = Some(image.sample.submission_order);
            if step == 0 {
                first.clone_from(&image.rgba);
            }
            if let Some(checkpoint) = CHECKPOINTS.iter().position(|&value| value == step) {
                positions[checkpoint] = marker_centroid(&image.rgba, extent[0] as usize);
                frozen[checkpoint] = marker_centroid(&first, extent[0] as usize);
            }
        }
        assert!(
            moves_diagonally(&positions),
            "{recipe} {scale}: {positions:?}"
        );
        assert!(
            !moves_diagonally(&frozen),
            "frozen pixels satisfied the movement proof"
        );
        println!(
            "{}",
            json!({
                "recipe": recipe, "scale": scale, "submissions": OFFSETS,
                "checkpoint_steps": CHECKPOINTS, "centroids": positions,
                "frozen_centroids": frozen, "frozen_rejected": true,
            })
        );
    }
}

#[test]
fn fractional_camera_scroll_on_portable() {
    verify_scroll("portable");
}

#[test]
fn fractional_camera_scroll_on_apple_tile() {
    verify_scroll("apple-tile");
}

#[test]
fn whole_pixel_scroll_equals_shifted_authored_texels() {
    let size = [3, 2];
    let texels = [
        [247, 4, 70, 255],
        [129, 228, 5, 255],
        [43, 82, 156, 255],
        [17, 53, 29, 255],
        [19, 229, 201, 255],
        [250, 247, 24, 255],
    ];
    let extent = [48, 36];
    let sheet = fixture::sheet(
        size,
        texels.as_flattened(),
        &[SpriteRect {
            origin: [0, 0],
            size,
        }],
    );
    for recipe in fixture::RECIPES {
        let Some(gpu) = fixture::open("whole pixel sprite scroll", recipe) else {
            return;
        };
        let mut host = fixture::host(
            &gpu.capabilities,
            &gpu.queue,
            extent,
            fixture::Scene::new(vec![SpriteRow::new(0, [0.0, 0.0])], 1),
            sheet.clone(),
        );
        for scale in 1..=6_u8 {
            host.source_mut().scale = f32::from(scale);
            for shift in [0, 1, -3, 5] {
                let left = 20 - shift;
                let top = 14 + shift;
                host.source_mut().centre = [
                    f64::from(24 - left) / f64::from(scale),
                    f64::from(top - 18) / f64::from(scale),
                ];
                let image = fixture::capture(&mut host, &gpu.device, extent, CaptureSource::Sdr);
                for (index, pixel) in image.rgba.as_chunks::<4>().0.iter().enumerate() {
                    let u = (i32::try_from(index % 48).expect("column") - left)
                        .div_euclid(i32::from(scale));
                    let v = (i32::try_from(index / 48).expect("row") - top)
                        .div_euclid(i32::from(scale));
                    let expected = if (0..3).contains(&u) && (0..2).contains(&v) {
                        texels[usize::try_from(v * 3 + u).expect("inside source")]
                    } else {
                        [0, 0, 0, 255]
                    };
                    assert_eq!(
                        *pixel, expected,
                        "{recipe}: {scale}× scroll {shift}, pixel {index}"
                    );
                }
            }
        }
    }
}

#[test]
fn quarter_size_smooth_checker_preserves_its_linear_mean() {
    let size = [32, 32];
    let art: Vec<_> = (0..32)
        .flat_map(|y| {
            (0..32).flat_map(move |x| {
                let code = if (x + y) % 2 == 0 { 32 } else { 224 };
                [code, code, code, 255]
            })
        })
        .collect();
    let mean = f64::midpoint(srgb_to_linear(32), srgb_to_linear(224));
    let expected = (mean.powf(1.0 / 2.4).mul_add(1.055, -0.055) * 255.0).round();
    let sheet = fixture::sheet(
        size,
        &art,
        &[SpriteRect {
            origin: [0; 2],
            size,
        }],
    );
    let extent = [160, 160];
    let rows: Vec<_> = (0..OFFSETS)
        .map(|offset| {
            let [dx, dy] = subpixel(offset);
            let mut row = SpriteRow::new(
                0,
                [
                    f64::from(offset % 16 * 10 + 1) + dx - 80.0,
                    80.0 - f64::from(offset / 16 * 10 + 1) - dy,
                ],
            );
            row.scale = [0.25; 2];
            row
        })
        .collect();
    for recipe in fixture::RECIPES {
        let Some(gpu) = fixture::open("smooth checker", recipe) else {
            return;
        };
        let mut config = SpriteScene::new(sheet.clone(), OFFSETS);
        config.sheets[0].sampling = SpriteSampling::Smooth;
        config.sheets[0].tiles = vec![0];
        let mut host = fixture::configured_host(
            &gpu.capabilities,
            &gpu.queue,
            extent,
            fixture::Scene::new(rows.clone(), 1),
            config,
        );
        let image = fixture::capture(&mut host, &gpu.device, extent, CaptureSource::Sdr);
        for offset in 0..OFFSETS {
            let [dx, dy] = subpixel(offset);
            let origin = [
                f64::from(offset % 16 * 10 + 1) + dx,
                f64::from(offset / 16 * 10 + 1) + dy,
            ];
            for y in offset / 16 * 10..offset / 16 * 10 + 10 {
                for x in offset % 16 * 10..offset % 16 * 10 + 10 {
                    let local = [
                        f64::from(x) + 0.5 - origin[0],
                        f64::from(y) + 0.5 - origin[1],
                    ];
                    if local.into_iter().all(|v| (0.01..7.99).contains(&v)) {
                        let i = ((y * extent[0] + x) * 4) as usize;
                        for c in 0..3 {
                            assert!(
                                (f64::from(image.rgba[i + c]) - expected).abs() <= 1.0,
                                "{recipe} checker offset {offset}, pixel {x},{y}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn two_x_colour_crossing_beside_transparency_is_continuous() {
    let extent = [16; 2];
    let size = [2; 2];
    let art = fixture::sheet(
        size,
        &[255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0],
        &[SpriteRect {
            origin: [0; 2],
            size,
        }],
    );
    for recipe in fixture::RECIPES {
        let Some(gpu) = fixture::open("2× colour crossing", recipe) else {
            return;
        };
        let mut host = fixture::host(
            &gpu.capabilities,
            &gpu.queue,
            extent,
            fixture::Scene::new(vec![SpriteRow::new(0, [0.0; 2])], 2),
            art.clone(),
        );
        // At the witness pixel's local Y of 0.75, the stagger shifts the
        // projected edge by 1/192 source texel. Probe both shoulder ends and
        // its interior so the continuity check cannot skip the blend.
        let crossing = 1.0 - 1.0 / 192.0;
        let samples = [
            0.75,
            crossing - 0.00135,
            crossing - 0.00127,
            crossing - 0.000015,
            crossing,
            crossing + 0.000015,
            crossing + 0.00127,
            crossing + 0.00135,
            1.25,
        ]
        .map(|u| {
            host.continuous_source_mut().centre = [u - 0.75, 0.0];
            let image = fixture::capture(&mut host, &gpu.device, extent, CaptureSource::Sdr);
            image.rgba.as_chunks::<4>().0[9 * 16 + 9]
        });
        assert_eq!(samples[0], [255, 0, 0, 255]);
        assert_eq!(samples[8], [0, 0, 255, 255]);
        assert!(
            (1..255).contains(&samples[4][0]) && (1..255).contains(&samples[4][2]),
            "{recipe}: the shoulder centre did not blend: {samples:?}"
        );
        for (a, b) in [(1, 2), (3, 4), (4, 5), (6, 7)] {
            for channel in [0, 2] {
                assert!(
                    samples[a][channel].abs_diff(samples[b][channel]) <= 4,
                    "{recipe}: adjacent samples jumped: {samples:?}"
                );
            }
        }
        for pair in samples.windows(2) {
            assert!(
                pair[0][0] >= pair[1][0] && pair[0][2] <= pair[1][2],
                "{recipe}: the crossing reversed: {samples:?}"
            );
        }
        println!("{recipe}: submitted 2× colour crossing {samples:?}");
    }
}
