use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::{
    Error, PhaseClock, PhaseType, Result, SceneType, Seconds,
    rule::params::{Context, Param, ParameterList, ReadContext, ReadOnly},
    runtime::{Requests, Transition, ValuePool, ValueRef},
};

use super::{Landing, Names};

fn lock_requests(requests: &Mutex<Requests>) -> MutexGuard<'_, Requests> {
    // A panicking rule is reported and the request scratch is reset before retry.
    requests.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Request a scene change to land after the tick's rules finish.
/// ```
/// use thrum::{Scene, Scenes};
/// #[derive(Scene)] struct Cleared(u32);
/// fn finish(mut scenes: Scenes) { scenes.go_to(Cleared(10)); }
/// ```
pub struct Scenes<'t>(&'t SceneRequests);

impl Scenes<'_> {
    /// Ask to enter another scene, carrying its typed value.
    /// ```
    /// use thrum::{Scene, Scenes};
    /// #[derive(Scene)] struct Cleared;
    /// fn finish(mut scenes: Scenes) -> bool { scenes.go_to(Cleared) }
    /// ```
    pub fn go_to<S: SceneType>(&mut self, scene: S) -> bool {
        let mut requests = lock_requests(&self.0.requests);
        if requests.next.is_some() || requests.error.is_some() {
            return false;
        }
        if let Err(error) = crate::runtime::check_value(&scene) {
            requests.error = Some(error);
            return false;
        }
        if let Some(index) = self.0.names.scene::<S>() {
            let scene = self
                .0
                .values
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .store(scene);
            requests.next = Some(Transition::Scene(index, scene));
            true
        } else {
            requests.error = Some(Error::refused(
                "T0305",
                std::any::type_name::<S>(),
                "the requested scene was never registered",
                "Add its description with Game::scene(..).",
            ));
            false
        }
    }

    /// Ask to leave and re-enter the current scene with the same value.
    /// ```
    /// fn repeat(mut scenes: thrum::Scenes) -> bool { scenes.restart() }
    /// ```
    pub fn restart(&mut self) -> bool {
        let mut requests = lock_requests(&self.0.requests);
        if requests.next.is_some() || requests.error.is_some() {
            return false;
        }
        if let Some(value) = &self.0.value {
            requests.next = Some(Transition::Scene(self.0.scene, Arc::clone(value)));
            true
        } else {
            false
        }
    }
}

#[derive(Default)]
#[doc(hidden)]
/// The SDK's prepared scene requests.
/// ```
/// fn rule(_: thrum::Scenes) {}
/// ```
pub struct SceneRequests {
    requests: Arc<Mutex<Requests>>,
    names: Arc<Names>,
    scene: usize,
    value: Option<ValueRef>,
    values: Arc<Mutex<ValuePool>>,
}

// SAFETY: This adapter only lends its own scratch and acquires no column loans.
#[expect(unsafe_code, reason = "trusted SDK parameter adapter")]
unsafe impl Param for Scenes<'_> {
    type State = SceneRequests;
    type Item<'t> = Scenes<'t>;
    fn describe(name: &str, list: &mut ParameterList) {
        list.writer::<SceneRequests>(name);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        state.requests = Arc::clone(&context.candidate.requests);
        state.names = Arc::clone(context.names);
        state.scene = context.candidate.state.scene;
        state.value = context
            .candidate
            .state
            .scene_value
            .as_ref()
            .map(|value| Arc::clone(&value.0));
        state.values = Arc::clone(&context.candidate.values);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Scenes<'t> {
        Scenes(state)
    }
}

/// Read the current phase and its clock.
/// ```
/// use thrum::{Phase, Seconds};
/// #[derive(Phase)] enum Stage { Play }
/// fn time(phase: Phase<Stage>) -> Seconds { phase.time() }
/// ```
pub struct Phase<'t, P: PhaseType>(&'t PhaseRequests<P>);

impl<P: PhaseType> Phase<'_, P> {
    /// Copy the phase's current value.
    ///
    /// # Panics
    /// The running scene has no phase of this type.
    /// ```
    /// use thrum::Phase;
    /// #[derive(Phase)] enum Stage { Play }
    /// fn current(phase: Phase<Stage>) -> Stage { phase.get() }
    /// ```
    pub fn get(&self) -> P {
        self.0
            .current
            .clone()
            .expect("the scene has a phase of this type")
    }

    /// Whether the phase is the named value.
    /// ```
    /// use thrum::Phase;
    /// #[derive(Phase)] enum Stage { Play }
    /// fn playing(phase: Phase<Stage>) -> bool { phase.is(Stage::Play) }
    /// ```
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the scene API takes phase enum values"
    )]
    pub fn is(&self, phase: P) -> bool {
        self.0.current.as_ref() == Some(&phase)
    }

    /// Elapsed time since the phase was entered, or the resumed elapsed time.
    /// ```
    /// use thrum::{Phase, Seconds};
    /// #[derive(Phase)] enum Stage { Play }
    /// fn time(phase: Phase<Stage>) -> Seconds { phase.time() }
    /// ```
    pub fn time(&self) -> Seconds {
        self.clock().time()
    }

    /// Capture the phase's elapsed and remaining time.
    /// ```
    /// use thrum::{Phase, PhaseClock};
    /// #[derive(Phase)] enum Stage { Play }
    /// fn clock(phase: Phase<Stage>) -> PhaseClock { phase.clock() }
    /// ```
    pub fn clock(&self) -> PhaseClock {
        self.0.clock
    }
}

/// Read the phase and request a phase change or a resume.
/// ```
/// use thrum::{Phase, PhaseMut};
/// #[derive(Phase)] enum Stage { Play, Done }
/// fn finish(mut phase: PhaseMut<Stage>) { phase.go_to(Stage::Done); }
/// ```
pub struct PhaseMut<'t, P: PhaseType>(Phase<'t, P>);

impl<'t, P: PhaseType> std::ops::Deref for PhaseMut<'t, P> {
    type Target = Phase<'t, P>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: PhaseType> PhaseMut<'_, P> {
    /// Request a phase change at the end of this tick.
    /// ```
    /// use thrum::{Phase, PhaseMut};
    /// #[derive(Phase)] enum Stage { Play }
    /// fn again(mut phase: PhaseMut<Stage>) -> bool { phase.go_to(Stage::Play) }
    /// ```
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the scene API takes phase enum values"
    )]
    pub fn go_to(&mut self, phase: P) -> bool {
        let state = self.0.0;
        if matches!(state.stage, Landing::LeavingScene) {
            return false;
        }
        let Some(index) = state.names.phase(state.scene, &phase) else {
            return false;
        };
        let mut requests = lock_requests(&state.requests);
        if requests.next.is_some() || requests.error.is_some() {
            return false;
        }
        requests.next = Some(Transition::Phase(index));
        true
    }

    /// Resume a phase and its clock from a scene-enter rule.
    /// ```
    /// use thrum::{Phase, PhaseClock, PhaseMut, seconds};
    /// #[derive(Phase)] enum Stage { Play }
    /// fn resume(mut phase: PhaseMut<Stage>) -> bool {
    ///     phase.resume(Stage::Play, PhaseClock::at(seconds(4)))
    /// }
    /// ```
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the scene API takes phase enum values"
    )]
    pub fn resume(&mut self, phase: P, clock: PhaseClock) -> bool {
        let state = self.0.0;
        if state.stage != Landing::EnteringScene {
            return false;
        }
        let Some(index) = state.names.phase(state.scene, &phase) else {
            return false;
        };
        let mut requests = lock_requests(&state.requests);
        if requests.resume.is_some() {
            return false;
        }
        requests.resume = Some((index, clock));
        true
    }
}

#[doc(hidden)]
/// The SDK's prepared phase parameter.
/// ```
/// #[derive(thrum::Phase)]
/// enum Stage { Play }
/// ```
pub struct PhaseRequests<P> {
    current: Option<P>,
    requests: Arc<Mutex<Requests>>,
    names: Arc<Names>,
    clock: PhaseClock,
    scene: usize,
    stage: Landing,
}

impl<P> Default for PhaseRequests<P> {
    fn default() -> Self {
        Self {
            current: None,
            requests: Arc::default(),
            names: Arc::default(),
            clock: PhaseClock::at(Seconds::ZERO),
            scene: 0,
            stage: Landing::Rules,
        }
    }
}

fn phase_read<P: PhaseType>(state: &mut PhaseRequests<P>, context: &ReadContext<'_>) {
    let current = &context.candidate.state;
    state.current = current
        .phase
        .and_then(|phase| context.names.phase_value(current.scene, phase))
        .cloned();
    state.requests = Arc::clone(&context.candidate.requests);
    state.names = Arc::clone(context.names);
    state.clock = current.clock(context.tick);
    state.scene = current.scene;
    state.stage = context.stage;
}

// SAFETY: This adapter only lends its own scratch and acquires no column loans.
#[expect(unsafe_code, reason = "trusted SDK parameter adapter")]
unsafe impl<P: PhaseType> Param for Phase<'_, P> {
    type State = PhaseRequests<P>;
    type Item<'t> = Phase<'t, P>;
    fn describe(_: &str, _: &mut ParameterList) {}
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        phase_read(state, context);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Phase<'t, P> {
        Phase(state)
    }
}
impl<P: PhaseType> ReadOnly for Phase<'_, P> {}

// SAFETY: This adapter only lends its own scratch and acquires no column loans.
#[expect(unsafe_code, reason = "trusted SDK parameter adapter")]
unsafe impl<P: PhaseType> Param for PhaseMut<'_, P> {
    type State = PhaseRequests<P>;
    type Item<'t> = PhaseMut<'t, P>;
    fn describe(name: &str, list: &mut ParameterList) {
        list.writer::<PhaseRequests<P>>(name);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        phase_read(state, context);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> PhaseMut<'t, P> {
        PhaseMut(Phase(state))
    }
    fn write(_: &mut Self::State, _: &mut Context<'_, '_>) -> Result {
        Ok(())
    }
}
