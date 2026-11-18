use super::Settings;

pub(super) const INITIAL_BUFFER: u64 = 4;

/// A recorded interval when the position could not advance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stall {
    /// Held position, in ticks.
    pub position: f64,
    /// Duration in the client's microseconds.
    pub duration_us: u64,
    /// Whether the per-advance commit cap, rather than a missing set, held it.
    pub work_cap: bool,
}

/// A recorded jump after the backlog overtook the behind threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skip {
    /// First tick committed during the skip.
    pub first: u64,
    /// Last tick committed during the skip.
    pub last: u64,
}

/// Client presentation position sampled independently of committing ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playout {
    /// Fractional tick bracketed by the last two committed endpoints.
    pub position: f64,
    /// Current rate in ticks per tick period.
    pub rate: f64,
}

pub(super) struct Clock {
    position: f64,
    rate: f64,
    at: u64,
    buffer: u16,
    stall: Option<Stall>,
    pub(super) skip: Option<Skip>,
    pub(super) completed_stall: Option<Stall>,
    arrival: Option<(u64, u64)>,
    on_time: u16,
    skipping: bool,
}

impl Clock {
    pub(super) fn new(tick: u64, now: u64) -> Self {
        Self {
            position: tick as f64,
            rate: 1.0,
            at: now,
            buffer: 4,
            stall: None,
            skip: None,
            completed_stall: None,
            arrival: None,
            on_time: 0,
            skipping: false,
        }
    }

    pub(super) fn observe(&mut self, held: u64, now: u64, settings: Settings) {
        if let Some((last, time)) = self.arrival {
            let delay = now.saturating_sub(time) / u64::from(settings.tick_us);
            if delay > (held - last).max(2) + 1 && delay <= u64::from(settings.behind) {
                self.buffer = self
                    .buffer
                    .max(u16::try_from(delay).expect("bounded lag"))
                    .min(settings.behind / 2);
                self.on_time = 0;
            } else {
                self.on_time += 1;
                if self.on_time >= 180 {
                    self.buffer = self.buffer.saturating_sub(1).max(4);
                    self.on_time = 0;
                }
            }
        }
        self.arrival = Some((held, now));
    }

    // Interpolation cannot cross the committed endpoint between polls.
    pub(super) fn sample(&self, now: u64, committed: u64, settings: Settings) -> Playout {
        let elapsed = now.saturating_sub(self.at) as f64 / f64::from(settings.tick_us);
        Playout {
            position: (self.position + elapsed * self.rate).min(committed as f64),
            rate: self.rate,
        }
    }

    pub(super) fn advance(
        &mut self,
        now: u64,
        stepped: u64,
        held: u64,
        settings: Settings,
        mut commit: impl FnMut() -> bool,
    ) {
        let elapsed = now.saturating_sub(self.at);
        self.at = now;
        self.skip = None;
        self.completed_stall = None;
        self.skipping |= held - stepped > u64::from(settings.behind);
        if self.skipping {
            let mut last = stepped;
            for _ in 0..settings.catch_up {
                if held - last <= u64::from(self.buffer) {
                    break;
                }
                if !commit() {
                    break;
                }
                last += 1;
            }
            if last > stepped {
                self.position = last as f64;
                self.skip = Some(Skip {
                    first: stepped + 1,
                    last,
                });
            }
            self.rate = 1.0;
            self.skipping = held - last > u64::from(self.buffer);
            self.completed_stall = self.stall.take();
            return;
        }

        let target = held.saturating_sub(u64::from(self.buffer)) as f64;
        let next_rate = if self.position + 2.0 < target {
            1.1
        } else if self.position > target + 6.0 {
            0.85
        } else {
            1.0
        };
        let travel_rate = if self.stall.is_some() { 0.0 } else { self.rate };
        let desired = self.position + elapsed as f64 * travel_rate / f64::from(settings.tick_us);
        let mut committed = stepped;
        for _ in 0..settings.catch_up {
            if committed >= held || desired < committed as f64 || !commit() {
                break;
            }
            committed += 1;
        }
        self.position = desired.min(committed as f64);
        let blocked = self.position >= committed as f64 && committed == held;
        let work_cap = self.position >= committed as f64
            && committed < held
            && committed - stepped == u64::from(settings.catch_up);
        if blocked || work_cap {
            let stall = self.stall.get_or_insert(Stall {
                position: self.position,
                duration_us: 0,
                work_cap,
            });
            stall.duration_us += elapsed;
            self.rate = 0.0;
        } else {
            self.completed_stall = self.stall.take();
            self.rate = next_rate;
        }
    }
}

pub(super) struct Lead {
    pub(super) target: u64,
    next: u64,
    pub(super) half_rtt: u64,
    floor: i16,
    samples: u16,
    adjust_at: u64,
    margin_sequence: u64,
    pinned: Option<u16>,
    pub(super) changes: u64,
}

impl Lead {
    pub(super) fn new(
        now: u64,
        held: u64,
        rtt: u64,
        settings: Settings,
        pinned: Option<u16>,
    ) -> Self {
        let period = u64::from(settings.tick_us);
        let half_rtt = (rtt / 2).div_ceil(period);
        let lead = pinned.map_or(half_rtt + 2, u64::from);
        Self {
            target: held + half_rtt + lead.min(u64::from(settings.staging)),
            next: now,
            half_rtt,
            floor: i16::MAX,
            samples: 0,
            adjust_at: now + 30 * period,
            margin_sequence: 0,
            pinned,
            changes: 0,
        }
    }

    pub(super) fn observe(&mut self, sequence: u64, margin: i16) {
        if sequence <= self.margin_sequence || self.pinned.is_some() {
            return;
        }
        self.margin_sequence = sequence;
        self.floor = self.floor.min(margin);
        self.samples += 1;
    }

    pub(super) fn author(&mut self, now: u64, held: u64, settings: Settings) -> Option<u64> {
        if now < self.next {
            return None;
        }
        let period = u64::from(settings.tick_us);
        let ticks = (now - self.next) / period + 1;
        self.next += ticks * period;
        self.target += ticks;
        if now >= self.adjust_at && self.samples >= 6 {
            let adjustment = if self.floor < 1 {
                u64::try_from(2 - i32::from(self.floor))
                    .expect("positive adjustment")
                    .min(8)
            } else {
                0
            };
            let previous = self.target;
            self.target += adjustment;
            if self.floor > 2 && ticks == 1 {
                self.target -= 1;
            }
            self.adjust_at = now + if adjustment == 0 { 60 } else { 18 } * period;
            self.floor = i16::MAX;
            self.samples = 0;
            self.changes += u64::from(previous != self.target);
        }
        let cap = held + self.half_rtt + u64::from(settings.staging);
        (self.target <= cap).then_some(self.target)
    }
}
