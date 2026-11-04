//! Bounded complete-controller speculation for the no-collision V1 fixture.
//! Confirmed state remains owned by the adjacent confirmed-client path.

use std::{mem::size_of, time::Instant};

use thrum_core::{
    Origin,
    sim::{ControllerInput, ControllerRule, ControllerState, SimAction},
};
use thrum_network::input_batch::{MAX_ACTIONS, canonical_axes};

const HISTORY: usize = 64;
const MAX_AUTHOR_GAP: usize = 10;
const MAX_REPLAY_STEPS: usize = 9;
const REPLAY_BUDGET_NS: u128 = 1_000_000;
const POSITION_MATCH_M: f64 = 0.002;
const VELOCITY_MATCH_MPS: f64 = 0.0001;

#[derive(Clone, Copy)]
struct AuthoredInput {
    origin: Origin,
    axes: [f32; 3],
    actions: [SimAction; MAX_ACTIONS],
    action_count: usize,
}

impl AuthoredInput {
    fn new(origin: Origin, axes: [f32; 3], actions: &[SimAction]) -> Result<Self, String> {
        if actions.len() > MAX_ACTIONS {
            return Err("prediction action capacity exceeded".to_owned());
        }
        let mut stored = [SimAction::Damage(0); MAX_ACTIONS];
        stored[..actions.len()].copy_from_slice(actions);
        Ok(Self {
            origin,
            axes: canonical_axes(axes).map_err(|error| format!("prediction axes: {error:?}"))?,
            actions: stored,
            action_count: actions.len(),
        })
    }

    fn as_step(&self) -> ControllerInput<'_> {
        ControllerInput::Frame {
            axes: self.axes,
            actions: &self.actions[..self.action_count],
        }
    }
}

#[derive(Clone, Copy)]
struct Record {
    tick: u64,
    input: Option<AuthoredInput>,
    state: ControllerState,
}

/// Separate speculative branch for one confirmed controller.
pub(super) struct PredictedPeer {
    rule: ControllerRule,
    records: Box<[Record]>,
    state: ControllerState,
    tick: u64,
    active: bool,
    matches: u64,
    corrections: u64,
    fallbacks: u64,
    replay_steps: u64,
    max_replay_steps: usize,
    max_correction_m: f64,
    resolved_origins: u64,
    missed_authored: u64,
    origin_mismatches: u64,
}

impl PredictedPeer {
    pub(super) fn new(state: ControllerState, rule: ControllerRule) -> Self {
        Self {
            rule,
            records: vec![
                Record {
                    tick: u64::MAX,
                    input: None,
                    state,
                };
                HISTORY
            ]
            .into_boxed_slice(),
            state,
            tick: 0,
            active: true,
            matches: 0,
            corrections: 0,
            fallbacks: 0,
            replay_steps: 0,
            max_replay_steps: 0,
            max_correction_m: 0.0,
            resolved_origins: 0,
            missed_authored: 0,
            origin_mismatches: 0,
        }
    }

    fn slot(tick: u64) -> usize {
        usize::try_from(tick % HISTORY as u64).expect("bounded history index")
    }

    pub(super) fn hold_to(&mut self, target: u64) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        if self.tick.checked_add(1) != Some(target) {
            return Err("prediction hold target is not contiguous".to_owned());
        }
        let next = self
            .rule
            .advance(self.state, ControllerInput::Hold)
            .map_err(|error| format!("prediction hold step: {error:?}"))?;
        self.records[Self::slot(target)] = Record {
            tick: target,
            input: None,
            state: next,
        };
        self.state = next;
        self.tick = target;
        Ok(())
    }

    pub(super) fn reset(&mut self, tick: u64, state: ControllerState) {
        self.records.fill(Record {
            tick: u64::MAX,
            input: None,
            state,
        });
        self.state = state;
        self.tick = tick;
        self.active = true;
    }

    /// Canonicalize and apply newly authored input at its future target tick.
    /// The current fixture extends its local horizon by one tick per call.
    pub(super) fn author(
        &mut self,
        target: u64,
        origin: Origin,
        axes: [f32; 3],
        actions: &[SimAction],
    ) -> Result<(), String> {
        if origin.peer != self.state.binding.peer
            || origin.connection_epoch != self.state.binding.connection_epoch
        {
            return Err("prediction input origin does not match controller".to_owned());
        }
        let input = AuthoredInput::new(origin, axes, actions)?;
        if !self.active {
            return Ok(());
        }
        let gap = target
            .checked_sub(self.tick)
            .filter(|gap| (1..=MAX_AUTHOR_GAP as u64).contains(gap))
            .ok_or("prediction target is stale or beyond history")?;
        if gap == 1 {
            let next = self
                .rule
                .advance(self.state, input.as_step())
                .map_err(|error| format!("prediction author step: {error:?}"))?;
            self.records[Self::slot(target)] = Record {
                tick: target,
                input: Some(input),
                state: next,
            };
            self.state = next;
            self.tick = target;
            return Ok(());
        }
        let mut candidate = self.state;
        let mut staged = [self.state; MAX_AUTHOR_GAP];
        for offset in 0..gap {
            let step = if offset + 1 == gap {
                input.as_step()
            } else {
                ControllerInput::Hold
            };
            candidate = self
                .rule
                .advance(candidate, step)
                .map_err(|error| format!("prediction author step: {error:?}"))?;
            staged[usize::try_from(offset).expect("bounded author gap")] = candidate;
        }
        for offset in 0..gap {
            let tick = self.tick + offset + 1;
            self.records[Self::slot(tick)] = Record {
                tick,
                input: (offset + 1 == gap).then_some(input),
                state: staged[usize::try_from(offset).expect("bounded author gap")],
            };
        }
        self.state = candidate;
        self.tick = target;
        Ok(())
    }

    /// Apply a complete authority group to the separate branch. Missing
    /// history, excessive replay or an incompatible epoch sheds speculation.
    pub(super) fn confirm(
        &mut self,
        tick: u64,
        confirmed: ControllerState,
        committed: Option<Origin>,
        frame_started: Instant,
    ) {
        if !self.active {
            self.state = confirmed;
            self.tick = tick;
            return;
        }
        let record = self.records[Self::slot(tick)];
        if record.tick != tick || self.tick < tick || record.state.binding != confirmed.binding {
            self.fallback(tick, confirmed);
            return;
        }
        if let Some(origin) = committed {
            if record.input.map(|input| input.origin) != Some(origin) {
                self.origin_mismatches += 1;
                self.fallback(tick, confirmed);
                return;
            }
            self.resolved_origins += 1;
        } else if record.input.is_some() {
            self.missed_authored += 1;
        }
        let delta = std::array::from_fn::<_, 3, _>(|axis| {
            record.state.position.0[axis] - confirmed.position.0[axis]
        });
        let distance_squared = delta.iter().map(|value| value * value).sum::<f64>();
        if distance_squared <= POSITION_MATCH_M * POSITION_MATCH_M
            && record.state.health == confirmed.health
            && record
                .state
                .held_axes
                .iter()
                .zip(confirmed.held_axes)
                .all(|(left, right)| left.to_bits() == right.to_bits())
            && record
                .state
                .velocity
                .0
                .iter()
                .zip(confirmed.velocity.0)
                .all(|(a, b)| (a - b).abs() <= VELOCITY_MATCH_MPS)
        {
            self.matches += 1;
            return;
        }
        self.corrections += 1;
        let distance = delta[0].hypot(delta[1]).hypot(delta[2]);
        self.max_correction_m = self.max_correction_m.max(distance);
        let Some(steps) = self
            .tick
            .checked_sub(tick)
            .and_then(|span| usize::try_from(span).ok())
        else {
            self.fallback(tick, confirmed);
            return;
        };
        if steps > MAX_REPLAY_STEPS {
            self.fallback(tick, confirmed);
            return;
        }
        if frame_started.elapsed().as_nanos() > REPLAY_BUDGET_NS {
            self.fallback(tick, confirmed);
            return;
        }
        let mut candidate = confirmed;
        let mut staged = [confirmed; MAX_REPLAY_STEPS];
        for (index, output) in staged.iter_mut().take(steps).enumerate() {
            let replay_tick = tick + u64::try_from(index).expect("bounded replay") + 1;
            let record = &self.records[Self::slot(replay_tick)];
            if record.tick != replay_tick {
                self.fallback(tick, confirmed);
                return;
            }
            let input = record
                .input
                .as_ref()
                .map_or(ControllerInput::Hold, AuthoredInput::as_step);
            let Ok(next) = self.rule.advance(candidate, input) else {
                self.fallback(tick, confirmed);
                return;
            };
            candidate = next;
            *output = next;
        }
        if frame_started.elapsed().as_nanos() > REPLAY_BUDGET_NS {
            self.fallback(tick, confirmed);
            return;
        }
        self.records[Self::slot(tick)].state = confirmed;
        for (index, state) in staged.into_iter().take(steps).enumerate() {
            let replay_tick = tick + u64::try_from(index).expect("bounded replay") + 1;
            self.records[Self::slot(replay_tick)].state = state;
        }
        self.state = candidate;
        self.max_replay_steps = self.max_replay_steps.max(steps);
        self.replay_steps += u64::try_from(steps).expect("bounded replay");
    }

    fn fallback(&mut self, tick: u64, confirmed: ControllerState) {
        self.state = confirmed;
        self.tick = tick;
        self.active = false;
        self.fallbacks += 1;
    }

    pub(super) fn report(&self) -> PredictionReport {
        PredictionReport {
            tick: self.tick,
            state: self.state,
            active: self.active,
            matches: self.matches,
            corrections: self.corrections,
            fallbacks: self.fallbacks,
            replay_steps: self.replay_steps,
            max_replay_steps: self.max_replay_steps,
            max_correction_m: self.max_correction_m,
            resolved_origins: self.resolved_origins,
            missed_authored: self.missed_authored,
            origin_mismatches: self.origin_mismatches,
            retained_bytes: size_of::<Self>() + self.records.len() * size_of::<Record>(),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct PredictionReport {
    pub(super) tick: u64,
    pub(super) state: ControllerState,
    pub(super) active: bool,
    pub(super) matches: u64,
    pub(super) corrections: u64,
    pub(super) fallbacks: u64,
    pub(super) replay_steps: u64,
    pub(super) max_replay_steps: usize,
    pub(super) max_correction_m: f64,
    pub(super) resolved_origins: u64,
    pub(super) missed_authored: u64,
    pub(super) origin_mismatches: u64,
    pub(super) retained_bytes: usize,
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use thrum_core::Origin;
    use thrum_core::sim::{ControllerInput, SimAction};
    use thrum_network::input_batch::{self, EncodeRecord};

    use super::{PredictedPeer, size_of};
    use crate::{conditioned::SceneActionWire, scene};

    fn origin(sequence: u64) -> Origin {
        Origin {
            peer: scene::LOCAL_PEER,
            connection_epoch: scene::binding(scene::LOCAL_PEER).connection_epoch,
            sequence,
        }
    }

    #[test]
    fn fractional_axis_prediction_uses_the_actual_wire_value() {
        let mut sim = scene::simulation().expect("V1 world");
        let peer = scene::LOCAL_PEER;
        let initial = sim.controller_state(peer).expect("initial controller");
        let mut predicted = PredictedPeer::new(initial, sim.controller_rule());
        let mut frame = scene::control_frame(peer, [0.5, 0.0, 0.0], 1, 1);
        frame.actions.push(SimAction::Impulse([1.25, 0.0, 0.0]));
        frame.actions.push(SimAction::Damage(7));
        predicted
            .author(1, frame.origin, frame.axes, &frame.actions)
            .expect("author fractional axis and actions");
        let mut bytes = [0u8; 128];
        let len = input_batch::encode(
            &[EncodeRecord {
                frame: &frame,
                actor_wire_id: frame.actor.0,
            }],
            &SceneActionWire,
            &mut bytes,
        )
        .expect("encoded fractional frame");
        let batch = input_batch::decode(&bytes[..len]).expect("decoded packet");
        let decoded = batch
            .records()
            .next()
            .expect("one record")
            .decode_frame(scene::binding(peer), frame.actor.0, &SceneActionWire)
            .expect("game action schema");
        sim.admit_frame(decoded).expect("admit decoded frame");
        sim.step(&[]).expect("authoritative tick");
        predicted.confirm(
            1,
            sim.controller_state(peer).expect("committed controller"),
            Some(frame.origin),
            Instant::now(),
        );
        let report = predicted.report();
        assert_eq!(report.matches, 1);
        assert_eq!(report.corrections, 0);
        assert_eq!(report.fallbacks, 0);
        assert_eq!(report.resolved_origins, 1);
        assert_eq!(report.state.health, 93);
    }

    #[test]
    fn loss_replays_later_authored_input_and_bad_input_does_not_change_branch() {
        let sim = scene::simulation().expect("V1 world");
        let initial = sim
            .controller_state(scene::LOCAL_PEER)
            .expect("initial controller");
        let rule = sim.controller_rule();
        let mut predicted = PredictedPeer::new(initial, rule);
        assert!(
            predicted
                .author(1, origin(1), [f32::NAN, 0.0, 0.0], &[])
                .is_err()
        );
        assert!(
            predicted
                .author(
                    1,
                    origin(1),
                    [0.0; 3],
                    &[SimAction::Impulse([f64::NAN, 0.0, 0.0])]
                )
                .is_err()
        );
        assert_eq!(predicted.report().tick, 0);
        assert_eq!(predicted.report().state, initial);
        predicted
            .author(1, origin(1), [1.0, 0.0, 0.0], &[])
            .expect("first local input");
        predicted
            .author(2, origin(2), [1.0, 0.0, 0.0], &[])
            .expect("next local input");
        let missed_first = rule
            .advance(initial, ControllerInput::Hold)
            .expect("held authority");
        predicted.confirm(1, missed_first, None, Instant::now());
        let report = predicted.report();
        assert_eq!(report.corrections, 1);
        assert_eq!(report.missed_authored, 1);
        assert_eq!(report.replay_steps, 1);
        assert_eq!(report.max_replay_steps, 1);
        assert!((report.state.position.0[0] - initial.position.0[0] - 0.1).abs() < 1e-9);
        let committed_second = rule
            .advance(
                missed_first,
                ControllerInput::Frame {
                    axes: [1.0, 0.0, 0.0],
                    actions: &[],
                },
            )
            .expect("later committed input");
        predicted.confirm(2, committed_second, Some(origin(2)), Instant::now());
        assert_eq!(predicted.report().matches, 1);
        assert_eq!(predicted.report().resolved_origins, 1);
        assert_eq!(predicted.report().fallbacks, 0);
    }

    #[test]
    fn incompatible_binding_discards_speculation_and_history_stays_bounded() {
        let sim = scene::simulation().expect("V1 world");
        let initial = sim
            .controller_state(scene::LOCAL_PEER)
            .expect("initial controller");
        let mut predicted = PredictedPeer::new(initial, sim.controller_rule());
        assert!(size_of::<PredictedPeer>() + 64 * size_of::<super::Record>() < 256 * 1024);
        predicted
            .author(1, origin(1), [1.0, 0.0, 0.0], &[])
            .expect("local input");
        let mut replacement = initial;
        replacement.binding.connection_epoch += 1;
        predicted.confirm(1, replacement, None, Instant::now());
        let report = predicted.report();
        assert!(!report.active);
        assert_eq!(report.fallbacks, 1);
        assert_eq!(report.state, replacement);
        assert_eq!(report.tick, 1);
    }

    #[test]
    fn shared_deadline_fallback_keeps_following_confirmed_state_visible() {
        let sim = scene::simulation().expect("V1 world");
        let initial = sim
            .controller_state(scene::LOCAL_PEER)
            .expect("initial controller");
        let rule = sim.controller_rule();
        let mut predicted = PredictedPeer::new(initial, rule);
        predicted
            .author(1, origin(1), [1.0, 0.0, 0.0], &[])
            .expect("local input");
        let held = rule
            .advance(initial, ControllerInput::Hold)
            .expect("missed input");
        let exhausted = Instant::now()
            .checked_sub(Duration::from_millis(2))
            .expect("monotonic clock is older than two milliseconds");
        predicted.confirm(1, held, None, exhausted);
        assert!(!predicted.report().active);
        assert_eq!(predicted.report().fallbacks, 1);
        let next = rule
            .advance(held, ControllerInput::Hold)
            .expect("next confirmed tick");
        predicted.confirm(2, next, None, Instant::now());
        assert_eq!(predicted.report().state, next);
        assert_eq!(predicted.report().tick, 2);
        assert_eq!(predicted.report().fallbacks, 1);
    }

    #[test]
    fn wrong_committed_origin_drops_the_speculative_branch() {
        let sim = scene::simulation().expect("V1 world");
        let initial = sim
            .controller_state(scene::LOCAL_PEER)
            .expect("initial controller");
        let mut predicted = PredictedPeer::new(initial, sim.controller_rule());
        predicted
            .author(1, origin(1), [1.0, 0.0, 0.0], &[])
            .expect("authored first input");
        let result = predicted.report().state;
        predicted.confirm(1, result, Some(origin(2)), Instant::now());
        assert!(!predicted.report().active);
        assert_eq!(predicted.report().origin_mismatches, 1);
        assert_eq!(predicted.report().resolved_origins, 0);
    }
}
