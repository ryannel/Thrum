//! Sample ordinary buffered motion, underrun recovery, and a deliberate skip.
/// The shared integer rule set and carrier fixture.
use crate::support;

use support::{Match, local};
use thrum_network::session::{Controls, Playout, Settings, Skip, World};

#[derive(Clone, Copy)]
struct Frame {
    at: u64,
    playout: Playout,
    committed: u64,
    held: u64,
    skip: Option<Skip>,
}

#[test]
fn bursts_underruns_and_behind_have_distinct_positions() {
    for (catch_up, slow_down) in [(1_100, 850), (1_175, 925)] {
        check_bursts(Settings {
            digest_interval: 1,
            playout_catch_up: catch_up,
            playout_slow_down: slow_down,
            ..Settings::default()
        });
    }
}

fn check_bursts(settings: Settings) {
    let mut fixture = Match::with_settings(local(), 1, None, false, settings);
    fixture.ready();
    assert_eq!(fixture.clients[0].settings(), Some(settings));
    let catch_up = f64::from(settings.playout_catch_up) / 1_000.0;
    let period = f64::from(settings.tick_us);
    while fixture.host.world().tick() < 20 {
        fixture.neutral(400);
    }
    let mut frames = Vec::new();
    let mut stalls = Vec::new();
    let mut windows = Vec::new();
    for (pause, follow) in [(30_000, 160_000), (130_000, 350_000), (1_050_000, 400_000)] {
        let first_frame = frames.len();
        let start = fixture.now;
        fixture.memory.stall(true);
        while fixture.now < start + pause {
            sample(&mut fixture, &mut frames, &mut stalls);
        }
        fixture.memory.stall(false);
        while fixture.now < start + pause + follow {
            sample(&mut fixture, &mut frames, &mut stalls);
        }
        windows.push((start, fixture.now));
        assert!(
            frames[first_frame..]
                .windows(2)
                .any(|pair| pair[1].held >= pair[0].held + 2)
        );
    }
    assert!(
        assert_motion(&frames, &windows, period, catch_up).is_ok(),
        "{:?}",
        assert_motion(&frames, &windows, period, catch_up)
    );
    assert!(
        stalls
            .iter()
            .any(|stall| *stall >= 40_000 && *stall < 200_000),
        "{stalls:?}"
    );
    assert!(stalls.iter().any(|stall| *stall >= 800_000), "{stalls:?}");

    // In the control every held set commits as soon as it arrives. Its frame
    // interpolates the last two of those endpoints at the ordinary rate.
    let mut last = (0, frames[0].at);
    let control = frames
        .iter()
        .map(|frame| {
            if frame.held != last.0 {
                last = (frame.held, frame.at);
            }
            let alpha = ((frame.at - last.1) as f64 / f64::from(fixture.settings.tick_us)).min(1.0);
            Frame {
                committed: frame.held,
                skip: None,
                playout: Playout {
                    position: frame.held.saturating_sub(1) as f64 + alpha,
                    rate: 1.0,
                },
                ..*frame
            }
        })
        .collect::<Vec<_>>();
    assert!(assert_motion(&control, &windows, period, catch_up).is_err());
}

#[test]
fn a_smaller_display_budget_records_holding_at_the_work_cap() {
    let mut fixture = Match::new(local(), 1, None, false);
    fixture.ready();
    while fixture.host.world().tick() < 50 {
        fixture.neutral(200);
    }
    let start = fixture.now;
    let before = fixture.clients[0].world().tick();
    let mut samples = [None; 3];
    for (sample, offset) in samples.iter_mut().zip([50_000, 58_333, 66_666]) {
        let now = start + offset;
        let target = now + 8_333;
        fixture.memory.advance(now / 1_000);
        fixture.host.advance(now, 200);
        fixture.memory.advance(now / 1_000);
        let client = &mut fixture.clients[0];
        client
            .advance_for_display(now, target, 1, |_| Controls::axes([0.0; 3]), |_| {})
            .expect("bounded display advance");
        assert!(client.held() > client.world().tick());
        assert!(client.take_skip().is_none());
        *sample = Some((client.playout(target).unwrap(), client.world().tick()));
        if offset == 50_000 {
            assert_eq!(client.playout(target + 4_166), client.playout(target));
            assert!(client.take_stall().is_none());
        } else if offset == 58_333 {
            let stall = client
                .take_stall()
                .expect("work-cap stall ended on recovery");
            assert!(stall.work_cap);
            assert_eq!(stall.position.to_bits(), ((before + 1) as f64).to_bits());
            assert!((40_000..65_000).contains(&stall.duration_us), "{stall:?}");
        }
    }
    let [(held, committed), (resumed, next), (moving, _)] = samples.map(Option::unwrap);
    assert_eq!(committed, before + 1);
    assert_eq!(next, committed + 1);
    assert_eq!(
        held,
        Playout {
            position: committed as f64,
            rate: 0.0
        }
    );
    assert_eq!(resumed.position.to_bits(), held.position.to_bits());
    let fastest = 8_333.0 / f64::from(fixture.settings.tick_us)
        * f64::from(fixture.settings.playout_catch_up)
        / 1_000.0;
    assert!(moving.position > resumed.position && moving.position <= resumed.position + fastest);
}

#[test]
fn a_stall_includes_the_interval_ending_at_the_recovery_poll() {
    let mut fixture = Match::new(local(), 1, None, false);
    fixture.ready();
    while fixture.host.world().tick() < 25 {
        fixture.neutral(100);
    }
    fixture.memory.stall(true);
    let (start, held) = loop {
        let at = fixture.now;
        fixture.neutral(100);
        let playout = fixture.clients[0].playout(at).unwrap();
        if playout.rate == 0.0 {
            break (at, playout.position);
        }
        assert!(fixture.host.world().tick() < 50);
    };
    fixture.memory.stall(false);
    for _ in 0..80 {
        fixture.memory.advance(fixture.now / 1_000);
        fixture.host.advance(fixture.now, 100);
        fixture.now += 2_500;
    }
    let resumed = fixture.now;
    assert_eq!(
        fixture.clients[0]
            .playout(resumed - 1)
            .unwrap()
            .position
            .to_bits(),
        held.to_bits()
    );
    fixture.neutral(100);
    let client = &mut fixture.clients[0];
    assert_eq!(
        client.playout(resumed).unwrap().position.to_bits(),
        held.to_bits()
    );
    assert!(client.take_skip().is_none());
    let stall = client.take_stall().expect("stall completed on recovery");
    assert!(
        (resumed - start..=resumed - start + 2_500).contains(&stall.duration_us),
        "{stall:?}"
    );
}

fn sample(fixture: &mut Match, frames: &mut Vec<Frame>, stalls: &mut Vec<u64>) {
    let at = fixture.now;
    fixture.neutral(400);
    let client = &mut fixture.clients[0];
    let mut skip = client.take_skip();
    if let Some(stall) = client.take_stall() {
        stalls.push(stall.duration_us);
    }
    for at in [at, at + 1_250] {
        frames.push(Frame {
            at,
            playout: client.playout(at).expect("playing"),
            committed: client.world().tick(),
            held: client.held(),
            skip: skip.take(),
        });
    }
}

fn assert_motion(
    frames: &[Frame],
    windows: &[(u64, u64)],
    period: f64,
    catch_up: f64,
) -> Result<(), String> {
    let mut first_skip = None;
    let mut last_skip = None;
    let mut caught_up = false;
    let mut underrun = false;
    for (index, frame) in frames.iter().enumerate() {
        let frame_at = frame.at;
        let position = frame.playout.position;
        let in_window = |window: usize| (windows[window].0..windows[window].1).contains(&frame_at);
        if position < frame.committed.saturating_sub(1) as f64 || position > frame.committed as f64
        {
            return Err(format!(
                "position outside committed endpoints at {frame_at}: {position} / {}",
                frame.committed
            ));
        }
        if let Some(previous) = index.checked_sub(1).map(|index| &frames[index]) {
            let travel = position - previous.playout.position;
            let limit = catch_up * (frame_at - previous.at) as f64 / period;
            if travel < -1e-8 || frame.skip.is_none() && travel > limit + 1e-8 {
                return Err(format!(
                    "position changed by {travel} at {frame_at}, limit {limit}"
                ));
            }
            if frame.skip.is_none()
                && frame.committed > previous.committed
                && position < (frame.committed - 1) as f64
            {
                return Err(format!("committed ahead of position at {frame_at}"));
            }
        }
        if in_window(0)
            && (frame.playout.rate.to_bits() != 1.0_f64.to_bits() || frame.skip.is_some())
        {
            return Err(format!(
                "burst inside buffer changed playout at {frame_at}: {:?}",
                frame.playout
            ));
        }
        if in_window(1) {
            underrun |= frame.playout.rate == 0.0;
            caught_up |= underrun && frame.playout.rate.to_bits() == catch_up.to_bits();
        }
        if let Some(skip) = frame.skip {
            if !in_window(2)
                || skip.first > skip.last
                || skip.last != frame.committed
                || last_skip.is_some_and(|last| skip.first != last + 1)
            {
                return Err(format!("unexpected skip {skip:?} at {frame_at}"));
            }
            first_skip.get_or_insert(skip.first);
            last_skip = Some(skip.last);
        }
    }
    if !underrun
        || !caught_up
        || first_skip
            .zip(last_skip)
            .is_none_or(|(first, last)| last - first < 30)
    {
        return Err(format!(
            "missing recovery witness: underrun={underrun}, catch-up={caught_up}, skip={first_skip:?}..{last_skip:?}"
        ));
    }
    Ok(())
}
