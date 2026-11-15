//! Stage connectivity and clamping.
#[expect(
    dead_code,
    reason = "shared basin and wave fixtures are used by other tests"
)]
#[path = "support/fixtures.rs"]
mod support;
use support::{body, close, grid, profile};
use thrum_water_model::{BodyKind, Map, Stage};

#[test]
fn moving_stage_floods_only_after_sill_and_drains_reversibly() {
    // Two wells separated by the x=10 sill at h=3; outer wall h=10.
    let bed = grid([21, 11], |x, z| {
        if x < 0.5 || x > 19.5 || z < 0.5 || z > 9.5 {
            10.0
        } else if (x - 10.0).abs() < 0.5 {
            3.0
        } else {
            ((x - 5.0).abs().min((x - 15.0).abs()) * 0.5).min(2.5)
        }
    });
    let mut b = body("fill", BodyKind::Basin { seed: [5.0, 5.0] }, 1.0);
    b.stage = Stage {
        min: -0.5,
        max: 3.0,
        value: 0.0,
    };
    let mut map = Map::prepare(bed, vec![b], profile()).unwrap();
    let original = map.sample([5.1, 5.0]);
    assert!(map.sample([15.0, 5.0]).is_none());
    for s in [0.2, 0.9, 1.5, 1.999] {
        map.set_stage(0, s).unwrap();
        assert!(map.sample([15.0, 5.0]).is_none());
        close(map.sample([5.0, 5.0]).unwrap().row.level, 1.0 + s);
    }
    map.set_stage(0, 2.001).unwrap();
    assert!(map.sample([15.0, 5.0]).unwrap().depth > 3.0);
    map.set_stage(0, 0.0).unwrap();
    assert_eq!(map.sample([5.1, 5.0]), original);
    assert!(map.sample([15.0, 5.0]).is_none());
    map.set_stage(0, 99.0).unwrap();
    close(map.sample([5.0, 5.0]).unwrap().row.level, 4.0);
    map.set_stage(0, -99.0).unwrap();
    close(map.sample([5.0, 5.0]).unwrap().row.level, 0.5);
    assert_eq!(map.clamped_stages, 2);
    assert!(map.set_stage(0, f64::NAN).is_err());
    assert_eq!(map.clamped_stages, 2);
}

#[test]
fn preparation_checks_the_whole_stage_range() {
    let mut b = body("eventual-spill", BodyKind::Basin { seed: [5.0, 5.0] }, 1.0);
    b.stage = Stage {
        min: 0.0,
        max: 20.0,
        value: 0.0,
    };
    let e = Map::prepare(
        grid([11, 11], |x, z| (x - 5.0).hypot(z - 5.0)),
        vec![b],
        profile(),
    )
    .err()
    .unwrap();
    assert_eq!(e.name, "eventual-spill");
    assert!(e.reason.contains("edge"));
}
