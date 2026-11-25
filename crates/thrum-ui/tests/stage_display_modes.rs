//! Submitted half-floats preserve the scene's highlight under the interface.

#[expect(
    dead_code,
    reason = "individual proof binaries select different fixture helpers"
)]
mod support;

use support::{FLOAT, RECIPES, SDR};
use thrum_ui::{PaintListBuilder, SpriteRow};

#[test]
fn interface_white_is_sdr_white_at_every_admitted_headroom() {
    let extent = [12, 8];
    for recipe in RECIPES {
        let Some(gpu) = support::device("interface EDR white and blend", recipe) else {
            return;
        };
        for (format, background, headroom) in [
            (SDR, 0.0, 1.0),
            (FLOAT, 0.0, 1.0),
            (FLOAT, 3.0, 2.0),
            (FLOAT, 3.0, 4.0),
        ] {
            let mut host = support::host(
                &gpu.capabilities,
                &gpu.queue,
                extent,
                format,
                Some(support::interface(3)),
                background,
            )
            .expect("prepare the display mode with its interface");
            host.set_display_headroom(headroom);
            let drawable = support::drawable(&gpu.device, extent, format);
            let plain = support::capture(&mut host, &drawable, extent);
            let mut list = PaintListBuilder::new(extent);
            list.push(SpriteRow::new(1, [0.0; 2]));
            let mut scrim = SpriteRow::new(1, [1.0, 0.0]);
            scrim.tint = [0.0, 0.0, 0.0, 0.5];
            list.push(scrim);
            let mut half_white = SpriteRow::new(1, [2.0, 0.0]);
            half_white.tint[3] = 0.5;
            list.push(half_white);
            host.source_mut().paint = Some(list.finish());
            let painted = support::capture(&mut host, &drawable, extent);
            if format == SDR {
                assert_eq!(&painted.rgba[..4], &[255; 4]);
                assert_eq!(painted.rgba[4], 0);
                assert!((i16::from(painted.rgba[8]) - 188).abs() <= 1);
                continue;
            }
            for channel in painted.rgba[..8].as_chunks::<2>().0 {
                assert_eq!(channel, &half::f16::ONE.to_le_bytes());
            }
            for channel in 0..3 {
                let before = support::half(&plain.rgba[8 + 2 * channel..]);
                if headroom > 1.0 {
                    assert!(before > 1.0, "fixture must present a real highlight");
                }
                let after = support::half(&painted.rgba[8 + 2 * channel..]);
                let expected = support::encode(before * 0.5) * 255.0;
                assert!((support::encode(after) * 255.0 - expected).abs() <= 1.0);
                if headroom.to_bits() == 1.0_f32.to_bits() {
                    let half_white = support::half(&painted.rgba[16 + 2 * channel..]);
                    assert!((support::encode(half_white) * 255.0 - 188.0).abs() <= 1.0);
                }
            }
        }
    }
}
