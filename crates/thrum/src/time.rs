use std::ops::{Add, Div, Mul, Sub};

use crate::{Data, Num};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// A duration on the game's deterministic clock.
/// ```
/// use thrum::seconds;
/// assert_eq!(seconds(0.5) + seconds(0.5), seconds(1));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Seconds(u64);

/// Express a duration in seconds.
/// ```
/// use thrum::{Seconds, seconds};
/// assert_eq!(seconds(0), Seconds::ZERO);
/// ```
pub fn seconds(value: impl Into<Seconds>) -> Seconds {
    value.into()
}

impl Seconds {
    /// A duration of zero.
    /// ```
    /// use thrum::{Seconds, seconds};
    /// assert_eq!(Seconds::ZERO, seconds(0));
    /// ```
    pub const ZERO: Self = Self(0);

    /// Express this duration as a portable number.
    /// ```
    /// use thrum::{Num, seconds};
    /// assert_eq!(seconds(2).num(), Num::from(2));
    /// ```
    pub fn num(self) -> Num {
        Num::ratio(self.0, NANOS_PER_SECOND)
    }

    pub(crate) fn ticks(self, rate: u32) -> u64 {
        let rate = u128::from(rate);
        let ticks = (u128::from(self.0) * rate)
            .saturating_sub(rate / 2)
            .div_ceil(u128::from(NANOS_PER_SECOND));
        u64::try_from(ticks).unwrap_or(u64::MAX)
    }

    pub(crate) fn from_ticks(ticks: u64, rate: u32) -> Self {
        let rate = u128::from(rate);
        let nanos = (u128::from(ticks) * u128::from(NANOS_PER_SECOND) + rate / 2) / rate;
        Self(u64::try_from(nanos).unwrap_or(u64::MAX))
    }
}

impl From<i32> for Seconds {
    fn from(value: i32) -> Self {
        Self(u64::try_from(value).unwrap_or(0) * NANOS_PER_SECOND)
    }
}

impl From<f64> for Seconds {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Duration literals round to a nanosecond and saturate at zero and the maximum"
    )]
    fn from(value: f64) -> Self {
        Self((value * NANOS_PER_SECOND as f64).round() as u64)
    }
}

impl Add for Seconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Seconds {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Mul<Num> for Seconds {
    type Output = Self;

    fn mul(self, rhs: Num) -> Self {
        seconds(f64::from((self.num() * rhs).float()))
    }
}

impl Div for Seconds {
    type Output = Num;

    fn div(self, rhs: Self) -> Num {
        Num::ratio(self.0, rhs.0)
    }
}

impl Data for Seconds {
    const SCHEMA: &'static str = "thrum::Seconds(nanoseconds)";

    fn same_encoding(&self, other: &Self) -> bool {
        self == other
    }
    fn write(&self, writer: &mut crate::__derive::DataWriter<'_, '_>) {
        self.0.write(writer);
    }
    fn read(
        reader: &mut crate::__derive::DataReader<'_, '_>,
    ) -> Result<Self, crate::__derive::InvalidData> {
        u64::read(reader).map(Self)
    }
}

/// A number on the game's clock.
/// ```
/// use thrum::TickNumber;
/// assert_eq!(TickNumber::default(), TickNumber::default());
/// ```
#[derive(Data, Copy, Default, Eq, PartialOrd, Ord)]
pub struct TickNumber(pub(crate) u64);

/// The game's time handed to a rule.
/// ```
/// use thrum::Tick;
/// fn pace(tick: Tick) -> thrum::Num { tick.dt() }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Tick {
    pub(crate) number: TickNumber,
    pub(crate) rate: u32,
    pub(crate) scene_entered: TickNumber,
    pub(crate) phase_time: Seconds,
}

impl Tick {
    /// The game's current tick number.
    /// ```
    /// use thrum::{Tick, TickNumber};
    /// fn now(tick: Tick) -> TickNumber { tick.number() }
    /// ```
    pub fn number(self) -> TickNumber {
        self.number
    }

    /// The duration of one tick.
    /// ```
    /// use thrum::{Seconds, Tick};
    /// fn step_size(tick: Tick) -> thrum::Num { tick.dt() }
    /// ```
    pub fn dt(self) -> Num {
        Num::ratio(1, u64::from(self.rate))
    }

    /// The time elapsed since the game's clock started.
    /// ```
    /// use thrum::{Seconds, Tick};
    /// fn elapsed(tick: Tick) -> Seconds { tick.seconds() }
    /// ```
    pub fn seconds(self) -> Seconds {
        Seconds::from_ticks(self.number.0, self.rate)
    }

    /// Time since the scene was entered.
    /// ```
    /// assert_eq!(thrum::Tick::test(0).in_scene(), thrum::Seconds::ZERO);
    /// ```
    pub fn in_scene(self) -> Seconds {
        Seconds::from_ticks(
            self.number.0.saturating_sub(self.scene_entered.0),
            self.rate,
        )
    }

    /// Time on the running phase's clock.
    /// ```
    /// assert_eq!(thrum::Tick::test(0).in_phase(), thrum::Seconds::ZERO);
    /// ```
    pub fn in_phase(self) -> Seconds {
        self.phase_time
    }

    /// Whether the tick is at the end of a whole interval since scene entry.
    /// ```
    /// assert!(thrum::Tick::test(60).every(thrum::seconds(1)));
    /// ```
    pub fn every(self, interval: Seconds) -> bool {
        let elapsed = self.number.0.saturating_sub(self.scene_entered.0);
        elapsed != 0 && elapsed.is_multiple_of(interval.ticks(self.rate).max(1))
    }

    /// Whether the given deadline has been reached.
    /// ```
    /// let tick = thrum::Tick::test(10);
    /// assert!(tick.reached(tick.number()));
    /// ```
    pub fn reached(self, deadline: TickNumber) -> bool {
        self.number >= deadline
    }

    /// Construct a tick at 60 Hz for a pure rule test.
    /// ```
    /// assert_eq!(thrum::Tick::test(60).seconds(), thrum::seconds(1));
    /// ```
    pub fn test(number: u64) -> Self {
        Self {
            number: TickNumber(number),
            rate: 60,
            scene_entered: TickNumber(0),
            phase_time: Seconds::ZERO,
        }
    }

    /// The first tick at or after the given wait.
    /// ```
    /// use thrum::{Tick, TickNumber, seconds};
    /// fn deadline(tick: Tick) -> TickNumber { tick.after(seconds(1)) }
    /// ```
    pub fn after(self, wait: Seconds) -> TickNumber {
        TickNumber(self.number.0.saturating_add(wait.ticks(self.rate)))
    }
}

/// A deadline that can be cleared.
/// ```
/// use thrum::Timer;
/// assert_eq!(Timer::default(), Timer::IDLE);
/// ```
#[derive(Data, Copy)]
pub struct Timer(Option<Window>);

#[derive(Data, Copy)]
struct Window {
    start: TickNumber,
    end: TickNumber,
}

impl Window {
    fn new(tick: Tick, wait: Seconds) -> Self {
        Self {
            start: tick.number(),
            end: tick.after(wait),
        }
    }

    fn left(self, tick: Tick) -> Seconds {
        Seconds::from_ticks(self.end.0.saturating_sub(tick.number.0), tick.rate)
    }

    fn fraction(self, tick: Tick) -> Num {
        let elapsed = tick.number.0.saturating_sub(self.start.0);
        let length = self.end.0 - self.start.0;
        if elapsed >= length {
            Num::ONE
        } else {
            Num::ratio(elapsed, length)
        }
    }
}

impl Timer {
    /// A timer with no deadline.
    /// ```
    /// assert_eq!(thrum::Timer::IDLE, thrum::Timer::default());
    /// ```
    pub const IDLE: Self = Self(None);

    /// Set a deadline relative to this tick.
    /// ```
    /// use thrum::{Tick, Timer, seconds};
    /// fn start(tick: Tick) -> Timer { Timer::after(tick, seconds(2)) }
    /// ```
    pub fn after(tick: Tick, wait: Seconds) -> Self {
        Self(Some(Window::new(tick, wait)))
    }

    /// Replace this timer's deadline.
    /// ```
    /// use thrum::{Tick, Timer, seconds};
    /// fn restart(timer: &mut Timer, tick: Tick) { timer.start(tick, seconds(2)); }
    /// ```
    pub fn start(&mut self, tick: Tick, wait: Seconds) {
        *self = Self::after(tick, wait);
    }

    /// Clear the deadline and return to idle.
    /// ```
    /// let mut timer = thrum::Timer::IDLE;
    /// timer.clear();
    /// assert_eq!(timer, thrum::Timer::IDLE);
    /// ```
    pub fn clear(&mut self) {
        *self = Self::IDLE;
    }

    /// Whether a set deadline lies in the future.
    /// ```
    /// use thrum::{Tick, Timer};
    /// fn waiting(tick: Tick) -> bool { Timer::IDLE.running(tick) }
    /// ```
    pub fn running(&self, tick: Tick) -> bool {
        self.0.is_some_and(|wait| tick.number < wait.end)
    }

    /// Whether a set deadline has been reached.
    /// ```
    /// use thrum::{Tick, Timer};
    /// fn done(tick: Tick) -> bool { Timer::IDLE.finished(tick) }
    /// ```
    pub fn finished(&self, tick: Tick) -> bool {
        self.0.is_some_and(|wait| tick.number >= wait.end)
    }

    /// Time remaining, or zero when idle or finished.
    /// ```
    /// use thrum::{Seconds, Tick, Timer};
    /// fn waiting(tick: Tick) -> Seconds { Timer::IDLE.left(tick) }
    /// ```
    pub fn left(&self, tick: Tick) -> Seconds {
        self.0.map_or(Seconds::ZERO, |wait| wait.left(tick))
    }

    /// Share of a set wait that has elapsed, or zero when idle.
    /// ```
    /// use thrum::{Num, Tick, Timer};
    /// fn elapsed(tick: Tick) -> Num { Timer::IDLE.fraction(tick) }
    /// ```
    pub fn fraction(&self, tick: Tick) -> Num {
        self.0.map_or(Num::ZERO, |wait| wait.fraction(tick))
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::IDLE
    }
}

/// A gate that can be taken once per wait.
/// ```
/// use thrum::Cooldown;
/// assert_eq!(Cooldown::default(), Cooldown::READY);
/// ```
#[derive(Data, Copy)]
pub struct Cooldown(Option<Window>);

impl Cooldown {
    /// A gate ready for an immediate take.
    /// ```
    /// assert_eq!(thrum::Cooldown::READY, thrum::Cooldown::default());
    /// ```
    pub const READY: Self = Self(None);

    /// A gate that first becomes ready after this wait.
    /// ```
    /// use thrum::{Cooldown, Tick, seconds};
    /// fn after(tick: Tick) -> Cooldown { Cooldown::ready_after(tick, seconds(1)) }
    /// ```
    pub fn ready_after(tick: Tick, wait: Seconds) -> Self {
        Self(Some(Window::new(tick, wait)))
    }

    /// Whether this gate can be taken now.
    /// ```
    /// use thrum::{Cooldown, Tick};
    /// fn available(tick: Tick) -> bool { Cooldown::READY.ready(tick) }
    /// ```
    pub fn ready(&self, tick: Tick) -> bool {
        self.0.is_none_or(|wait| tick.number >= wait.end)
    }

    /// Take this gate and start its next wait when it is ready.
    /// ```
    /// use thrum::{Cooldown, Tick, seconds};
    /// fn try_fire(gun: &mut Cooldown, tick: Tick) -> bool { gun.take(tick, seconds(1)) }
    /// ```
    pub fn take(&mut self, tick: Tick, every: Seconds) -> bool {
        if !self.ready(tick) {
            return false;
        }
        *self = Self::ready_after(tick, every);
        true
    }

    /// The time until this gate can next be taken.
    /// ```
    /// use thrum::{Cooldown, Seconds, Tick};
    /// fn waiting(tick: Tick) -> Seconds { Cooldown::READY.left(tick) }
    /// ```
    pub fn left(&self, tick: Tick) -> Seconds {
        self.0.map_or(Seconds::ZERO, |wait| wait.left(tick))
    }

    /// The elapsed share of the wait, or one when ready.
    /// ```
    /// use thrum::{Cooldown, Num, Tick};
    /// fn elapsed(tick: Tick) -> Num { Cooldown::READY.fraction(tick) }
    /// ```
    pub fn fraction(&self, tick: Tick) -> Num {
        self.0.map_or(Num::ONE, |wait| wait.fraction(tick))
    }
}

impl Default for Cooldown {
    fn default() -> Self {
        Self::READY
    }
}

#[derive(Data, Copy, Default)]
pub(crate) struct PhaseTicks {
    pub(crate) elapsed: u64,
    pub(crate) remaining: Option<u64>,
}

impl PhaseTicks {
    pub(crate) fn advance(&mut self) {
        self.elapsed = self.elapsed.saturating_add(1);
        self.remaining = self.remaining.map(|ticks| ticks.saturating_sub(1));
    }

    pub(crate) fn time(self, rate: u32) -> Seconds {
        Seconds::from_ticks(self.elapsed, rate)
    }
}

#[derive(Data, Copy)]
enum ClockValue {
    Authored {
        elapsed: Seconds,
        remaining: Option<Seconds>,
    },
    Captured {
        ticks: PhaseTicks,
        rate: u32,
    },
}

/// A phase's elapsed time and any remaining time.
/// ```
/// use thrum::{PhaseClock, seconds};
/// assert_eq!(PhaseClock::at(seconds(2)).time(), seconds(2));
/// ```
#[derive(Data, Copy)]
pub struct PhaseClock(ClockValue);

impl PhaseClock {
    pub(crate) fn capture(ticks: PhaseTicks, rate: u32) -> Self {
        Self(ClockValue::Captured { ticks, rate })
    }

    pub(crate) fn ticks(self, rate: u32) -> PhaseTicks {
        match self.0 {
            ClockValue::Authored { elapsed, remaining } => PhaseTicks {
                elapsed: elapsed.ticks(rate),
                remaining: remaining.map(|time| time.ticks(rate)),
            },
            ClockValue::Captured { ticks, .. } => ticks,
        }
    }

    /// Make a clock with elapsed time and no scheduled end.
    /// ```
    /// use thrum::{PhaseClock, seconds};
    /// assert_eq!(PhaseClock::at(seconds(3)).left(), None);
    /// ```
    pub fn at(time: Seconds) -> Self {
        Self(ClockValue::Authored {
            elapsed: time,
            remaining: None,
        })
    }

    /// Add remaining time to a clock made by hand.
    /// ```
    /// use thrum::{PhaseClock, seconds};
    /// assert_eq!(PhaseClock::at(seconds(3)).ending_in(seconds(2)).left(), Some(seconds(2)));
    /// ```
    #[must_use]
    pub fn ending_in(self, left: Seconds) -> Self {
        Self(match self.0 {
            ClockValue::Authored { elapsed, .. } => ClockValue::Authored {
                elapsed,
                remaining: Some(left),
            },
            ClockValue::Captured { mut ticks, rate } => {
                ticks.remaining = Some(left.ticks(rate));
                ClockValue::Captured { ticks, rate }
            }
        })
    }

    /// Elapsed time when the clock was read.
    /// ```
    /// use thrum::{PhaseClock, seconds};
    /// assert_eq!(PhaseClock::at(seconds(3)).time(), seconds(3));
    /// ```
    pub fn time(self) -> Seconds {
        match self.0 {
            ClockValue::Authored { elapsed, .. } => elapsed,
            ClockValue::Captured { ticks, rate } => ticks.time(rate),
        }
    }

    /// Remaining time if an end is scheduled and has yet to run.
    /// ```
    /// use thrum::{PhaseClock, seconds};
    /// assert_eq!(PhaseClock::at(seconds(3)).left(), None);
    /// ```
    pub fn left(self) -> Option<Seconds> {
        match self.0 {
            ClockValue::Authored { remaining, .. } => remaining,
            ClockValue::Captured { ticks, rate } => {
                ticks.remaining.map(|left| Seconds::from_ticks(left, rate))
            }
        }
    }
}
