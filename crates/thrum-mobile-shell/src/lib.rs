//! Allocation-free lifecycle and touch admission for the initial mobile shells.
//!
//! Platform callbacks reconcile this gate before clock, input, and presentation
//! work. The authoritative world and its clock remain in `thrum-core`.

use std::sync::atomic::{AtomicU64, Ordering};

mod receipts;

pub use receipts::{LocalDelivery, LocalReceiptCursor, LocalReceiptError};

/// The maximum simultaneous touch starts retained by the initial shell.
pub const TOUCH_CAPACITY: usize = 16;

/// One atomic Android Activity state. The low bit is foreground; the remaining
/// bits count pause transitions, so a pause/resume between Rust callbacks remains
/// visible. The Java lifecycle bridge calls these methods in callback order.
#[derive(Debug, Default)]
pub struct AndroidLifecycleSignal {
    packed: AtomicU64,
}

/// A coherent snapshot of the latest Android lifecycle signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AndroidLifecycleSnapshot {
    /// Number of Activity pause callbacks observed by the bridge.
    pub pause_generation: u64,
    /// Whether the latest Activity callback was resume.
    pub foreground: bool,
}

impl AndroidLifecycleSignal {
    /// Construct a signal before registering the Android Activity bridge.
    pub const fn new() -> Self {
        Self {
            packed: AtomicU64::new(0),
        }
    }

    /// Record Activity pause. Returns false only if the generation is exhausted;
    /// the bridge must treat that impossible-in-practice case as terminal.
    pub fn on_pause(&self) -> bool {
        self.packed
            .try_update(Ordering::AcqRel, Ordering::Acquire, |prior| {
                let generation = prior >> 1;
                (generation < u64::MAX >> 1).then_some((generation + 1) << 1)
            })
            .is_ok()
    }

    /// Record Activity resume without concealing an intervening pause.
    pub fn on_resume(&self) {
        self.packed.fetch_or(1, Ordering::Release);
    }

    /// Read the latest state before admitting clock, input or presentation work.
    pub fn snapshot(&self) -> AndroidLifecycleSnapshot {
        let packed = self.packed.load(Ordering::Acquire);
        AndroidLifecycleSnapshot {
            pause_generation: packed >> 1,
            foreground: packed & 1 != 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TouchStart {
    id: u64,
    surface_epoch: u64,
}

/// Result of starting a touch gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchAdmission {
    /// A new touch was tracked and may later produce one action.
    Started,
    /// The shell is inactive or this ID is already tracked.
    Ignored,
    /// The finite touch table is full; expose input overload.
    Capacity,
}

/// The small state shared by the mobile event-loop adapter and fake-surface
/// lifecycle tests. It does not own the world, clock or GPU surface.
#[derive(Debug)]
pub struct MobileGate {
    surface_epoch: u64,
    surface_available: bool,
    foreground: bool,
    focused: bool,
    clock_active: bool,
    pause_pending: bool,
    observed_android_pause: u64,
    touches: [Option<TouchStart>; TOUCH_CAPACITY],
}

impl Default for MobileGate {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileGate {
    /// Construct a gate with no surface or foreground permission.
    pub const fn new() -> Self {
        Self {
            surface_epoch: 0,
            surface_available: false,
            foreground: false,
            focused: true,
            clock_active: false,
            pause_pending: false,
            observed_android_pause: 0,
            touches: [None; TOUCH_CAPACITY],
        }
    }

    /// Current generation used to reject releases from a prior surface.
    pub fn surface_epoch(&self) -> u64 {
        self.surface_epoch
    }

    /// Whether a new or existing host may advance the local world.
    pub fn ready_for_play(&self) -> bool {
        self.surface_available && self.foreground && !self.pause_pending
    }

    /// Whether a normalized touch may enter the local action queue.
    pub fn accepts_input(&self) -> bool {
        self.ready_for_play() && self.focused && self.clock_active
    }

    /// A pause must be applied to the host before draining or resuming it.
    pub fn pause_pending(&self) -> bool {
        self.pause_pending
    }

    /// Reconcile a coherent Android snapshot before any externally driven work.
    /// A compressed pause/resume invalidates gestures and still requests pause.
    pub fn observe_android(&mut self, snapshot: AndroidLifecycleSnapshot) {
        if snapshot.pause_generation != self.observed_android_pause {
            self.observed_android_pause = snapshot.pause_generation;
            self.require_pause();
        }
        if !snapshot.foreground && self.foreground {
            self.require_pause();
        }
        self.foreground = snapshot.foreground;
    }

    /// Record iOS foreground/occlusion state.
    pub fn set_ios_foreground(&mut self, visible: bool) {
        if !visible && self.foreground {
            self.require_pause();
        }
        self.foreground = visible;
    }

    /// Record a newly available platform surface. A redundant callback is inert.
    pub fn surface_ready(&mut self) {
        if !self.surface_available {
            self.surface_available = true;
            if self.surface_epoch == 0 {
                self.surface_epoch = 1;
            }
        }
    }

    /// Lose the surface and invalidate every tracked gesture. Returns false if
    /// its generation cannot advance; the adapter must terminate in that case.
    pub fn surface_lost(&mut self) -> bool {
        if !self.surface_available {
            return true;
        }
        let Some(next) = self.surface_epoch.checked_add(1) else {
            return false;
        };
        self.surface_epoch = next;
        self.surface_available = false;
        self.require_pause();
        true
    }

    /// Focus affects local input. Focus loss cancels existing gestures.
    pub fn set_focus(&mut self, focused: bool) {
        if !focused {
            self.clear_touches();
        }
        self.focused = focused;
    }

    /// Call only after `HeadlessHost::pause` has stopped elapsed-time admission.
    pub fn mark_host_paused(&mut self) {
        self.clock_active = false;
        self.pause_pending = false;
    }

    /// Mark the initial already-active host, or a successfully resumed host.
    /// Returns false while a pause has not been applied or a gate is closed.
    pub fn mark_host_active(&mut self) -> bool {
        if !self.ready_for_play() {
            return false;
        }
        self.clock_active = true;
        true
    }

    /// Track one start from the current surface and foreground.
    pub fn touch_started(&mut self, id: u64) -> TouchAdmission {
        if !self.accepts_input() {
            return TouchAdmission::Ignored;
        }
        if self.touches.iter().flatten().any(|touch| touch.id == id) {
            return TouchAdmission::Ignored;
        }
        let Some(slot) = self.touches.iter_mut().find(|slot| slot.is_none()) else {
            return TouchAdmission::Capacity;
        };
        *slot = Some(TouchStart {
            id,
            surface_epoch: self.surface_epoch,
        });
        TouchAdmission::Started
    }

    /// Produce one action only for a tracked start in the current generation.
    pub fn touch_ended(&mut self, id: u64) -> bool {
        if !self.accepts_input() {
            return false;
        }
        for slot in &mut self.touches {
            if let Some(started) = *slot
                && started.id == id
            {
                *slot = None;
                return started.surface_epoch == self.surface_epoch;
            }
        }
        false
    }

    /// Cancel one touch without creating an action.
    pub fn touch_cancelled(&mut self, id: u64) {
        for slot in &mut self.touches {
            if slot.is_some_and(|touch| touch.id == id) {
                *slot = None;
                break;
            }
        }
    }

    fn require_pause(&mut self) {
        self.pause_pending = true;
        self.clock_active = false;
        self.clear_touches();
    }

    fn clear_touches(&mut self) {
        self.touches.fill(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{hint::black_box, time::Instant};
    use thrum_core::{ClockConfig, ClockState, TickClock};

    fn active_ios() -> MobileGate {
        let mut gate = MobileGate::new();
        gate.surface_ready();
        gate.set_ios_foreground(true);
        assert!(gate.mark_host_active());
        gate
    }

    #[test]
    fn compressed_android_pause_cancels_touch_before_next_input() {
        let signal = AndroidLifecycleSignal::new();
        let mut gate = MobileGate::new();
        gate.surface_ready();
        signal.on_resume();
        gate.observe_android(signal.snapshot());
        assert!(gate.mark_host_active());
        assert_eq!(gate.touch_started(7), TouchAdmission::Started);

        assert!(signal.on_pause());
        signal.on_resume();
        gate.observe_android(signal.snapshot()); // input path, before next clock callback
        assert!(gate.pause_pending());
        assert!(!gate.touch_ended(7));
        gate.mark_host_paused();
        assert!(gate.mark_host_active());
        assert!(!gate.touch_ended(7));
    }

    #[test]
    fn android_pause_generation_exhaustion_is_terminal() {
        let signal = AndroidLifecycleSignal::new();
        signal.packed.store(u64::MAX - 2, Ordering::Release);
        assert!(signal.on_pause());
        assert_eq!(signal.snapshot().pause_generation, u64::MAX >> 1);
        assert!(!signal.on_pause());
        assert_eq!(signal.snapshot().pause_generation, u64::MAX >> 1);
    }

    #[test]
    fn surface_recreation_and_duplicate_events_never_repeat_action() {
        let mut gate = active_ios();
        assert_eq!(gate.touch_started(5), TouchAdmission::Started);
        assert_eq!(gate.touch_started(5), TouchAdmission::Ignored);
        assert!(gate.touch_ended(5));
        assert!(!gate.touch_ended(5));
        assert_eq!(gate.touch_started(5), TouchAdmission::Started);
        assert!(gate.surface_lost());
        let old_epoch = gate.surface_epoch();
        gate.mark_host_paused();
        gate.surface_ready();
        assert!(gate.mark_host_active());
        assert_eq!(gate.surface_epoch(), old_epoch);
        assert!(!gate.touch_ended(5));
        assert_eq!(gate.touch_started(5), TouchAdmission::Started);
        gate.touch_cancelled(5);
        assert!(!gate.touch_ended(5));
    }

    #[test]
    fn foreground_and_focus_gate_touch_without_destroying_surface() {
        let mut gate = active_ios();
        assert_eq!(gate.touch_started(1), TouchAdmission::Started);
        gate.set_focus(false);
        assert!(!gate.touch_ended(1));
        gate.set_focus(true);
        gate.set_ios_foreground(false);
        assert!(gate.pause_pending());
        assert!(!gate.accepts_input());
        gate.set_ios_foreground(true);
        assert!(!gate.mark_host_active());
        gate.mark_host_paused();
        assert!(gate.mark_host_active());
    }

    #[test]
    fn bounded_touches_report_capacity_without_replacement() {
        let mut gate = active_ios();
        for id in 0..TOUCH_CAPACITY as u64 {
            assert_eq!(gate.touch_started(id), TouchAdmission::Started);
        }
        assert_eq!(gate.touch_started(100), TouchAdmission::Capacity);
        assert!(gate.touch_ended(0));
        assert_eq!(gate.touch_started(100), TouchAdmission::Started);
    }

    #[test]
    fn compressed_pause_drains_real_clock_without_hidden_catch_up() {
        use std::time::Duration;

        let signal = AndroidLifecycleSignal::new();
        let mut gate = MobileGate::new();
        gate.surface_ready();
        signal.on_resume();
        gate.observe_android(signal.snapshot());
        assert!(gate.mark_host_active());
        let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
        let mut ticks = 0;
        let first = clock
            .advance(Duration::from_millis(100), 1, || {
                ticks += 1;
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!((first.completed, first.queued), (1, 5));

        assert!(signal.on_pause());
        signal.on_resume();
        gate.observe_android(signal.snapshot());
        assert!(gate.pause_pending());
        clock.pause();
        gate.mark_host_paused();
        let drained = clock
            .advance(Duration::from_secs(600), 4, || {
                ticks += 1;
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(
            (drained.admitted, drained.completed, drained.queued),
            (0, 4, 1)
        );
        let final_drain = clock
            .advance(Duration::from_secs(610), 4, || {
                ticks += 1;
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!((final_drain.admitted, final_drain.completed), (0, 1));
        clock.resume(Duration::from_secs(610)).unwrap();
        assert!(gate.mark_host_active());
        let resumed = clock
            .advance(
                Duration::from_secs(610) + Duration::from_nanos(16_666_667),
                4,
                || {
                    ticks += 1;
                    Ok::<(), ()>(())
                },
            )
            .unwrap();
        assert_eq!(ticks, 7);
        assert_eq!((resumed.admitted, resumed.completed), (1, 1));
        assert_eq!(resumed.state, ClockState::Active);
    }

    #[test]
    #[ignore = "local release-mode cost probe"]
    fn bench_android_reconcile_touch_pair() {
        const ITERATIONS: u64 = 1_000_000;
        let signal = AndroidLifecycleSignal::new();
        signal.on_resume();
        let mut gate = MobileGate::new();
        gate.surface_ready();
        gate.observe_android(signal.snapshot());
        assert!(gate.mark_host_active());
        for run in 0..5 {
            let started = Instant::now();
            for id in 0..ITERATIONS {
                gate.observe_android(black_box(signal.snapshot()));
                black_box(gate.touch_started(black_box(id)));
                black_box(gate.touch_ended(black_box(id)));
            }
            let elapsed = started.elapsed();
            println!(
                "mobile gate run {run}: {:.2} ns/reconcile+start+end, gate={} B signal={} B",
                elapsed.as_nanos() as f64 / ITERATIONS as f64,
                size_of::<MobileGate>(),
                size_of::<AndroidLifecycleSignal>()
            );
        }
    }
}
