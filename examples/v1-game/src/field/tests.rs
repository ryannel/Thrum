use super::{ENTITY_COUNT, PATH_TICKS, WALL_COUNT, camera, entity};

#[test]
fn layout_is_deterministic_and_spans_far_beyond_the_view() {
    assert_eq!(ENTITY_COUNT, 100_513);
    let mut extent = [0.0_f64; 2];
    for index in (0..ENTITY_COUNT).step_by(97) {
        let first = entity(index);
        assert_eq!(first, entity(index), "slot {index} is not deterministic");
        assert!(first.half_extent.iter().all(|half| *half > 0.0));
        extent[0] = extent[0].max(first.position[0].abs());
        extent[1] = extent[1].max(first.position[2].abs());
    }
    assert!(extent.iter().all(|half| *half > 350.0), "{extent:?}");
}

#[test]
fn walls_stand_halfway_between_static_prop_lines() {
    // Prop lattice lines: x = (c - 157.5) * 2.5 and z = (r - 158) * 2.5.
    for wall in 1..=WALL_COUNT {
        let wall = entity(wall);
        let (thin, line_phase) = if wall.yaw == 0.0 {
            (wall.position[2], 0.0)
        } else {
            (wall.position[0], 0.5)
        };
        let between = (thin / 2.5 - line_phase).rem_euclid(1.0);
        assert!((between - 0.5).abs() < 1e-4, "{wall:?}");
    }
}

#[test]
fn flythrough_turns_and_cuts_are_flagged() {
    let mut cuts = 0;
    let mut previous = camera(1, 0.0);
    for tick in 2..=PATH_TICKS {
        let sample = camera(tick, 0.0);
        let turned = (sample.heading - previous.heading).abs() > 3.0;
        let jumped = (0..3).any(|axis| (sample.eye[axis] - previous.eye[axis]).abs() > 5.0);
        let cut = sample.history_epoch != previous.history_epoch;
        assert_eq!(cut, turned || jumped, "continuity at tick {tick}");
        cuts += usize::from(cut);
        assert!(
            sample.eye[1] >= 0.9,
            "camera below the ground at tick {tick}"
        );
        previous = sample;
    }
    // Two snaps per 600-tick block, plus the quarter-loop cut.
    assert_eq!(cuts, 13);
    assert_ne!(
        camera(300, 1.0).history_epoch,
        camera(302, 1.0).history_epoch
    );
    assert_eq!(
        camera(301, 0.0).history_epoch,
        camera(302, 1.0).history_epoch
    );
}
