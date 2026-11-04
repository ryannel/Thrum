//! W1 reports only origins consumed by a successful authoritative tick.

use std::num::NonZeroU32;

use thrum_core::{
    ActorId, Origin, PeerId, StableId, WorldEpoch,
    sim::{
        Command, ComponentEntity, InputFrame, Ownership, Position, SimConfig, Simulation, Velocity,
    },
};

fn fixture() -> (Simulation, thrum_core::Handle, Ownership) {
    let binding = Ownership {
        peer: PeerId(7),
        connection_epoch: 3,
        actor: ActorId(11),
    };
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("nonzero fixture rate"),
            entity_capacity: 1,
            peer_capacity: 1,
            actions_per_frame: 1,
            movement_speed: 6.0,
        },
    )
    .expect("one-entity fixture");
    let handle = sim
        .seed(ComponentEntity {
            id: StableId(1),
            position: Position([0.0; 3]),
            velocity: Velocity([0.0; 3]),
            health: 100,
            ownership: Some(binding),
            visual: None,
        })
        .expect("seed owned actor");
    sim.register_peer(binding, handle)
        .expect("one peer registration");
    (sim, handle, binding)
}

fn frame(binding: Ownership, tick: u64, sequence: u64) -> InputFrame<thrum_core::sim::SimAction> {
    InputFrame {
        origin: Origin {
            peer: binding.peer,
            connection_epoch: binding.connection_epoch,
            sequence,
        },
        actor: binding.actor,
        tick,
        axes: [1.0, 0.0, 0.0],
        actions: Vec::new(),
    }
}

#[test]
fn failed_tick_keeps_pending_origin_until_exact_commit_then_does_not_repeat_it() {
    let (mut sim, handle, binding) = fixture();
    let input = frame(binding, 1, 9);
    let origin = input.origin;
    sim.admit_frame(input).expect("admit complete input");
    assert!(
        sim.step(&[Command::SetPosition(handle, Position([f64::NAN, 0.0, 0.0]),)])
            .is_err()
    );
    assert_eq!(sim.tick(), 0);
    let committed = sim.step(&[]).expect("retry same tick");
    assert_eq!(committed.tick, 1);
    assert_eq!(committed.committed_inputs, &[origin]);
    let idle = sim.step(&[]).expect("following idle tick");
    assert_eq!(idle.tick, 2);
    assert_eq!(idle.committed_inputs, []);
}

#[test]
fn same_tick_despawn_still_resolves_the_consumed_input_origin() {
    let (mut sim, handle, binding) = fixture();
    let input = frame(binding, 1, 1);
    let origin = input.origin;
    sim.admit_frame(input).expect("admit before retire");
    let committed = sim
        .step(&[Command::Despawn(handle)])
        .expect("one committed retirement tick");
    assert_eq!(committed.tick, 1);
    assert_eq!(committed.committed_inputs, &[origin]);
    assert!(sim.handle(StableId(1)).is_none());
}
