//! One-finger virtual stick feeding the game's held movement axes.
//!
//! The shell sends the returned axes to the world lane's control mailbox; the
//! world step admits each change through the simulation's typed input frame.
//! Cancellation, suspend and surface replacement end the gesture under a new
//! input epoch, so moves or a release from the old touch change nothing.

/// Touch travel, in points, that reaches full deflection.
const FULL_DEFLECTION_POINTS: f64 = 80.0;
/// Deflection below this fraction of full travel is ignored.
const DEAD_ZONE: f64 = 0.1;
/// Axes are sent in steps of one eighth, so finger jitter does not wake the world.
const STEPS: f64 = 8.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct GestureCounters {
    pub(crate) starts: u64,
    pub(crate) releases: u64,
    pub(crate) cancels: u64,
    pub(crate) clears: u64,
    pub(crate) stale: u64,
    pub(crate) ignored_fingers: u64,
    pub(crate) sends: u64,
}

struct Active {
    id: u64,
    origin: [f64; 2],
}

pub(crate) struct Gesture {
    epoch: u64,
    active: Option<Active>,
    /// The touch an epoch change ended; its later events are stale.
    revoked: Option<u64>,
    sent: [f32; 3],
    counters: GestureCounters,
}

impl Gesture {
    pub(crate) fn new() -> Self {
        Self {
            epoch: 1,
            active: None,
            revoked: None,
            sent: [0.0; 3],
            counters: GestureCounters::default(),
        }
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }

    pub(crate) fn counters(&self) -> GestureCounters {
        self.counters
    }

    /// Apply one touch event in physical pixels with the view's scale factor.
    /// Returns new axes when the held control changes.
    pub(crate) fn touch(
        &mut self,
        id: u64,
        phase: Phase,
        location: [f64; 2],
        scale: f64,
    ) -> Option<[f32; 3]> {
        if phase == Phase::Started {
            if self.revoked == Some(id) {
                self.revoked = None;
            }
            if self.active.is_some() {
                self.counters.ignored_fingers += 1;
                return None;
            }
            self.counters.starts += 1;
            self.active = Some(Active {
                id,
                origin: location,
            });
            return self.send([0.0; 3]);
        }
        if self.revoked == Some(id) {
            self.counters.stale += 1;
            return None;
        }
        let Some(active) = self.active.as_ref() else {
            self.counters.stale += 1;
            return None;
        };
        if active.id != id {
            self.counters.ignored_fingers += 1;
            return None;
        }
        match phase {
            Phase::Started => unreachable!("handled above"),
            Phase::Moved => {
                let full = FULL_DEFLECTION_POINTS * scale.max(f64::MIN_POSITIVE);
                let axes = [
                    stick(location[0] - active.origin[0], full),
                    // Screen down is positive; forward is up the screen.
                    stick(active.origin[1] - location[1], full),
                    0.0,
                ];
                self.send(axes)
            }
            Phase::Ended | Phase::Cancelled => {
                if phase == Phase::Ended {
                    self.counters.releases += 1;
                } else {
                    self.counters.cancels += 1;
                }
                self.active = None;
                self.send([0.0; 3])
            }
        }
    }

    /// End any gesture under a new epoch (cancel, suspend, replacement).
    pub(crate) fn clear(&mut self) -> Option<[f32; 3]> {
        self.epoch += 1;
        if let Some(active) = self.active.take() {
            self.revoked = Some(active.id);
            self.counters.clears += 1;
        }
        self.send([0.0; 3])
    }

    fn send(&mut self, axes: [f32; 3]) -> Option<[f32; 3]> {
        if axes
            .iter()
            .zip(self.sent)
            .all(|(next, sent)| next.to_bits() == sent.to_bits())
        {
            return None;
        }
        self.sent = axes;
        self.counters.sends += 1;
        Some(axes)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is clamped to one of seventeen steps in [-1, 1]"
)]
fn stick(offset: f64, full: f64) -> f32 {
    let value = (offset / full).clamp(-1.0, 1.0);
    if value.abs() < DEAD_ZONE || !value.is_finite() {
        return 0.0;
    }
    ((value * STEPS).round() / STEPS) as f32
}

#[cfg(test)]
mod tests {
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
}
