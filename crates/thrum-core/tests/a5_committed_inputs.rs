//! W1 reports only origins consumed by a successful authoritative tick.

use std::num::NonZeroU32;

use thrum_core::{
    ActorId, Origin, PeerId, StableId, WorldEpoch,
    sim::{
        Command, ComponentEntity, ControllerInput, ControllerStepError, FrameError, InputFrame,
        Ownership, Position, SimAction, SimConfig, Simulation, Velocity,
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

fn frame(binding: Ownership, tick: u64, sequence: u64) -> InputFrame<SimAction> {
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
    let controller = sim
        .controller_state(binding.peer)
        .expect("committed control");
    assert_eq!(controller.binding, binding);
    assert_eq!(controller.held_axes, [1.0, 0.0, 0.0]);
    assert!((controller.position.0[0] - 0.1).abs() < 1e-9);
    assert!(
        controller.position.0[1..]
            .iter()
            .all(|value| value.abs() < 1e-9)
    );
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
    assert!(sim.controller_state(binding.peer).is_err());
}

#[test]
fn pure_controller_rule_matches_committed_input_actions_and_held_ticks() {
    let (mut sim, _, binding) = fixture();
    let rule = sim.controller_rule();
    let mut predicted = sim.controller_state(binding.peer).expect("initial control");
    let mut impulse = frame(binding, 1, 1);
    impulse.axes = [0.5, 0.0, 0.0];
    impulse.actions.push(SimAction::Impulse([1.0, 0.0, 0.0]));
    predicted = rule
        .advance(
            predicted,
            ControllerInput::Frame {
                axes: impulse.axes,
                actions: &impulse.actions,
            },
        )
        .expect("pure first tick");
    sim.admit_frame(impulse).expect("admit impulse");
    sim.step(&[]).expect("commit impulse");
    assert_eq!(
        predicted,
        sim.controller_state(binding.peer).expect("tick one")
    );

    predicted = rule
        .advance(predicted, ControllerInput::Hold)
        .expect("pure held tick");
    sim.step(&[]).expect("commit held tick");
    assert_eq!(
        predicted,
        sim.controller_state(binding.peer).expect("tick two")
    );

    let mut damage = frame(binding, 3, 2);
    damage.axes = [-0.25, 0.0, 0.0];
    damage.actions.push(SimAction::Damage(17));
    predicted = rule
        .advance(
            predicted,
            ControllerInput::Frame {
                axes: damage.axes,
                actions: &damage.actions,
            },
        )
        .expect("pure damage tick");
    sim.admit_frame(damage).expect("admit damage");
    sim.step(&[]).expect("commit damage");
    assert_eq!(
        predicted,
        sim.controller_state(binding.peer).expect("tick three")
    );
    assert_eq!(predicted.health, 83);

    assert_eq!(
        rule.advance(
            predicted,
            ControllerInput::Frame {
                axes: [2.0, 0.0, 0.0],
                actions: &[],
            }
        ),
        Err(ControllerStepError::InvalidInput(FrameError::InvalidAxis))
    );
    assert_eq!(
        rule.advance(
            predicted,
            ControllerInput::Frame {
                axes: [0.0; 3],
                actions: &[SimAction::Impulse([f64::NAN, 0.0, 0.0])],
            }
        ),
        Err(ControllerStepError::InvalidInput(FrameError::InvalidAction))
    );
    assert_eq!(
        predicted,
        sim.controller_state(binding.peer).expect("unchanged")
    );
}
