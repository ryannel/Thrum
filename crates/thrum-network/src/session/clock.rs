use std::cell::Cell;

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
    drawn_edge: Cell<Option<(u64, f64)>>,
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
            drawn_edge: Cell::new(None),
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
        let wanted = self.wanted(now, settings);
        let position = wanted.min(committed as f64);
        if wanted > position && self.drawn_edge.get().is_none_or(|(at, _)| now >= at) {
            self.drawn_edge.set(Some((now, position)));
        }
        Playout {
            position,
            rate: self.rate,
        }
    }

    pub(super) fn interpolation(&self, now: u64, committed: u64, settings: Settings) -> f64 {
        (self.wanted(now, settings) - committed.saturating_sub(1) as f64).clamp(0.0, 1.0)
    }

    pub(super) fn behind(&self) -> bool {
        self.skipping || self.skip.is_some()
    }

    pub(super) fn stalled(&self) -> bool {
        self.stall.is_some()
    }

    fn wanted(&self, now: u64, settings: Settings) -> f64 {
        let elapsed = now.saturating_sub(self.at) as f64 / f64::from(settings.tick_us);
        self.position + elapsed * self.rate
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
        let drawn_edge = self
            .drawn_edge
            .take()
            .filter(|(at, _)| (self.at..=now).contains(at));
        self.at = now;
        self.skip = None;
        self.completed_stall = None;
        if let Some(stall) = &mut self.stall {
            stall.duration_us += elapsed;
        }
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
            f64::from(settings.playout_catch_up) / 1_000.0
        } else if self.position > target + 6.0 {
            f64::from(settings.playout_slow_down) / 1_000.0
        } else {
            1.0
        };
        let travel_rate = if self.stall.is_some() { 0.0 } else { self.rate };
        let advance =
            |microseconds| microseconds as f64 * travel_rate / f64::from(settings.tick_us);
        let mut desired = self.position + advance(elapsed);
        // Frames already drawn at the committed edge cannot borrow time from
        // the part of the interval they spent there.
        if let Some((at, edge)) = drawn_edge {
            desired = desired.min(edge + advance(now - at));
        }
        let mut committed = stepped;
        for _ in 0..settings.catch_up {
            if committed >= held || desired < committed as f64 || !commit() {
                break;
            }
            committed += 1;
        }
        let previous = self.position;
        self.position = desired.min(committed as f64);
        let blocked = self.position >= committed as f64 && committed == held;
        let work_cap = self.position >= committed as f64
            && committed < held
            && committed - stepped == u64::from(settings.catch_up);
        if blocked || work_cap {
            self.stall.get_or_insert_with(|| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "elapsed travel to the held edge is nonnegative and rounded up to whole microseconds"
                )]
                let travel_us = ((self.position - previous) * f64::from(settings.tick_us)
                    / travel_rate)
                    .ceil() as u64;
                Stall {
                    position: self.position,
                    duration_us: elapsed.saturating_sub(travel_us),
                    work_cap,
                }
            });
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
    credit: u64,
    credit_tick: u64,
    last_authored: u64,
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
        let half_rtt = rtt.div_ceil(2 * period);
        // The fixed control pins the lead relative to the same host estimate used
        // when reporting lead. Adaptive authoring starts from the full round trip.
        let (present, lead) = pinned.map_or((rtt.div_ceil(period), half_rtt + 2), |lead| {
            (half_rtt, u64::from(lead))
        });
        Self {
            // The first author call increments the tick before stamping the frame.
            target: (held + present + lead.min(u64::from(settings.staging))) - 1,
            next: now,
            half_rtt,
            floor: i16::MAX,
            samples: 0,
            adjust_at: now + 30 * period,
            margin_sequence: 0,
            pinned,
            credit: settings.input_burst() / 2,
            credit_tick: held,
            last_authored: 0,
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
        // A slow poll can still fill the future slots for recent authoring
        // periods. Skip older debt so resuming a suspended client stays bounded.
        let due = (now - self.next) / period + 1;
        let ticks = 1 + due.saturating_sub(u64::from(settings.catch_up));
        self.next += ticks * period;
        self.target += ticks;
        if self.pinned.is_none() && self.target <= held && self.floor < 0 {
            // The newest received set already proves this target has passed.
            // A long authority catch-up burst can overtake many of our frame
            // periods at once. Re-anchor instead of sending obsolete frames
            // for several seconds while the ordinary margin correction catches up.
            self.target = held + 3 * self.half_rtt + 2;
            self.next = now + period;
            self.adjust_at = now + 30 * period;
            self.floor = i16::MAX;
            self.samples = 0;
            self.changes += 1;
        } else if now >= self.adjust_at && self.samples >= 6 {
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
        // The authority may run slower than our wall clock, including during
        // a busy tick. Stop advancing at the staging limit until more sets
        // arrive. A target that advanced past the limit could never produce
        // the next margin needed to bring itself back.
        let cap = held + u64::from(settings.staging);
        self.target = self.target.min(cap);
        // Pace new input against observed authority progress. Use only half the
        // host's burst allowance so delivery jitter and outstanding datagrams
        // do not exhaust the host's elapsed-time credit.
        let progressed = held.saturating_sub(self.credit_tick);
        self.credit_tick = self.credit_tick.max(held);
        self.credit = (self.credit + progressed).min(settings.input_burst() / 2);
        if self.credit == 0 || self.target <= self.last_authored {
            return None;
        }
        self.credit -= 1;
        self.last_authored = self.target;
        Some(self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_frame_uses_the_held_set_full_round_trip_and_starting_lead() {
        let settings = Settings::default();
        let now = 31_000_000;
        let newest_set = 27;
        // 100 ms is six periods rounded up. Its 50 ms half is three periods.
        let mut adaptive = Lead::new(now, newest_set, 100_000, settings, None);
        let mut pinned = Lead::new(now, newest_set, 100_000, settings, Some(5));
        for (lead, first) in [(&mut adaptive, 27 + 6 + 5), (&mut pinned, 27 + 3 + 5)] {
            assert_eq!(lead.half_rtt, 3);
            assert_eq!(lead.author(now, newest_set, settings), Some(first));
            assert_eq!(lead.author(now, newest_set, settings), None);
            assert_eq!(
                lead.author(now + u64::from(settings.tick_us), newest_set, settings),
                Some(first + 1)
            );
        }
    }

    #[test]
    fn a_stalled_authority_does_not_leave_the_next_frame_past_the_staging_window() {
        let settings = Settings::default();
        let period = u64::from(settings.tick_us);
        let held = 50;
        let mut lead = Lead::new(0, held, 6 * period, settings, None);
        let cap = held + u64::from(settings.staging);
        assert_eq!(lead.author(300 * period, held, settings), Some(cap));
        assert_eq!(lead.author(301 * period, held, settings), None);
        assert_eq!(lead.author(302 * period, held + 1, settings), Some(cap + 1));
    }

    #[test]
    fn negative_feedback_recovers_an_authoring_target_overtaken_by_the_host() {
        let settings = Settings::default();
        let period = u64::from(settings.tick_us);
        let mut lead = Lead::new(0, 50, 6 * period, settings, None);
        let first = lead.author(0, 50, settings).expect("first frame");
        lead.observe(1, -100);
        let recovered = lead
            .author(period, first + 100, settings)
            .expect("recovered frame");
        assert!(recovered > first + 100);
        assert_eq!(lead.author(period, first + 100, settings), None);
    }

    #[test]
    fn a_delayed_poll_can_author_each_recent_future_frame() {
        let settings = Settings::default();
        let period = u64::from(settings.tick_us);
        let mut lead = Lead::new(0, 50, 6 * period, settings, None);
        let first = lead.author(0, 50, settings).expect("first frame");
        for offset in 1..=4 {
            assert_eq!(lead.author(4 * period, 54, settings), Some(first + offset));
        }
        assert_eq!(lead.author(4 * period, 54, settings), None);
    }

    #[test]
    fn a_faster_client_earns_fresh_frame_credit_from_authority_progress() {
        let settings = Settings::default();
        let period = u64::from(settings.tick_us);
        let mut lead = Lead::new(0, 50, 6 * period, settings, None);
        let authored = (0..80)
            .filter(|offset| lead.author(offset * period, 50, settings).is_some())
            .count();
        assert_eq!(authored as u64, settings.input_burst() / 2);
        assert!(lead.author(80 * period, 51, settings).is_some());
        assert_eq!(lead.author(81 * period, 51, settings), None);
    }

    #[test]
    fn leading_the_buffer_uses_the_negotiated_slow_down_rate() {
        let settings = Settings {
            playout_slow_down: 925,
            ..Settings::default()
        };
        let mut clock = Clock::new(10, 0);
        clock.buffer = 8;
        clock.advance(0, 10, 11, settings, || true);
        assert_eq!(
            clock.sample(0, 11, settings).rate.to_bits(),
            0.925_f64.to_bits()
        );
    }
}
