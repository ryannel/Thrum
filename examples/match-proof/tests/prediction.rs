//! Exact reconciliation on the conditioned link.

use thrum_core::{
    profile::{Digest, Number, StateBytes},
    sim::{InputFrame, RuleSetIdentity},
};
use thrum_network::{
    input_batch::{ActionView, ActionWire, InputCodecError},
    session::{
        Client, ControllerInput, Controls, Host, PredictedController, PredictionEvent,
        RosterEvents, SLOTS, Settings, World,
    },
};
use thrum_transport::{
    Profile,
    memory::{Conditioner, Memory},
};

#[test]
fn after_every_copy_is_lost_the_held_tick_is_corrected_once() {
    let run = match_proof::run_prediction("held", "local", 150, 64, 29_721)
        .expect("valid conditioned prediction fixture");
    assert_eq!(run["lost_copies"], 2);
    assert_eq!(run["delivered_copies"], 0);
    assert_eq!(run["correction"]["tick"], run["held_tick"]);
    assert_ne!(
        run["correction"]["before_bits"],
        run["correction"]["after_bits"]
    );
}

#[test]
fn a_refused_step_keeps_the_last_committed_controller_during_repair() {
    let report = match_proof::run_prediction("refused", "a0", 120, 128, 29_721)
        .expect("valid conditioned prediction fixture");
    assert_eq!(report["committed_before_fault"], report["view_tick"]);
    assert_eq!(report["world_bits"], report["view_bits"]);
}

#[derive(Default)]
struct PulseWorld {
    tick: u64,
    energy: [u64; SLOTS],
}

fn pulse(state: u64, controls: &ControllerInput<'_, u8>, gain: u64) -> u64 {
    // A separate game rule with canonical axes and a wire-normalized action.
    state
        + u64::from(Number::from_axis(controls.axes[0]).to_bits()) * gain
        + controls.actions.iter().copied().map(u64::from).sum::<u64>()
}

impl World for PulseWorld {
    type Action = u8;
    type Error = ();

    fn tick(&self) -> u64 {
        self.tick
    }
    fn identity(&self) -> RuleSetIdentity {
        RuleSetIdentity([17; 32])
    }
    fn step(&mut self, frames: &[InputFrame<u8>; SLOTS], _: RosterEvents) -> Result<(), ()> {
        self.tick += 1;
        for (energy, frame) in self.energy.iter_mut().zip(frames) {
            *energy = pulse(
                *energy,
                &ControllerInput {
                    axes: frame.axes,
                    actions: &frame.actions,
                },
                2,
            );
        }
        Ok(())
    }
    fn digest(&self) -> u64 {
        let mut digest = Digest::default();
        for word in [self.tick].into_iter().chain(self.energy) {
            digest.bytes(&word.to_le_bytes());
        }
        digest.finish()
    }
    fn image(&self, destination: &mut StateBytes) -> Result<(), ()> {
        for word in [self.tick].into_iter().chain(self.energy) {
            destination.append(&word.to_le_bytes());
        }
        Ok(())
    }
    fn restore(&mut self, tick: u64, image: &[u8]) -> Result<(), ()> {
        let (chunks, rest) = image.as_chunks::<8>();
        let [image_tick, energy @ ..] =
            <&[[u8; 8]; SLOTS + 1]>::try_from(chunks).map_err(|_| ())?;
        if u64::from_le_bytes(*image_tick) != tick || !rest.is_empty() {
            return Err(());
        }
        self.tick = tick;
        self.energy = energy.map(u64::from_le_bytes);
        Ok(())
    }
}

struct PulseWire;

impl ActionWire<u8> for PulseWire {
    fn encode(&self, action: &u8, bytes: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
        bytes[0] = *action & 1;
        Ok((17, 1))
    }
    fn decode(&self, view: ActionView<'_>) -> Result<u8, InputCodecError> {
        match (view.schema_id, view.payload) {
            (17, [action @ (0 | 1)]) => Ok(*action),
            _ => Err(InputCodecError::InvalidAction),
        }
    }
}

struct PulseController;

impl PredictedController<PulseWorld> for PulseController {
    type State = u64;
    type Rules = u64;

    fn read(world: &PulseWorld, slot: usize) -> Option<u64> {
        Some(world.energy[slot])
    }
    fn step(state: u64, controls: ControllerInput<'_, u8>, rules: &u64) -> u64 {
        pulse(state, &controls, *rules)
    }
    fn same_bits(first: u64, second: u64) -> bool {
        first == second
    }
}

#[test]
fn a_second_rule_set_draws_both_endpoints_with_the_public_playout_fraction() {
    let mut carrier = Memory::new(
        Profile::default(),
        Conditioner {
            delay_ms: 2,
            jitter_ms: 0,
            loss_ppm: 0,
            ..Conditioner::default()
        },
    )
    .expect("valid conditioned prediction fixture");
    let [local, remote] = carrier
        .connect()
        .expect("valid conditioned prediction fixture");
    let settings = Settings::default();
    let mut host = Host::new(
        PulseWorld::default(),
        PulseWire,
        settings,
        [4; 16],
        5,
        false,
        0,
    )
    .expect("valid conditioned prediction fixture");
    host.connect(remote, 0)
        .expect("valid conditioned prediction fixture");
    host.start(0);
    let mut client = Client::<_, _, PulseController>::with_controller(
        PulseWorld::default(),
        PulseWire,
        local,
        0,
        Some(5),
        Some(2),
    );
    let axes = [0.234_321, 0.0, 0.0];
    let delta = pulse(
        0,
        &ControllerInput {
            axes,
            actions: &[1],
        },
        2,
    );
    let mut drawn = 0;
    let mut admitted = 0;
    let mut fractional = 0;
    for tick in 1..=120 {
        let now = tick * u64::from(settings.tick_us);
        carrier.advance(now / 1_000);
        host.advance(now, tick);
        client
            .advance_with_prediction_events(
                now,
                u8::MAX,
                |_| Controls {
                    axes,
                    actions: vec![3],
                },
                |event| {
                    if let PredictionEvent::Compared {
                        tick,
                        admitted: true,
                        equal,
                    } = event
                    {
                        assert!(equal, "alternate deterministic rule disagreed at {tick}");
                        admitted += 1;
                    }
                },
            )
            .expect("valid conditioned prediction fixture");
        if tick > 30 {
            let now = now + u64::from(settings.tick_us) / 3;
            let view = client
                .controller(now)
                .expect("controller endpoint available");
            let fraction = client
                .playout(now)
                .expect("controller endpoint available")
                .position
                - f64::from(
                    u32::try_from(client.world().tick() - 1)
                        .expect("controller endpoint available"),
                );
            assert_eq!(
                view.interpolation.to_bits(),
                fraction.clamp(0.0, 1.0).to_bits()
            );
            assert!(view.predicted && view.tick > client.world().tick());
            assert_eq!(view.tick, client.last_frame().0);
            assert_eq!(view.current - view.previous, delta);
            fractional += usize::from(view.interpolation > 0.0 && view.interpolation < 1.0);
            drawn += 1;
        }
        while client.result().is_some() {}
    }
    assert!(admitted > 90 && drawn == 90 && fractional > 0);
}
