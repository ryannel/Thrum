//! Sample ordinary buffered motion, underrun recovery, and a deliberate skip.
/// The shared integer rule set and carrier fixture.
#[path = "support/session.rs"]
pub mod session_fixture;

use session_fixture::{Match, local};
use thrum_network::session::{Playout, Skip, World};

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
    let mut fixture = Match::new(local(), 1, None, false);
    fixture.ready();
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
        assert_motion(&frames, &windows, f64::from(fixture.settings.tick_us)).is_ok(),
        "{:?}",
        assert_motion(&frames, &windows, f64::from(fixture.settings.tick_us))
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
    assert!(assert_motion(&control, &windows, f64::from(fixture.settings.tick_us)).is_err());
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

fn assert_motion(frames: &[Frame], windows: &[(u64, u64)], period: f64) -> Result<(), String> {
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
            let limit = 1.1 * (frame_at - previous.at) as f64 / period;
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
            caught_up |= underrun && frame.playout.rate > 1.0;
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
