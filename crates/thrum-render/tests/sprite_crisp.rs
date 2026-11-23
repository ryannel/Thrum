//! Measure coverage at the scene-linear tap and compare whole-pixel scroll at the SDR tap.

#[expect(
    dead_code,
    reason = "the fixtures are shared with sprite lifecycle and ordering tests"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use serde_json::{Value, json};
use thrum_render::{
    sprite::{NamedSheet, SpriteRect, SpriteRow, SpriteSampling, SpriteScene},
    synthetic::CaptureSource,
};

const SCALES: [f32; 8] = [1.0, 1.25, 1.5, 2.0, 2.37, 3.0, 4.0, 6.0];
const MODES: [SpriteSampling; 3] = [
    SpriteSampling::Pixel,
    SpriteSampling::NearestControl,
    SpriteSampling::BilinearControl,
];
const OFFSETS: u32 = 256;
const ART_SIDE: u32 = 4;
const WEIGHT: SpriteRect = SpriteRect {
    origin: [0, 4],
    size: [1, 1],
};

fn random_art() -> Vec<u8> {
    let mut state = 0xC493_671A_u32;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state.to_le_bytes()[0]
    };
    (0..ART_SIDE.pow(2))
        .flat_map(|_| [next(), next(), next(), 255])
        .chain([255; 4])
        .chain([0; 12])
        .collect()
}

fn srgb_to_linear(code: u8) -> f64 {
    let signal = f64::from(code) / 255.0;
    if signal <= 0.04045 {
        signal / 12.92
    } else {
        ((signal + 0.055) / 1.055).powf(2.4)
    }
}

fn box_pixel(art: &[[f64; 3]], scale: f64, origin: [f64; 2], xy: [u32; 2]) -> [f64; 3] {
    let overlap = |axis: usize, texel: u32| {
        let start = origin[axis] + f64::from(texel) * scale;
        let pixel = f64::from(xy[axis]);
        ((start + scale).min(pixel + 1.0) - start.max(pixel)).max(0.0)
    };
    let mut result = [0.0; 3];
    for y in 0..ART_SIDE {
        for x in 0..ART_SIDE {
            let weight = overlap(0, x) * overlap(1, y);
            let index = (y * ART_SIDE + x) as usize;
            for (channel, total) in result.iter_mut().enumerate() {
                *total += art[index][channel] * weight;
            }
        }
    }
    result
}

fn channel(bytes: &[u8], [width, _]: [u32; 2], x: u32, y: u32, channel: usize) -> f64 {
    let offset = ((y * width + x) * 8) as usize + channel * 2;
    f64::from(half::f16::from_le_bytes([bytes[offset], bytes[offset + 1]]).to_f32())
}

#[derive(Default)]
struct Figures {
    pixel: f64,
    pixel_at: Option<Value>,
    weight: f64,
    weight_at: Option<Value>,
}

fn take_case(
    figures: &mut Figures,
    art: &[[f64; 3]],
    image: &thrum_render::runtime::SubmittedImage,
    scale: f64,
    cell: [u32; 2],
    offset: u32,
    cell_side: u32,
) {
    let [x, y] = cell;
    let position = subpixel(offset);
    let origin = [
        f64::from(x + 4) + position[0],
        f64::from(y + 4) + position[1],
    ];
    for py in y..y + cell_side {
        for px in x..x + cell_side {
            for (c, expected) in box_pixel(art, scale, origin, [px, py])
                .into_iter()
                .enumerate()
            {
                let actual = channel(&image.rgba, image.extent, px, py, c);
                let error = (actual - expected).abs();
                if error > figures.pixel {
                    figures.pixel = error;
                    figures.pixel_at = Some(json!({"offset": offset, "pixel": [px - x, py - y],
                            "channel": c, "expected": expected, "measured": actual}));
                }
            }
        }
    }
    let mut weight = 0.0;
    for py in y + cell_side..y + 2 * cell_side {
        for px in x..x + cell_side {
            weight += channel(&image.rgba, image.extent, px, py, 0);
        }
    }
    let error = (weight - scale * scale).abs();
    if error > figures.weight {
        figures.weight = error;
        figures.weight_at =
            Some(json!({"offset": offset, "expected": scale * scale, "measured": weight}));
    }
}

fn subpixel(index: u32) -> [f64; 2] {
    [index, (index * 73) % OFFSETS].map(|offset| f64::from(offset) / f64::from(OFFSETS))
}

fn measure(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &thrum_render::capabilities::DeviceCapabilities,
    scale: f32,
) -> Vec<Figures> {
    let art = random_art();
    let sheet = fixture::sheet(
        [ART_SIDE, ART_SIDE + 1],
        &art,
        &[
            SpriteRect {
                origin: [0; 2],
                size: [ART_SIDE; 2],
            },
            WEIGHT,
        ],
    );
    let side = 32;
    let linear: Vec<_> = art[..64]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|rgba| std::array::from_fn(|channel| srgb_to_linear(rgba[channel])))
        .collect();
    let mode_count = u32::try_from(MODES.len()).expect("three modes");
    let extent = [16 * side * mode_count, 32 * side];
    let cell =
        |mode: u32, offset: u32| [side * (16 * mode + offset % 16), 2 * side * (offset / 16)];
    let mut rows = Vec::new();
    let mut config = SpriteScene::new(sheet.clone(), 2 * OFFSETS * mode_count);
    config.sheets = MODES
        .iter()
        .map(|sampling| NamedSheet {
            sampling: *sampling,
            ..NamedSheet::pixel(format!("{sampling:?}"), sheet.clone())
        })
        .collect();
    for slot in 0..mode_count {
        for offset in 0..OFFSETS {
            let [x, y] = cell(slot, offset);
            let [dx, dy] = subpixel(offset);
            for (frame, top) in [(0, y), (1, y + side)] {
                let mut row = SpriteRow::new(
                    frame,
                    [
                        f64::from(x + 4) + dx - f64::from(extent[0]) * 0.5,
                        f64::from(extent[1]) * 0.5 - f64::from(top + 4) - dy,
                    ],
                );
                row.sheet = slot;
                row.scale = [scale; 2];
                rows.push(row);
            }
        }
    }
    // Spatially batch translations of the same view, separated by black gutters,
    // so one Metal submission measures all 256 positions of each mode.
    let mut host =
        fixture::configured_host(caps, queue, extent, fixture::Scene::new(rows, 1), config);
    let image = fixture::capture(&mut host, device, extent, CaptureSource::SceneLinear);
    (0..mode_count)
        .map(|mode| {
            let mut figures = Figures::default();
            for offset in 0..OFFSETS {
                take_case(
                    &mut figures,
                    &linear,
                    &image,
                    f64::from(scale),
                    cell(mode, offset),
                    offset,
                    side,
                );
            }
            figures
        })
        .collect()
}

#[test]
fn records_scene_linear_coverage_for_both_metal_recipes() {
    if std::env::var_os("THRUM_MEASURE_SPRITE_COVERAGE").is_none() {
        return;
    }
    for recipe in fixture::RECIPES {
        let Some(gpu) = fixture::open("sprite coverage record", recipe) else {
            return;
        };
        for scale in SCALES {
            let results = measure(&gpu.device, &gpu.queue, &gpu.capabilities, scale);
            if scale.fract() != 0.0 {
                assert!(results[0].pixel < results[1].pixel && results[0].pixel < results[2].pixel);
            }
            for (mode, figures) in MODES.into_iter().zip(results) {
                println!(
                    "coverage {}",
                    json!({"recipe": recipe, "scale": scale, "sampling": format!("{mode:?}"),
                    "pixel_error": figures.pixel, "pixel_at": figures.pixel_at,
                    "weight_error": figures.weight, "weight_at": figures.weight_at})
                );
            }
        }
    }
}

fn scroll_weights(gpu: &thrum_test_support::Opened) -> [[f64; 2]; 3] {
    let extent = [48, 8];
    let art = fixture::sheet(
        [1; 2],
        &[255; 4],
        &[SpriteRect {
            origin: [0; 2],
            size: [1; 2],
        }],
    );
    let mut config = SpriteScene::new(art.clone(), 6);
    config.sheets.push(NamedSheet {
        sampling: SpriteSampling::NearestControl,
        ..NamedSheet::pixel("nearest", art)
    });
    let scales = [1.25_f32, 1.5, 2.37];
    let rows = scales
        .into_iter()
        .enumerate()
        .flat_map(|(i, scale)| {
            (0..2_u32).map(move |sheet| {
                let position = [(i * 16 + sheet as usize * 8 + 2) as f64 - 24.0, 2.0];
                SpriteRow {
                    sheet,
                    scale: [scale; 2],
                    ..SpriteRow::new(0, position)
                }
            })
        })
        .collect();
    let mut host = fixture::configured_host(
        &gpu.capabilities,
        &gpu.queue,
        extent,
        fixture::Scene::new(rows, 1),
        config,
    );
    let mut maxima = [[0.0_f64; 2]; 3];
    for offset in 0..OFFSETS {
        let [x, y] = subpixel(offset);
        host.source_mut().centre = [-x, y];
        let image = fixture::capture(&mut host, &gpu.device, extent, CaptureSource::SceneLinear);
        for (index, scale) in scales.into_iter().enumerate() {
            for mode in 0..2_u32 {
                let left = (u32::try_from(index).expect("three scales") * 2 + mode) * 8;
                let mut measured = 0.0;
                for y in 0..8 {
                    for x in left..left + 8 {
                        measured += channel(&image.rgba, extent, x, y, 0);
                    }
                }
                let error = (measured - f64::from(scale).powi(2)).abs();
                let maximum = &mut maxima[index][mode as usize];
                *maximum = maximum.max(error);
            }
        }
    }
    maxima
}

#[expect(
    clippy::print_stdout,
    reason = "records device measurements for the milestone proof"
)]
fn verify_scroll(recipe: &str) {
    let bounds: Value = serde_json::from_str(include_str!("support/coverage-mac-metal.json"))
        .expect("retained Metal sweep");
    let Some(gpu) = fixture::open("sprite coverage scroll", recipe) else {
        return;
    };
    let bound = bounds["bounds"][recipe]
        .as_f64()
        .expect("measured recipe bound");
    for (scale, [coverage, nearest]) in [1.25, 1.5, 2.37].into_iter().zip(scroll_weights(&gpu)) {
        assert!(
            coverage <= bound,
            "{recipe} {scale}: observed {coverage}, retained {bound}"
        );
        assert!(
            nearest >= 1.0 && nearest > bound,
            "{recipe} {scale}: nearest did not lose or gain an output pixel"
        );
        println!(
            "{recipe} {scale}: coverage error {coverage}, nearest error {nearest}, retained bound {bound}"
        );
    }
}

#[test]
fn fractional_camera_scroll_conserves_weight_on_portable() {
    verify_scroll("portable");
}

#[test]
fn fractional_camera_scroll_conserves_weight_on_apple_tile() {
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
