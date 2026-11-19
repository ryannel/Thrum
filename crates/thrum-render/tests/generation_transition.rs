//! Frame-boundary surface transition and restoration proofs.

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod generation_transition {
    mod metal;
}
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod support;

use thrum_render::runtime::{SurfaceSwitch, SurfaceSwitchOutcome};

const OLD: [u32; 2] = [64, 64];
const NEW: [u32; 2] = [96, 64];

#[test]
fn configure_deferral_and_restore_preserve_one_acquisition_extent() {
    let mut surface = SurfaceSwitch::configured();
    let mut descriptor = OLD;
    let mut drawable = OLD;
    for (held, busy) in [(true, false), (false, true)] {
        assert_eq!(
            surface.take(held, busy, &NEW, &OLD, |_| -> Result<(), &str> {
                panic!("a deferred boundary must not configure")
            }),
            SurfaceSwitchOutcome::Deferred
        );
        assert!(surface.usable());
    }
    let counts = |surface: &SurfaceSwitch| {
        let record = surface.record();
        (
            record.deferrals,
            record.configure_calls,
            record.generations_taken,
            record.restore_calls,
        )
    };
    assert_eq!(counts(&surface), (2, 0, 0, 0));
    assert_eq!(
        surface.take(false, false, &NEW, &OLD, |asked| {
            descriptor = *asked;
            if *asked == NEW {
                assert_ne!(descriptor, drawable, "skipping restore is not coherent");
                return Err("invalid new configuration");
            }
            drawable = *asked;
            Ok(())
        }),
        SurfaceSwitchOutcome::Refused
    );
    assert_eq!(descriptor, drawable);
    assert!(surface.usable());
    assert_eq!(counts(&surface), (2, 1, 0, 1));
    assert_eq!(surface.record().restored, Some(true));

    surface.begin();
    assert_eq!(
        surface.take(false, false, &NEW, &OLD, |asked| {
            descriptor = *asked;
            Err("refused")
        }),
        SurfaceSwitchOutcome::Refused
    );
    assert!(!surface.usable());
    assert_eq!(counts(&surface), (0, 1, 0, 1));
    assert_eq!(surface.record().restored, Some(false));
    surface.begin();
    assert_eq!(
        surface.take(false, false, &NEW, &OLD, |asked| {
            descriptor = *asked;
            drawable = *asked;
            Ok::<_, &str>(())
        }),
        SurfaceSwitchOutcome::Taken
    );
    assert!(surface.usable());
    assert_eq!(descriptor, drawable);
    assert_eq!(counts(&surface), (0, 1, 1, 0));
}
