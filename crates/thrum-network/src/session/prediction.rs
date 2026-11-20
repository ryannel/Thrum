use std::collections::VecDeque;

use thrum_core::sim::InputFrame;

use super::{World, wire::SlotResult};

/// Maximum unconfirmed span of a predicted controller, in ticks.
pub const PREDICTION_TICKS: usize = 64;

/// The input available to a controller rule. Axes have passed wire conversion.
pub struct ControllerInput<'a, A> {
    /// Canonical normalized axes.
    pub axes: [f32; 3],
    /// Timed actions in authoring order. A held frame has none.
    pub actions: &'a [A],
}

/// A game's self-contained character rule and its read-only confirmed endpoint.
pub trait PredictedController<W: World> {
    /// Complete, independently owned state for one step of this controller.
    type State: Copy;
    /// Immutable constants of the declared rule.
    type Rules;

    /// Read the controller for a slot from the committed world.
    fn read(world: &W, slot: usize) -> Option<Self::State>;
    /// Apply the same deterministic rule the authority uses.
    fn step(
        state: Self::State,
        input: ControllerInput<'_, W::Action>,
        rules: &Self::Rules,
    ) -> Self::State;
    /// Compare the exact stored bits of the complete state.
    fn same_bits(first: Self::State, second: Self::State) -> bool;
}

/// The controller type of a client constructed without prediction.
pub enum NoPrediction {}

impl<W: World> PredictedController<W> for NoPrediction {
    type State = ();
    type Rules = ();

    fn read(_: &W, _: usize) -> Option<()> {
        None
    }

    fn step((): (), _: ControllerInput<'_, W::Action>, (): &()) {}

    fn same_bits((): (), (): ()) -> bool {
        true
    }
}

/// Adjacent controller endpoints for local presentation.
#[derive(Clone, Copy, Debug)]
pub struct ControllerView<S> {
    /// Tick of the newer endpoint.
    pub tick: u64,
    /// State one tick before the newer endpoint, or that endpoint at a baseline.
    pub previous: S,
    /// State at the newer endpoint.
    pub current: S,
    /// Fraction of travel from previous to current, sampled from playout.
    pub interpolation: f64,
    /// Whether these endpoints are speculative. A dropped branch uses confirmed endpoints.
    pub predicted: bool,
}

/// Evidence available at the confirmed tick that caused a correction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrectionCause {
    /// The authority held or replaced the locally authored frame.
    HeldFrame,
    /// The authoring clock left a gap between authored ticks.
    AuthoringGap,
    /// An admitted ordered command also acted on the controller.
    Command,
    /// The confirmed rule produced a different state with the same timed input.
    Rule,
}

/// Reconciliation at a confirmed tick, and its effect at the drawn horizon.
#[derive(Clone, Copy, Debug)]
pub struct Correction<S> {
    /// Confirmed tick which exposed the mismatch.
    pub tick: u64,
    /// Tick whose old and replacement predicted states are supplied.
    pub horizon: u64,
    /// Predicted state before restoring and replaying.
    pub before: S,
    /// Replacement state at the same horizon.
    pub after: S,
    /// Number of later ticks replayed.
    pub replayed: usize,
    /// Observed cause.
    pub cause: CorrectionCause,
}

/// Observation emitted synchronously during a prediction-enabled advance.
#[derive(Clone, Copy, Debug)]
pub enum PredictionEvent<S> {
    /// A committed tick for which a predicted endpoint was retained.
    Compared {
        /// Committed tick.
        tick: u64,
        /// Whether the host admitted the authored timed frame for this tick.
        admitted: bool,
        /// Whether the recorded state equalled the committed state bit for bit.
        equal: bool,
    },
    /// The branch was restored and replayed.
    Corrected(Correction<S>),
}

struct Entry<A, S> {
    tick: u64,
    frame: Option<InputFrame<A>>,
    axes: [f32; 3],
    state: Option<S>,
}

pub(super) struct Prediction<W: World, P: PredictedController<W>> {
    rules: P::Rules,
    entries: VecDeque<Entry<W::Action, P::State>>,
    confirmed: Option<ControllerView<P::State>>,
    confirmed_axes: [f32; 3],
    suspended: bool,
    live: bool,
}

impl<W: World, P: PredictedController<W>> Prediction<W, P> {
    pub(super) fn new(rules: P::Rules) -> Self {
        Self {
            rules,
            entries: VecDeque::with_capacity(PREDICTION_TICKS),
            confirmed: None,
            confirmed_axes: [0.0; 3],
            suspended: true,
            live: false,
        }
    }

    pub(super) fn suspend(&mut self, discard: bool) {
        self.suspended = true;
        self.live = false;
        if discard {
            self.entries.clear();
        }
    }

    pub(super) fn install(&mut self, tick: u64, state: Option<P::State>, axes: [f32; 3]) {
        self.confirmed = state.map(|state| ControllerView {
            tick,
            previous: state,
            current: state,
            interpolation: 1.0,
            predicted: false,
        });
        self.confirmed_axes = axes;
        self.entries.retain(|entry| entry.tick > tick);
        self.replay();
    }

    pub(super) fn resume(&mut self) {
        if self.suspended {
            self.suspended = false;
            self.replay();
        }
    }

    pub(super) fn view(&self, interpolation: f64) -> Option<ControllerView<P::State>> {
        let mut view = self.confirmed?;
        view.interpolation = interpolation;
        if self.live
            && let Some(entry) = self.entries.back()
            && let Some(state) = entry.state
        {
            view.previous = self
                .entries
                .iter()
                .rev()
                .nth(1)
                .and_then(|previous| previous.state)
                .unwrap_or(view.current);
            view.current = state;
            view.tick = entry.tick;
            view.predicted = true;
        }
        Some(view)
    }

    pub(super) fn author(&mut self, authored: InputFrame<W::Action>) {
        let target = authored.tick;
        let Some(last) = self
            .entries
            .back()
            .map(|entry| entry.tick)
            .or_else(|| self.confirmed.map(|state| state.tick))
        else {
            return;
        };
        if target <= last {
            return;
        }
        let first_retained = target.saturating_sub(PREDICTION_TICKS as u64 - 1);
        if first_retained > self.entries.front().map_or(last + 1, |entry| entry.tick) {
            self.live = false;
            while self
                .entries
                .front()
                .is_some_and(|entry| entry.tick < first_retained)
            {
                self.entries.pop_front();
            }
        }
        for tick in (last + 1).max(first_retained)..target {
            self.push(tick, None);
        }
        self.push(target, Some(authored));
    }

    fn push(&mut self, tick: u64, frame: Option<InputFrame<W::Action>>) {
        let previous = self.entries.back();
        let mut entry = Entry {
            tick,
            frame,
            axes: previous.map_or(self.confirmed_axes, |entry| entry.axes),
            state: None,
        };
        if self.live
            && let Some(state) = previous
                .and_then(|entry| entry.state)
                .or_else(|| self.confirmed.map(|state| state.current))
        {
            let held = entry.axes;
            Self::predict(&self.rules, &mut entry, state, held);
        }
        self.entries.push_back(entry);
    }

    fn predict(
        rules: &P::Rules,
        entry: &mut Entry<W::Action, P::State>,
        state: P::State,
        held: [f32; 3],
    ) -> P::State {
        let input = entry.frame.as_ref().map_or(
            ControllerInput {
                axes: held,
                actions: &[],
            },
            |frame| ControllerInput {
                axes: frame.axes,
                actions: &frame.actions,
            },
        );
        entry.axes = input.axes;
        let next = P::step(state, input, rules);
        entry.state = Some(next);
        next
    }

    fn replay(&mut self) {
        let Some(confirmed) = self.confirmed.filter(|confirmed| {
            !self.suspended
                && self
                    .entries
                    .front()
                    .is_none_or(|entry| entry.tick == confirmed.tick + 1)
        }) else {
            self.live = false;
            return;
        };
        let mut state = confirmed.current;
        let mut axes = self.confirmed_axes;
        for entry in &mut self.entries {
            state = Self::predict(&self.rules, entry, state, axes);
            axes = entry.axes;
        }
        self.live = true;
    }

    pub(super) fn confirm(
        &mut self,
        tick: u64,
        actual: Option<P::State>,
        axes: [f32; 3],
        result: SlotResult,
        observe: &mut impl FnMut(PredictionEvent<P::State>),
    ) {
        let Some(actual) = actual else {
            self.install(tick, None, axes);
            return;
        };
        let head = self.view(1.0).filter(|view| view.predicted);
        self.confirmed = Some(ControllerView {
            tick,
            previous: self.confirmed.map_or(actual, |view| view.current),
            current: actual,
            interpolation: 1.0,
            predicted: false,
        });
        self.confirmed_axes = axes;
        let entry = self.entries.pop_front_if(|entry| entry.tick == tick);
        let (Some(entry), Some(head)) = (entry, head) else {
            self.replay();
            return;
        };
        let predicted = entry.state.expect("live prediction has endpoints");
        let admitted = entry
            .frame
            .as_ref()
            .is_some_and(|frame| frame.origin.sequence == result.frame);
        let equal = P::same_bits(predicted, actual);
        observe(PredictionEvent::Compared {
            tick,
            admitted,
            equal,
        });
        if equal && entry.axes.map(f32::to_bits) == axes.map(f32::to_bits) {
            return;
        }
        self.replay();
        let after = self.view(1.0).expect("confirmed controller").current;
        if !equal || !P::same_bits(head.current, after) {
            let cause = if result.commands != 0 {
                CorrectionCause::Command
            } else if entry.frame.is_none() {
                CorrectionCause::AuthoringGap
            } else if !admitted {
                CorrectionCause::HeldFrame
            } else {
                CorrectionCause::Rule
            };
            observe(PredictionEvent::Corrected(Correction {
                tick,
                horizon: head.tick,
                before: head.current,
                after,
                replayed: self.entries.len(),
                cause,
            }));
        }
    }
}
