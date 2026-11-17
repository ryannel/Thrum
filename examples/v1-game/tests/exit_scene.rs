//! Walk the exit authority with live peer input, retained correction and one game hour.
use std::{hash::Hasher, num::NonZeroUsize};
use thrum_core::{ActorId, Origin, PeerId, StableId, sim::InputFrame};
use v1_game::{
    exit_authority,
    simulation::{SimAction, Simulation},
};

// The typed authority data is compared byte for byte. Preserve the engine's
// complete metadata visitation contract without reducing it to a digest.
#[derive(Default)]
struct Metadata(Vec<u8>);
impl Hasher for Metadata {
    fn finish(&self) -> u64 {
        panic!("metadata comparison does not use a digest")
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

fn state(sim: &Simulation) -> Metadata {
    let mut bytes = Metadata::default();
    sim.capture().hash_authority_metadata(&mut bytes);
    bytes
}

fn input(peer: u64, tick: u64, corrected: bool) -> InputFrame<SimAction> {
    let origin = Origin {
        peer: PeerId(peer),
        connection_epoch: 1,
        sequence: tick,
    };
    let x = if (peer, tick, corrected) == (2, 5, true) {
        -0.5
    } else {
        0.25
    };
    InputFrame::new(origin, ActorId(peer), tick, [x, 0.0, 0.0])
}

#[test]
fn corrected_exit_replays_complete_authority() {
    let mut predicted = exit_authority().expect("exit authority");
    let mut straight = exit_authority().expect("exit authority");
    let mut history = predicted
        .begin_history(NonZeroUsize::new(12).unwrap())
        .expect("history");
    for tick in 1..=12 {
        for peer in 1..=4 {
            predicted
                .admit_frame(input(peer, tick, false))
                .expect("predicted input");
            straight
                .admit_frame(input(peer, tick, true))
                .expect("straight input");
        }
        history.step(&mut predicted, &[]).expect("predicted tick");
        let outcome = straight.step(&[]).expect("straight tick");
        assert_eq!(outcome.simulated_entities, 10_000);
        assert_eq!(outcome.changed_positions, 1_504);
    }
    let changed = StableId(9_998);
    assert_ne!(
        predicted
            .get(predicted.handle(changed).unwrap())
            .unwrap()
            .position,
        straight
            .get(straight.handle(changed).unwrap())
            .unwrap()
            .position,
    );
    history.rewind(&mut predicted, 8).expect("rewind");
    history
        .amend_input(&predicted, input(2, 5, true))
        .expect("amend");
    for _ in 0..8 {
        assert!(history.replay_next(&mut predicted).expect("replay"));
    }
    assert!(!history.replay_next(&mut predicted).expect("end"));
    assert_eq!(
        (predicted.tick(), predicted.len()),
        (straight.tick(), straight.len())
    );
    assert!(
        state(&predicted).0 == state(&straight).0,
        "authority metadata differs"
    );
    for i in 0..10_000 {
        let id = StableId(i);
        let left = predicted.handle(id).unwrap();
        let right = straight.handle(id).unwrap();
        assert_eq!(left, right);
        assert_eq!(
            predicted.get(left).unwrap(),
            straight.get(right).unwrap(),
            "{id:?}"
        );
        let left_value = predicted.get(left).unwrap();
        let right_value = straight.get(right).unwrap();
        assert_eq!(
            left_value.position.0.map(f64::to_bits),
            right_value.position.0.map(f64::to_bits)
        );
        assert_eq!(
            left_value.velocity.0.map(f64::to_bits),
            right_value.velocity.0.map(f64::to_bits)
        );
        assert_eq!(
            predicted.authored_drift(left),
            straight.authored_drift(right),
            "{id:?}"
        );
    }
}

#[test]
fn movers_keep_their_authored_loops_after_a_stepped_hour() {
    let mut sim = exit_authority().expect("exit authority");
    let duration = 3_600 * u64::from(sim.tick_hz().get());
    for tick in 1..=duration {
        let outcome = sim.step(&[]).expect("exit tick");
        assert_eq!(
            (
                outcome.tick,
                outcome.simulated_entities,
                outcome.changed_positions
            ),
            (tick, 10_000, 1_500)
        );
    }
    for i in 0..1_500 {
        let handle = sim.handle(StableId(i)).unwrap();
        let (center, extent, period, offset) = sim.authored_drift(handle).expect("authored drift");
        let phase = u32::try_from((duration + u64::from(offset)) % u64::from(period)).unwrap();
        let angle = std::f64::consts::TAU * f64::from(phase) / f64::from(period);
        let position = sim.get(handle).unwrap().position.0;
        for (axis, waveform) in [angle.cos(), angle.sin(), angle.sin()]
            .into_iter()
            .enumerate()
        {
            let expected = center[axis] + extent[axis] * waveform;
            assert!(
                (position[axis] - expected).abs() < 1e-9,
                "mover {i}, axis {axis}"
            );
        }
    }
}
