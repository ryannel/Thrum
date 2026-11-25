//! Whole-image CPU oracle for the two submitted display encodings.

#[expect(
    dead_code,
    reason = "individual proof binaries select different fixture helpers"
)]
mod support;

use support::{FLOAT, MARK, RECIPES, SDR};
use thrum_render::runtime::SubmittedImage;

#[test]
fn sprite_scene_matches_authored_paint_and_preserves_every_uncovered_pixel() {
    for recipe in RECIPES {
        let Some(gpu) = support::device("interface exact display", recipe) else {
            return;
        };
        for format in [SDR, FLOAT] {
            for (extent, scale) in [([18, 12], 1), ([19, 13], 3)] {
                let mut control =
                    support::host(&gpu.capabilities, &gpu.queue, extent, format, None, 3.0)
                        .expect("prepare the no-interface control");
                let mut host = support::host(
                    &gpu.capabilities,
                    &gpu.queue,
                    extent,
                    format,
                    Some(support::interface(3)),
                    3.0,
                )
                .expect("prepare the interface");
                control.set_display_headroom(4.0);
                host.set_display_headroom(4.0);
                host.source_mut().paint = Some(support::paint(extent, scale));
                let drawable = support::drawable(&gpu.device, extent, format);
                let plain = support::capture(&mut control, &drawable, extent);
                let painted = support::capture(&mut host, &drawable, extent);
                compare(&plain, &painted, scale, format == FLOAT);
            }
        }
    }
}

fn compare(plain: &SubmittedImage, painted: &SubmittedImage, scale: u8, float: bool) {
    let bytes = if float { 8 } else { 4 };
    let [width, height] = plain.extent;
    let width = width as usize;
    let scale = usize::from(scale);
    let mut highlight = false;
    assert_eq!(plain.rgba.len(), width * height as usize * bytes);
    assert_eq!(painted.rgba.len(), plain.rgba.len());
    for (index, (background, actual)) in plain
        .rgba
        .chunks_exact(bytes)
        .zip(painted.rgba.chunks_exact(bytes))
        .enumerate()
    {
        let x = index % width;
        let y = index / width;
        let mark = ((2..2 + 3 * scale).contains(&x) && (3..3 + 2 * scale).contains(&y))
            .then(|| MARK[(y - 3) / scale * 3 + (x - 2) / scale]);
        let scrim = (4..9).contains(&x) && y < 5;
        let white = (x, y) == (1, 1);
        if !scrim && !white && mark.is_none() {
            assert_eq!(actual, background, "uncovered pixel at {x},{y}");
            highlight |= float && support::half(background) > 1.0;
            continue;
        }
        for channel in 0..4 {
            let encode = |linear| {
                if channel == 3 {
                    linear * 255.0
                } else {
                    support::encode(linear) * 255.0
                }
            };
            let decode = |stored| {
                if channel == 3 {
                    stored
                } else {
                    support::decode(stored)
                }
            };
            let read = |pixel: &[u8]| {
                if float {
                    support::half(&pixel[channel * 2..])
                } else {
                    decode(f64::from(pixel[channel]) / 255.0)
                }
            };
            let authored = if white { Some([255; 4]) } else { mark };
            if scrim {
                let tint = [0.15, 0.0, 0.55, 1.0][channel];
                let destination = authored.map_or_else(
                    || read(background),
                    |texels| decode(f64::from(texels[channel]) / 255.0),
                );
                let reference = encode(0.5 * tint + 0.5 * destination);
                assert!(
                    (encode(read(actual)) - reference).abs() <= 1.0,
                    "blend at {x},{y} channel {channel}: actual {} reference {reference}",
                    encode(read(actual))
                );
            } else {
                let code = authored.expect("covered by an opaque row")[channel];
                assert_eq!(
                    encode(read(actual)).round(),
                    f64::from(code),
                    "opaque {x},{y} channel {channel}"
                );
            }
        }
    }
    assert!(
        !float || highlight,
        "the uncovered float scene must include values above white"
    );
}
