//! Diagnostic input clock. Time values come from view display-link targets.

const PRE_HOLD: f64 = 0.75;
const SWEEP: f64 = 2.0;
const DEGREES_PER_SECOND: f64 = 18.0;
const END_YAW: f64 = SWEEP * DEGREES_PER_SECOND;

#[derive(Debug, PartialEq)]
pub(crate) enum PanAction {
    Hold,
    Begin,
    Move(f64),
    End(f64),
}

pub(crate) struct PanTimeline {
    start: f64,
    previous: f64,
    begun: bool,
    ended: bool,
}

impl PanTimeline {
    pub(crate) fn armed(target: f64) -> Result<Self, &'static str> {
        if !target.is_finite() {
            return Err("nonfinite pan arm target");
        }
        Ok(Self {
            start: target + PRE_HOLD,
            previous: target,
            begun: false,
            ended: false,
        })
    }

    pub(crate) fn sample(&mut self, target: f64) -> Result<PanAction, &'static str> {
        if !target.is_finite() || target < self.previous {
            return Err("pan display targets must be finite and monotone");
        }
        if target == self.previous || self.ended || target < self.start {
            self.previous = target;
            return Ok(PanAction::Hold);
        }
        self.previous = target;
        let elapsed = target - self.start;
        if !self.begun {
            if elapsed >= SWEEP {
                return Err("first pan target missed the entire sweep");
            }
            self.begun = true;
            self.start = target;
            return Ok(PanAction::Begin);
        }
        if elapsed >= SWEEP {
            self.ended = true;
            return Ok(PanAction::End(END_YAW));
        }
        Ok(PanAction::Move(elapsed * DEGREES_PER_SECOND))
    }
}

#[cfg(test)]
mod tests {
    use super::{PanAction, PanTimeline};

    #[test]
    fn pan_uses_elapsed_time_and_dispatches_one_endpoint() {
        for hz in [60.0, 120.0] {
            let mut pan = PanTimeline::armed(1000.0).expect("valid clock");
            assert_eq!(pan.sample(1000.5), Ok(PanAction::Hold));
            assert_eq!(pan.sample(1000.75), Ok(PanAction::Begin));
            let next = 1000.75 + 1.0 / hz;
            let Ok(PanAction::Move(yaw)) = pan.sample(next) else {
                panic!("expected a new yaw");
            };
            assert!((yaw - 18.0 / hz).abs() < 1e-9);
            assert_eq!(pan.sample(next), Ok(PanAction::Hold));
            assert_eq!(pan.sample(1002.75), Ok(PanAction::End(36.0)));
            assert_eq!(pan.sample(1003.0), Ok(PanAction::Hold));
        }
    }

    #[test]
    fn a_delayed_callback_uses_its_time_instead_of_replaying_stale_steps() {
        let mut pan = PanTimeline::armed(1000.0).expect("valid clock");
        assert_eq!(pan.sample(1000.75), Ok(PanAction::Begin));
        assert_eq!(pan.sample(1001.75), Ok(PanAction::Move(18.0)));
        assert!(pan.sample(1001.0).is_err());
        assert!(pan.sample(f64::NAN).is_err());
    }

    #[test]
    fn a_late_start_still_gets_the_full_sweep() {
        let mut pan = PanTimeline::armed(1000.0).expect("valid clock");
        assert_eq!(pan.sample(1000.875), Ok(PanAction::Begin));
        assert_eq!(pan.sample(1002.75), Ok(PanAction::Move(33.75)));
        assert_eq!(pan.sample(1002.875), Ok(PanAction::End(36.0)));
    }
}
