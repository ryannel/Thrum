use super::{Gesture, Phase};

const SCALE: f64 = 3.0;

#[test]
fn a_drag_deflects_and_a_release_returns_to_rest() {
    let mut gesture = Gesture::new();
    assert_eq!(
        gesture.touch(1, Phase::Started, [300.0, 900.0], SCALE),
        None
    );
    // 40 points right and 80 points up: half right, full forward.
    assert_eq!(
        gesture.touch(1, Phase::Moved, [420.0, 660.0], SCALE),
        Some([0.5, 1.0, 0.0])
    );
    assert_eq!(
        gesture.touch(1, Phase::Moved, [421.0, 661.0], SCALE),
        None,
        "jitter"
    );
    assert_eq!(
        gesture.touch(1, Phase::Moved, [9_000.0, 900.0], SCALE),
        Some([1.0, 0.0, 0.0]),
        "clamped and inside the dead zone"
    );
    assert_eq!(
        gesture.touch(1, Phase::Ended, [9_000.0, 900.0], SCALE),
        Some([0.0; 3])
    );
    assert_eq!(gesture.counters().releases, 1);
}

#[test]
fn a_cancelled_touch_clears_the_held_axes() {
    let mut gesture = Gesture::new();
    gesture.touch(1, Phase::Started, [0.0, 0.0], SCALE);
    gesture.touch(1, Phase::Moved, [-240.0, 0.0], SCALE);
    assert_eq!(
        gesture.touch(1, Phase::Cancelled, [-240.0, 0.0], SCALE),
        Some([0.0; 3])
    );
    assert_eq!(gesture.counters().cancels, 1);
}

#[test]
fn a_stale_release_after_suspend_cannot_change_the_resumed_game() {
    let mut gesture = Gesture::new();
    gesture.touch(1, Phase::Started, [0.0, 0.0], SCALE);
    gesture.touch(1, Phase::Moved, [240.0, 0.0], SCALE);
    let before = gesture.epoch();
    assert_eq!(gesture.clear(), Some([0.0; 3]), "suspend clears held input");
    assert!(gesture.epoch() > before);
    // The resumed player holds a new touch forward.
    gesture.touch(2, Phase::Started, [0.0, 0.0], SCALE);
    assert_eq!(
        gesture.touch(2, Phase::Moved, [0.0, -240.0], SCALE),
        Some([0.0, 1.0, 0.0])
    );
    // The old touch's move and release arrive late.
    assert_eq!(gesture.touch(1, Phase::Moved, [240.0, 0.0], SCALE), None);
    assert_eq!(gesture.touch(1, Phase::Ended, [240.0, 0.0], SCALE), None);
    assert_eq!(gesture.counters().stale, 2);
    // The new touch still steers, and a later background ends it too.
    assert_eq!(
        gesture.touch(2, Phase::Moved, [0.0, 0.0], SCALE),
        Some([0.0; 3])
    );
    gesture.touch(2, Phase::Moved, [0.0, -240.0], SCALE);
    gesture.clear();
    assert_eq!(gesture.touch(2, Phase::Ended, [0.0, 0.0], SCALE), None);
    assert_eq!(gesture.counters().stale, 3);
    assert_eq!(gesture.counters().clears, 2);
}

#[test]
fn a_second_finger_does_not_steal_the_stick() {
    let mut gesture = Gesture::new();
    gesture.touch(1, Phase::Started, [0.0, 0.0], SCALE);
    assert_eq!(gesture.touch(2, Phase::Started, [10.0, 10.0], SCALE), None);
    assert_eq!(gesture.touch(2, Phase::Moved, [900.0, 10.0], SCALE), None);
    assert_eq!(
        gesture.touch(1, Phase::Moved, [240.0, 0.0], SCALE),
        Some([1.0, 0.0, 0.0])
    );
}

#[test]
fn clearing_at_rest_sends_nothing() {
    let mut gesture = Gesture::new();
    assert_eq!(gesture.clear(), None);
    assert_eq!(gesture.counters().clears, 0);
}
