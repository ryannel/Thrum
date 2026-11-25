//! Submitted SDR and scene-linear checks for the display-referred output half.

#[expect(
    dead_code,
    reason = "each sprite proof uses a subset of the shared host fixture"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use thrum_render::{
    sprite::{EmbeddedSheet, SpriteRect, SpriteRow, SpriteScene},
    synthetic::CaptureSource,
};

const BACK: [u8; 4] = [48, 60, 72, 255];
const FRONT: [u8; 4] = [222, 154, 91, 128];
const BRIGHT: [u8; 4] = [255, 128, 64, 255];

fn decode(code: u8) -> f64 {
    let value = f64::from(code) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(value: f64) -> f64 {
    255.0
        * if value <= 0.003_130_8 {
            value * 12.92
        } else {
            value.powf(1.0 / 2.4).mul_add(1.055, -0.055)
        }
}

const EXTENT: [u32; 2] = [40, 32];

fn colour_card() -> (Vec<u8>, [[u8; 4]; 7], EmbeddedSheet, Vec<SpriteRow>) {
    let mut art: Vec<_> = (0..=255_u8)
        .flat_map(|code| [code, code, code, 255])
        .collect();
    let saturated = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
        [0, 255, 255, 255],
        [255, 0, 255, 255],
        [21, 233, 111, 255],
    ];
    art.extend(saturated.into_iter().flatten());
    art.extend(BACK);
    art.extend(FRONT);
    art.extend(BRIGHT);
    art.resize(16 * 17 * 4, 255);
    let rect = |x| SpriteRect {
        origin: [x, 16],
        size: [1, 1],
    };
    let sheet = fixture::sheet(
        [16, 17],
        &art,
        &[
            SpriteRect {
                origin: [0, 0],
                size: [16, 16],
            },
            SpriteRect {
                origin: [0, 16],
                size: [7, 1],
            },
            rect(7),
            rect(8),
            rect(9),
        ],
    );
    let mut rows = vec![
        SpriteRow::new(0, [-19.0, 15.0]),
        SpriteRow::new(1, [-19.0, -1.0]),
        SpriteRow::new(2, [4.0, 12.0]),
        SpriteRow::new(3, [4.0, 12.0]),
        SpriteRow::new(4, [6.0, 12.0]),
    ];
    rows[4].tint[0] = 2.0;
    (art, saturated, sheet, rows)
}

#[test]
fn completes_the_srgb_round_trip_and_keeps_the_linear_checkpoint() {
    let (art, saturated, sheet, rows) = colour_card();
    let extent = EXTENT;
    for recipe in fixture::RECIPES {
        let Some(thrum_test_support::Opened {
            device,
            queue,
            capabilities: caps,
            ..
        }) = fixture::open("sprite colour", recipe)
        else {
            return;
        };
        let mut host = fixture::host(
            &caps,
            &queue,
            extent,
            fixture::Scene::new(rows.clone(), 1),
            sheet.clone(),
        );
        let sdr = fixture::capture(&mut host, &device, extent, CaptureSource::Sdr);
        for (i, pixel) in sdr.rgba.as_chunks::<4>().0.iter().enumerate() {
            let (x, y) = (i % 40, i / 40);
            match (x, y) {
                (1..=16, 1..=16) => {
                    let index = ((y - 1) * 16 + x - 1) * 4;
                    assert_eq!(pixel, &art[index..index + 4], "{recipe} grey at {x},{y}");
                }
                (1..=7, 17) => {
                    assert_eq!(*pixel, saturated[x - 1], "{recipe} saturated at {x},{y}");
                }
                (24, 4) => {
                    let alpha = f64::from(FRONT[3]) / 255.0;
                    for channel in 0..3 {
                        let linear = decode(FRONT[channel])
                            .mul_add(alpha, decode(BACK[channel]) * (1.0 - alpha));
                        assert!((f64::from(pixel[channel]) - encode(linear).round()).abs() <= 1.0);
                    }
                    assert_eq!(pixel[3], 255);
                }
                (26, 4) => assert_eq!(*pixel, BRIGHT, "{recipe} channel-wise clip"),
                _ => assert_eq!(*pixel, [0, 0, 0, 255], "{recipe} clear at {x},{y}"),
            }
        }
        let checkpoint = fixture::capture(&mut host, &device, extent, CaptureSource::SceneLinear);
        assert!(checkpoint.exposure_gpu_words.is_none());
        let start = (4 * 40 + 26) * 8;
        let channel = |offset: usize| {
            f64::from(
                half::f16::from_le_bytes(
                    checkpoint.rgba[start + offset..start + offset + 2]
                        .try_into()
                        .expect("one half-float channel"),
                )
                .to_f32(),
            )
        };
        assert_eq!(channel(0).to_bits(), 2.0_f64.to_bits());
        assert!((channel(2) - decode(BRIGHT[1])).abs() < 0.000_2);
        assert!((channel(4) - decode(BRIGHT[2])).abs() < 0.000_2);
        assert_eq!(channel(6).to_bits(), 1.0_f64.to_bits());
        println!(
            "{recipe}: 256 exact grey codes, 7 exact saturated colours, one linear blend and 2.0 retained before SDR clip"
        );
    }
}

#[test]
#[cfg(target_os = "macos")]
fn float_mode_preserves_authored_codes_and_clips_at_the_stored_ceiling() {
    let (art, saturated, sheet, rows) = colour_card();
    let mut rows = rows;
    // Two exactly representable scene values also distinguish retained from clipped channels.
    let mut brighter = rows[4];
    brighter.position[0] = 8.0;
    brighter.tint[0] = 4.0;
    rows.push(brighter);
    let policy = fixture::Scene::new(rows, 1);
    for recipe in fixture::RECIPES {
        let Some(thrum_test_support::Opened {
            device,
            queue,
            capabilities,
            ..
        }) = fixture::open("float sprite colour", recipe)
        else {
            return;
        };
        let scene = SpriteScene::new(sheet.clone(), 32);
        let mut sdr = fixture::host_for_format(
            &capabilities,
            &queue,
            EXTENT,
            policy.clone(),
            scene.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let surface =
            fixture::drawable_for_format(&device, EXTENT, wgpu::TextureFormat::Rgba8Unorm);
        fixture::submit(&mut sdr, &surface, EXTENT);
        let sdr = thrum_test_support::readback::texture(
            &device,
            &queue,
            surface.texture(),
            thrum_test_support::readback::Mip::level(0),
            ..,
        );
        let mut host = fixture::host_for_format(
            &capabilities,
            &queue,
            EXTENT,
            policy.clone(),
            scene,
            fixture::FLOAT,
        );
        for (headroom, stored_ceiling) in
            [(1.0, 1.0_f64), (1.7, 1.699_218_75), (2.0, 2.0), (8.0, 8.0)]
        {
            host.set_display_headroom(headroom);
            let image = fixture::capture(&mut host, &device, EXTENT, CaptureSource::Edr);
            assert!(image.exposure_gpu_words.is_none());
            assert_eq!(image.rgba.len(), 40 * 32 * 8);
            for (i, bytes) in image.rgba.as_chunks::<8>().0.iter().enumerate() {
                let (x, y) = (i % 40, i / 40);
                let pixel = half_pixel(*bytes);
                let codes = pixel.map(|value| encode(value).round());
                assert!(pixel.iter().all(|&value| value <= f64::from(headroom)));
                assert_eq!(pixel[3].to_bits(), 1.0_f64.to_bits());
                match (x, y) {
                    (1..=16, 1..=16) => {
                        let index = ((y - 1) * 16 + x - 1) * 4;
                        for channel in 0..3 {
                            assert_eq!(
                                codes[channel].to_bits(),
                                f64::from(art[index + channel]).to_bits()
                            );
                        }
                    }
                    (1..=7, 17) => {
                        assert_eq!(
                            codes.map(f64::to_bits),
                            saturated[x - 1].map(|code| f64::from(code).to_bits())
                        );
                    }
                    (24, 4) => {
                        let alpha = f64::from(FRONT[3]) / 255.0;
                        for channel in 0..3 {
                            let expected = decode(FRONT[channel])
                                .mul_add(alpha, decode(BACK[channel]) * (1.0 - alpha));
                            assert!((codes[channel] - encode(expected).round()).abs() <= 1.0);
                        }
                    }
                    (26 | 28, 4) => {
                        let scene = if x == 26 { 2.0 } else { 4.0 };
                        assert_eq!(pixel[0].to_bits(), stored_ceiling.min(scene).to_bits());
                        for channel in 1..3 {
                            assert_eq!(
                                codes[channel].to_bits(),
                                f64::from(BRIGHT[channel]).to_bits()
                            );
                        }
                    }
                    _ => assert_eq!(
                        pixel[..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                        [0; 3]
                    ),
                }
                if stored_ceiling.to_bits() == 1.0_f64.to_bits() {
                    for channel in 0..4 {
                        assert!(
                            (codes[channel] - f64::from(sdr[i * 4 + channel])).abs() <= 1.0,
                            "{recipe} {x},{y} channel {channel}"
                        );
                    }
                }
            }
            println!(
                "{recipe} H={headroom}: float drawable verified at every pixel, ceiling={stored_ceiling}"
            );
        }
    }
}

#[cfg(target_os = "macos")]
fn half_pixel(bytes: [u8; 8]) -> [f64; 4] {
    std::array::from_fn(|channel| {
        let start = channel * 2;
        f64::from(half::f16::from_le_bytes([bytes[start], bytes[start + 1]]).to_f32())
    })
}
