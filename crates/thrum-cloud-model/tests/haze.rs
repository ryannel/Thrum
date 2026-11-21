//! Clear-air range is an ordinary ramped weather control.
use crate::support;
use thrum_cloud_model::{GroundState, aerosol_extinction, visual_range};

#[test]
fn koschmieder_inverts_and_range_moves_on_ticks() {
    for i in 0..=1000 {
        let range = 3000.0 * 100.0_f64.powf(f64::from(i) / 1000.0);
        for gas in [0.0, 1e-6, 8e-6] {
            let aerosol = aerosol_extinction(range, gas).expect("range");
            support::close(
                visual_range(aerosol, gas).expect("inverse"),
                range,
                range * 4e-16,
            );
        }
    }
    assert!(aerosol_extinction(300_000.0, 0.1).is_err());
    assert!(visual_range(-1.0, 0.0).is_err());
    let mut s = support::state(0.0, 0.0);
    s.set_ground(GroundState::new(vec![], 0.0, 3000.0).expect("haze"));
    s.ground_mut()
        .set_targets(&[], 0.0, 300_000.0)
        .expect("target");
    assert_eq!(s.ground().visual_range(), 3000.0);
    s.step(1, 5.0).expect("half response");
    assert_eq!(s.ground().visual_range(), 151_500.0);
    s.step(2, 5.0).expect("response");
    assert_eq!(s.ground().visual_range(), 300_000.0);
}
