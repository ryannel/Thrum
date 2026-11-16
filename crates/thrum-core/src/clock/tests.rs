use super::{ClockConfig, TickClock};
use std::{num::NonZeroU32, time::Duration};

#[test]
fn observation_acceptance_uses_the_endpoint_before_float_rounding() {
    for (hz, before, at) in [(60, 16_666_666, 16_666_667), (100, 19_999_999, 20_000_000)] {
        let mut clock = TickClock::new(
            ClockConfig {
                tick_hz: NonZeroU32::new(hz).expect("nonzero rate"),
                ..ClockConfig::default()
            },
            Duration::ZERO,
        )
        .expect("clock");
        clock
            .advance(Duration::from_millis(25), 4, || Ok::<_, ()>(()))
            .expect("script");
        let sample = clock.presentation_sample();
        assert!(!sample.accepts_observation(Duration::from_nanos(before)));
        assert!(sample.accepts_observation(Duration::from_nanos(at)));
        assert!(sample.accepts_observation(Duration::from_secs(1)));
        clock.pause();
        // Endpoint-only clocks consume no target, even for an old observation.
        assert!(
            clock
                .presentation_sample()
                .accepts_observation(Duration::ZERO)
        );
    }
}
