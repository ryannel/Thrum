//! Opt-in headless capture admission; the renderer tap remains nonblocking.
use super::{
    Duration, Game, ImageWriter, Instant, Offscreen, OpportunityClock, Trace, Value, json,
};

pub(super) struct Admission {
    pub(super) policy: &'static str,
    hold: Duration,
    hold_started: Option<Instant>,
    hold_finished: bool,
    pub(super) waits: Vec<Value>,
    pub(super) skipped: u32,
}

impl Admission {
    pub(super) fn new(wait: bool, hold: Duration, mode: &str) -> Result<Self, String> {
        if wait && mode != "realtime" {
            return Err("wait admission requires realtime mode".to_owned());
        }
        Ok(Self {
            policy: if wait { "wait" } else { "omit" },
            hold,
            hold_started: None,
            hold_finished: false,
            waits: Vec::new(),
            skipped: 0,
        })
    }

    // Diagnostic only: withhold image collection until all six slots are full,
    // then hold them for the declared time. GPU frame reports still drain.
    pub(super) fn collect_images<G: Game>(
        &mut self,
        driver: &Offscreen<G>,
        origin: Instant,
    ) -> bool {
        if self.hold.is_zero() || self.hold_finished || origin.elapsed() < Duration::from_secs(1) {
            return true;
        }
        if driver.capture_usage().is_some_and(|usage| usage.0 == 6) {
            self.hold_started.get_or_insert_with(Instant::now);
        }
        if self
            .hold_started
            .is_some_and(|start| start.elapsed() >= self.hold)
        {
            self.hold_finished = true;
            return true;
        }
        false
    }

    pub(super) fn wait<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        trace: &mut Trace,
        writer: Option<&ImageWriter>,
        origin: Instant,
        clock: &mut OpportunityClock,
    ) -> Result<(), String> {
        if self.policy != "wait" || driver.capture_usage().is_none_or(|usage| usage.0 != 6) {
            return Ok(());
        }
        let busy = clock.discard_elapsed();
        let start = origin.elapsed();
        let deadline = Instant::now() + Duration::from_secs(10);
        while driver.capture_usage().is_some_and(|usage| usage.0 == 6) {
            let collect = self.collect_images(driver, origin);
            trace.drain(driver, writer, origin, collect)?;
            if Instant::now() >= deadline {
                return Err("capture admission wait deadline".to_owned());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let skipped = clock.discard_elapsed();
        self.skipped = self.skipped.saturating_add(skipped);
        self.waits.push(
            json!({"reason":"capture admission wait","start_ns":start.as_nanos(),
            "end_ns":origin.elapsed().as_nanos(),"skipped_opportunities":skipped,
            "busy_skipped_before":busy,"busy_skip_reason":"driver busy or host oversleep"}),
        );
        Ok(())
    }

    pub(super) fn diagnostic(&self) -> Value {
        json!({"hold_full_ms":self.hold.as_millis(),"filled":self.hold_started.is_some(),
            "released":self.hold_finished})
    }
}
