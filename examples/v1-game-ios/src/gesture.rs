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
    #[cfg(all(target_os = "ios", feature = "proving"))]
    suspended: Option<Active>,
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
            #[cfg(all(target_os = "ios", feature = "proving"))]
            suspended: None,
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
        #[cfg(all(target_os = "ios", feature = "proving"))]
        {
            self.suspended = None;
        }
        self.epoch += 1;
        if let Some(active) = self.active.take() {
            self.revoked = Some(active.id);
            self.counters.clears += 1;
        }
        self.send([0.0; 3])
    }

    /// Release the axes while retaining the stick origin for a still-held contact.
    #[cfg(all(target_os = "ios", feature = "proving"))]
    pub(crate) fn interrupt(&mut self) -> Option<[f32; 3]> {
        self.suspended = self.active.take();
        self.send([0.0; 3])
    }

    /// Take the contact the router returned, with its current touch position.
    #[cfg(all(target_os = "ios", feature = "proving"))]
    pub(crate) fn resume(
        &mut self,
        touches: impl Iterator<Item = (u64, [f64; 2])>,
        scale: f64,
    ) -> Option<[f32; 3]> {
        let suspended = self.suspended.take();
        let mut fallback = None;
        for (id, point) in touches {
            if suspended.as_ref().is_some_and(|active| active.id == id) {
                self.active = suspended;
                return self.touch(id, Phase::Moved, point, scale);
            }
            fallback.get_or_insert((id, point));
        }
        fallback.and_then(|(id, point)| self.touch(id, Phase::Started, point, scale))
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
mod tests;
