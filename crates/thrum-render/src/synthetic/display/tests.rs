use super::*;
use crate::synthetic::{PRE_EXPOSURE, SyntheticView};

fn reference(stored: [f64; 3], inverse: f32) -> [f64; 3] {
    let colour = stored.map(|value| value / (f64::from(PRE_EXPOSURE) * f64::from(inverse)));
    let peak = colour.into_iter().fold(0.0_f64, f64::max);
    if peak <= 0.76 {
        return colour;
    }
    let compressed = 1.0 - 0.24 * 0.24 / (peak - 0.52);
    let retained = 1.0 / (1.0 + 0.15 * (peak - compressed));
    colour.map(|value| compressed * (1.0 - retained) + value / peak * compressed * retained)
}

fn display_byte(linear: f64) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    // The reference domain is nonnegative finite display colour in [0,1].
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the bounded reference display sample is rounded to one byte"
    )]
    let byte = (encoded * 255.0).round() as u8;
    byte
}

#[test]
fn tone_choice_changes_only_reserved_policy_word_for_fixed_and_incident_views() {
    for view in [
        SyntheticView::default().with_exposure(0.123_456).unwrap(),
        SyntheticView::default()
            .with_incident_exposure(0.0, 0.0, 18.0)
            .unwrap(),
    ] {
        let selected = view.with_sdr_tone_map(SdrToneMap::NeutralHighlights);
        let old = view.exposure_policy(1.0, true);
        let new = selected.exposure_policy(1.0, true);
        assert_eq!(
            &old[..28],
            &new[..28],
            "meter/physical settings remain intact"
        );
        assert_eq!(&old[28..32], &0_u32.to_le_bytes());
        assert_eq!(&new[28..32], &1_u32.to_le_bytes());
        assert_eq!(view.pack_view(&view, 1.0), selected.pack_view(&view, 1.0));
        let reset = selected.with_history_reset(true);
        assert_eq!(reset.exposure_policy(1.0, true), new);
        assert_eq!(
            &reset.pack_aux(128, 64)[112..128],
            &selected.pack_aux(256, 128)[112..128]
        );
    }
}

#[test]
fn overlay_uses_only_its_selector_bits_and_off_keeps_every_policy_byte() {
    for view in [
        SyntheticView::default().with_exposure(0.123_456).unwrap(),
        SyntheticView::default()
            .with_incident_exposure(0.0, 0.0, 18.0)
            .unwrap(),
    ] {
        for tone_map in [SdrToneMap::Reinhard, SdrToneMap::NeutralHighlights] {
            let toned = view.with_sdr_tone_map(tone_map);
            let off = toned.with_display_overlay(DisplayOverlay::Off);
            let zones = toned.with_display_overlay(DisplayOverlay::ExposureZones);
            assert_eq!(
                off.exposure_policy(1.0, true),
                toned.exposure_policy(1.0, true)
            );
            let policy = zones.exposure_policy(1.0, true);
            assert_eq!(&policy[..28], &toned.exposure_policy(1.0, true)[..28]);
            assert_eq!(
                &policy[28..32],
                &(tone_map as u32 | 1 << 8).to_le_bytes(),
                "tone map in bits 0-7, overlay in bits 8-15"
            );
            assert_eq!(zones.pack_view(&view, 1.0), toned.pack_view(&view, 1.0));
        }
    }
}

#[test]
fn tone_reference_has_identity_shadows_continuous_knee_and_bounded_highlights() {
    let inverse = 1.0 / PRE_EXPOSURE;
    assert_eq!(reference([0.0; 3], inverse), [0.0; 3]);
    let dark = [0.001, 0.0001, 0.0003];
    assert_eq!(reference(dark, inverse), dark, "no shadow/Fresnel offset");
    let low = reference([0.76 - 1e-7; 3], inverse);
    let high = reference([0.76 + 1e-7; 3], inverse);
    assert!((high[0] - low[0] - 2e-7).abs() < 1e-12);
    assert_eq!(high[0], high[1]);
    assert_eq!(high[1], high[2]);
    assert!(display_byte(high[0]) < 255);
    for peak in [0.0, 0.2, 0.76, 1.0, 10.0, 1e20, 1e42] {
        for colour in [
            [peak, 0.0, 0.0],
            [peak, peak, 0.0],
            [0.0, peak, peak],
            [peak; 3],
        ] {
            assert!(
                reference(colour, inverse)
                    .into_iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod gpu;

#[cfg(target_os = "macos")]
mod input;
