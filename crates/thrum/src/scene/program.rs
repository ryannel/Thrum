use std::{
    any::TypeId,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};

use thrum_core::sim::{
    Access, AccessKey, AccessMode, ComponentRegistry, EditWriter, Executor, GameRuntime,
    RegisteredSystem, Stage, StructuralPermit, SystemId, SystemSpec, WorldRead, WorldTxn,
};

use crate::{
    Error, Result, Seconds, Tick, TickNumber,
    entity::{Changes, contains_entities},
    rule::{
        Invocation, ReadContextParts, Rule, RuleStreams,
        access::{Bindings, Column, Signature},
        params::Context,
    },
    runtime::{
        Authority, Candidate, Operation, SceneValue, State, Transition, collect_entities,
        despawn_all,
    },
};

use super::{Definition, Duration, Expiry, Landing, Names};

const STRUCTURE: u64 = 1;

pub(crate) struct Program {
    names: Arc<Names>,
    slots: Vec<Slot>,
    scenes: Vec<ScenePlan>,
    pub(crate) columns: Vec<Column>,
    pub(crate) streams: RuleStreams,
    rate: u32,
}

struct ScenePlan {
    resources: Vec<crate::runtime::ValueRef>,
    first_phase: Option<usize>,
    signatures: Vec<(&'static str, Signature)>,
}

struct Slot {
    step: Mutex<Step>,
    columns: Vec<(Column, AccessMode)>,
    bindings: Bindings,
    permit: Option<StructuralPermit>,
    scratch: Mutex<Changes>,
}

enum Step {
    Manual,
    Main(usize, Option<usize>, Rule),
    Expire(usize, usize, Option<usize>),
    ExpiryRule(usize, usize, Rule),
    Choose,
    Exit(usize, Option<usize>, Rule),
    Install,
    EnterScene(usize, Rule),
    SelectPhase,
    EnterPhase(usize, usize, Rule),
    Length(usize, usize, Duration),
    Finish,
}

impl Step {
    fn signatures(&self) -> impl Iterator<Item = (&'static str, &Signature)> {
        let (rule, duration) = match self {
            Self::Main(_, _, rule)
            | Self::ExpiryRule(_, _, rule)
            | Self::Exit(_, _, rule)
            | Self::EnterScene(_, rule)
            | Self::EnterPhase(_, _, rule) => (Some(rule), None),
            Self::Length(_, _, Duration::Read(read)) => (None, Some((read.name, &read.signature))),
            _ => (None, None),
        };
        let base = rule.map(|rule| (rule.invoke.name, &rule.invoke.signature));
        let conditions = rule.into_iter().flat_map(|rule| {
            rule.conditions
                .iter()
                .map(|condition| (condition.name, &condition.signature))
        });
        base.into_iter().chain(conditions).chain(duration)
    }

    fn identify(&mut self, slot: u32, streams: &mut RuleStreams) {
        let rule = match self {
            Self::Main(_, _, rule)
            | Self::ExpiryRule(_, _, rule)
            | Self::Exit(_, _, rule)
            | Self::EnterScene(_, rule)
            | Self::EnterPhase(_, _, rule) => rule,
            Self::Length(_, _, Duration::Read(read)) => {
                read.identify_scheduled(u64::from(slot), streams);
                return;
            }
            _ => return,
        };
        rule.invoke.identify_scheduled(u64::from(slot), streams);
        for (index, condition) in (1_u64..).zip(&rule.conditions) {
            condition.identify((index << 32) | u64::from(slot), streams);
        }
    }

    fn scene(&self) -> Option<usize> {
        match self {
            Self::Main(scene, ..)
            | Self::Expire(scene, ..)
            | Self::ExpiryRule(scene, ..)
            | Self::Exit(scene, ..)
            | Self::EnterScene(scene, ..)
            | Self::EnterPhase(scene, ..)
            | Self::Length(scene, ..) => Some(*scene),
            _ => None,
        }
    }
}

impl Program {
    pub(crate) fn compile(
        names: Names,
        definitions: Vec<Definition>,
        mut columns: Vec<Column>,
        global: &[TypeId],
        rate: u32,
    ) -> Result<(Self, ComponentRegistry, Vec<RegisteredSystem<Authority>>)> {
        let mut scenes = Vec::new();
        let mut main = vec![Step::Manual];
        let mut expiries = Vec::new();
        let mut exits = Vec::new();
        let mut enters = Vec::new();
        let mut phases = Vec::new();
        for (scene, definition) in definitions.into_iter().enumerate() {
            for value in definition
                .resources
                .iter()
                .chain(&names.0[scene].phase_values)
            {
                crate::runtime::check_value(value.as_ref())?;
            }
            let mut phase_exits = Vec::new();
            main.extend(
                definition
                    .main
                    .into_iter()
                    .map(|(phase, rule)| Step::Main(scene, phase, rule)),
            );
            enters.extend(
                definition
                    .enter
                    .into_iter()
                    .map(|rule| Step::EnterScene(scene, rule)),
            );
            for (phase, definition) in definition.phases.into_iter().enumerate() {
                let next = match &definition.expiry {
                    Some(Expiry::Phase(next)) => Some(*next),
                    _ => None,
                };
                if definition.duration.is_some() {
                    expiries.push(Step::Expire(scene, phase, next));
                }
                if let Some(Expiry::Run(rules)) = definition.expiry {
                    expiries.extend(
                        rules
                            .into_iter()
                            .map(|rule| Step::ExpiryRule(scene, phase, rule)),
                    );
                }
                phase_exits.extend(
                    definition
                        .exit
                        .into_iter()
                        .map(|rule| Step::Exit(scene, Some(phase), rule)),
                );
                phases.extend(
                    definition
                        .enter
                        .into_iter()
                        .map(|rule| Step::EnterPhase(scene, phase, rule)),
                );
                if let Some(length) = definition.duration {
                    phases.push(Step::Length(scene, phase, length));
                }
            }
            exits.extend(phase_exits);
            exits.extend(
                definition
                    .exit
                    .into_iter()
                    .map(|rule| Step::Exit(scene, None, rule)),
            );
            scenes.push(ScenePlan {
                resources: definition.resources,
                first_phase: definition.first_phase,
                signatures: Vec::new(),
            });
        }
        main.extend(expiries);
        main.push(Step::Choose);
        main.extend(exits);
        main.push(Step::Install);
        main.extend(enters);
        main.push(Step::SelectPhase);
        main.extend(phases);
        main.push(Step::Finish);

        for step in &main {
            if let Some(scene) = step.scene() {
                let mut providers = global.to_vec();
                providers.extend(scenes[scene].resources.iter().map(|value| value.id()));
                for (name, signature) in step.signatures() {
                    signature.validate(name, &providers, Some(names.0[scene].id))?;
                    for (column, _) in signature.columns() {
                        if !columns.iter().any(|known| known.key == column.key) {
                            columns.push(column);
                        }
                    }
                    if signature.checks_entity_overlap() {
                        scenes[scene].signatures.push((name, signature.clone()));
                    }
                }
            }
        }
        let mut registry = ComponentRegistry::default();
        for &column in &columns {
            column.register(&mut registry)?;
        }

        let mut slots = Vec::new();
        let mut systems = Vec::new();
        let mut streams = RuleStreams::default();
        for (index, mut step) in main.into_iter().enumerate() {
            let id = u32::try_from(index).map_err(|_| Error::invalid("scene", "too many rules"))?;
            step.identify(id, &mut streams);
            let writes_structure = matches!(step, Step::Manual | Step::Install)
                || step
                    .signatures()
                    .any(|(_, signature)| contains_entities(signature));
            let mut accesses = Vec::new();
            if writes_structure {
                accesses.extend(
                    columns
                        .iter()
                        .copied()
                        .map(|column| (column, AccessMode::Write)),
                );
            } else {
                for (_, signature) in step.signatures() {
                    for (column, mode) in signature.columns() {
                        if let Some((_, current)) = accesses
                            .iter_mut()
                            .find(|(known, _)| known.key == column.key)
                        {
                            if mode == AccessMode::Write {
                                *current = mode;
                            }
                        } else {
                            accesses.push((column, mode));
                        }
                    }
                }
            }
            let mut declared = Signature::accesses(&accesses);
            if writes_structure {
                declared.push(Access {
                    key: AccessKey::External(STRUCTURE),
                    mode: AccessMode::Write,
                });
            }
            systems.push(RegisteredSystem::direct(
                SystemSpec {
                    id: SystemId(id),
                    stage: Stage::Simulate,
                    accesses: declared,
                    after: id.checked_sub(1).map(SystemId).into_iter().collect(),
                },
                dispatch,
            ));
            slots.push(Slot {
                step: Mutex::new(step),
                columns: accesses,
                bindings: Bindings::default(),
                permit: None,
                scratch: Mutex::new(Changes::default()),
            });
        }
        Ok((
            Self {
                names: Arc::new(names),
                slots,
                scenes,
                columns,
                streams,
                rate,
            },
            registry,
            systems,
        ))
    }

    pub(crate) fn bind(&mut self, runtime: &GameRuntime<Authority>) {
        for (index, slot) in (0_u32..).zip(&mut self.slots) {
            let id = SystemId(index);
            slot.bindings = Bindings::new(runtime, id, &slot.columns);
            slot.permit = runtime.structural_permit(id, STRUCTURE).ok();
        }
    }

    pub(crate) fn validate_manual(
        &self,
        signature: &Signature,
        rule: &str,
        state: &State,
    ) -> Result {
        signature.validate(
            rule,
            &state.resources.types(),
            state.scene_value.as_ref().map(|value| value.0.id()),
        )?;
        for (column, _) in signature.columns() {
            if !self.columns.iter().any(|known| known.key == column.key) {
                return Err(Error::refused(
                    "T0301",
                    rule,
                    format!("component {} is not registered", column.name),
                    "Register it with Game::component::<T>() before starting the harness.",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn names(&self) -> &Names {
        &self.names
    }

    pub(crate) fn bindings(&self) -> &Bindings {
        &self.slots[0].bindings
    }

    pub(crate) fn rate(&self) -> u32 {
        self.rate
    }

    fn invoke<O>(
        &self,
        invocation: &mut Invocation<O>,
        stage: Landing,
        call: &mut Call<'_, '_, '_>,
    ) -> Result<O> {
        let world = call.world.read();
        if invocation.signature.needs_entities() && !call.candidate.entities_valid {
            collect_entities(&world, &mut call.candidate.entities);
            call.candidate.entities_valid = true;
        }
        if call.world.system_id().0 == 0 && invocation.signature.checks_entity_overlap() {
            for &(entity, handle) in &call.candidate.entities {
                invocation
                    .signature
                    .validate_entity(invocation.name, entity, |key| {
                        world.has_component(handle, key)
                    })
                    .map_err(|error| rule_failure(invocation.name, call.tick, &error))?;
            }
        }
        let tick = Tick {
            number: TickNumber(call.tick),
            rate: self.rate,
            scene_entered: TickNumber(call.candidate.state.scene_entered),
            phase_time: Seconds::ZERO,
        };
        let tick = Tick {
            phase_time: if call.candidate.state.phase.is_some() {
                call.candidate.state.clock(tick).time()
            } else {
                Seconds::ZERO
            },
            ..tick
        };
        let entities = std::mem::take(&mut call.candidate.entities);
        let mut scratch = call.slot.scratch.lock().expect("rule scratch");
        scratch.reset();
        let parts = ReadContextParts {
            entities: &entities,
            executor: call.executor,
            tick,
            seed: call.candidate.state.seed,
            stream: invocation.stream,
            stage,
            names: &self.names,
        };
        let mut context = Context {
            world: call.world,
            bindings: &call.slot.bindings,
            candidate: call.candidate,
            columns: &self.columns,
            changes: &mut scratch,
            name: invocation.name,
        };
        let result = catch_unwind(AssertUnwindSafe(|| invocation.call(&mut context, &parts)));
        call.candidate.entities = entities;
        match result {
            Ok(Ok(output)) => {
                if let Some(error) = call
                    .candidate
                    .requests
                    .lock()
                    .expect("requests")
                    .error
                    .take()
                {
                    return Err(rule_failure(invocation.name, call.tick, &error));
                }
                if let Some(permit) = &call.slot.permit {
                    scratch.submit(
                        &call.world.read(),
                        &call.slot.bindings,
                        permit,
                        call.candidate,
                        call.edits,
                    );
                }
                Ok(output)
            }
            Ok(Err(error)) => Err(rule_failure(invocation.name, call.tick, &error)),
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("non-text panic");
                Err(Error::refused(
                    "T0307",
                    format!("rule {} at tick {}", invocation.name, call.tick),
                    format!("panicked: {message}"),
                    "Correct the rule and retry the tick.",
                ))
            }
        }
    }

    fn rule(
        &self,
        rule: &mut Rule,
        stage: Landing,
        entered: u64,
        call: &mut Call<'_, '_, '_>,
    ) -> Result {
        if let Some(interval) = rule.cadence {
            let elapsed = call.tick.saturating_sub(entered);
            if elapsed == 0 || !elapsed.is_multiple_of(interval.ticks(self.rate).max(1)) {
                return Ok(());
            }
        }
        for condition in &rule.conditions {
            if !condition.evaluate(call.candidate.attempt, |invocation| {
                self.invoke(invocation, stage, call)
            })? {
                return Ok(());
            }
        }
        self.invoke(&mut rule.invoke, stage, call)
    }

    fn install(&self, authority: &Authority, call: &mut Call<'_, '_, '_>) -> Result {
        let candidate = &mut *call.candidate;
        match candidate.transition.as_ref() {
            Some(Transition::Scene(index, value)) => {
                crate::runtime::check_value(value.as_ref())?;
                if matches!(candidate.state.held, Some(Transition::Phase(_))) {
                    candidate.state.held = None;
                }
                if !matches!(
                    *authority.operation.lock().expect("operation"),
                    Operation::Start(..)
                ) {
                    despawn_all(
                        call.slot
                            .permit
                            .as_ref()
                            .expect("scene can retire entities"),
                        call.edits,
                        &call.world.read(),
                    );
                }
                candidate.entities.clear();
                candidate.entities_valid = true;
                candidate.state.scene = *index;
                candidate.state.scene_value = Some(SceneValue(Arc::clone(value)));
                candidate.state.scene_entered = call.tick;
                candidate.state.resources.remove_scene();
                for resource in &self.scenes[*index].resources {
                    candidate.state.resources.install(resource.as_ref(), true);
                }
                set_phase(candidate, None, call.tick);
            }
            Some(Transition::Phase(phase)) => {
                set_phase(candidate, Some(*phase), call.tick);
            }
            None => {}
        }
        Ok(())
    }

    fn execute(
        &self,
        authority: &Authority,
        step: &mut Step,
        call: &mut Call<'_, '_, '_>,
    ) -> Result {
        let scene = call.candidate.state.scene;
        let phase = call.candidate.state.phase;
        match step {
            Step::Manual => {
                let mut operation = authority.operation.lock().expect("operation");
                if let Operation::Manual(invocation) = &mut *operation {
                    self.invoke(invocation, Landing::Rules, call)?;
                }
            }
            Step::Main(when_scene, when_phase, rule)
                if *when_scene == scene && (when_phase.is_none() || *when_phase == phase) =>
            {
                if matches!(
                    *authority.operation.lock().expect("operation"),
                    Operation::Tick
                ) {
                    let entered = if when_phase.is_some() {
                        call.candidate.state.phase_entered
                    } else {
                        call.candidate.state.scene_entered
                    };
                    self.rule(rule, Landing::Rules, entered, call)?;
                }
            }
            Step::Expire(when_scene, when_phase, next)
                if *when_scene == scene && Some(*when_phase) == phase =>
            {
                if matches!(
                    *authority.operation.lock().expect("operation"),
                    Operation::Tick
                ) && let Some(end) = call.candidate.state.phase_end
                    && end <= call.tick
                {
                    let mut requests = call.candidate.requests.lock().expect("requests");
                    if requests.next.is_none()
                        && let Some(next) = next
                    {
                        requests.next = Some(Transition::Phase(*next));
                    }
                }
            }
            Step::ExpiryRule(when_scene, when_phase, rule)
                if *when_scene == scene && Some(*when_phase) == phase =>
            {
                if matches!(
                    *authority.operation.lock().expect("operation"),
                    Operation::Tick
                ) && !call.candidate.state.phase_expired
                    && call
                        .candidate
                        .state
                        .phase_end
                        .is_some_and(|end| end <= call.tick)
                {
                    self.rule(
                        rule,
                        Landing::Rules,
                        call.candidate.state.phase_entered,
                        call,
                    )?;
                }
            }
            Step::Choose => {
                let mut requests = call.candidate.requests.lock().expect("requests");
                let operation = authority.operation.lock().expect("operation");
                call.candidate.transition = match &*operation {
                    Operation::Start(index, value) => {
                        Some(Transition::Scene(*index, Arc::clone(value)))
                    }
                    Operation::Transition(transition) => Some(transition.clone()),
                    Operation::Tick | Operation::Manual(_) => requests.next.take(),
                };
                if matches!(*operation, Operation::Tick)
                    && call
                        .candidate
                        .state
                        .phase_end
                        .is_some_and(|end| end <= call.tick)
                {
                    call.candidate.state.phase_expired = true;
                    call.candidate.state.phase_end = None;
                }
            }
            Step::Exit(when_scene, when_phase, rule) if *when_scene == scene => {
                let stage = match (&call.candidate.transition, *when_phase) {
                    (Some(Transition::Scene(..)), when) if when.is_none() || when == phase => {
                        Some(Landing::LeavingScene)
                    }
                    (Some(Transition::Phase(_)), Some(when)) if Some(when) == phase => {
                        Some(Landing::LeavingPhase)
                    }
                    _ => None,
                };
                if let Some(stage) = stage
                    && !matches!(
                        *authority.operation.lock().expect("operation"),
                        Operation::Start(..)
                    )
                {
                    self.rule(rule, stage, call.tick, call)?;
                }
            }
            Step::Install => self.install(authority, call)?,
            Step::EnterScene(when, rule)
                if *when == scene
                    && matches!(call.candidate.transition, Some(Transition::Scene(..))) =>
            {
                self.rule(rule, Landing::EnteringScene, call.tick, call)?;
            }
            Step::SelectPhase
                if matches!(call.candidate.transition, Some(Transition::Scene(..))) =>
            {
                let resume = call
                    .candidate
                    .requests
                    .lock()
                    .expect("requests")
                    .resume
                    .take();
                if let Some((phase, clock)) = resume {
                    set_phase(call.candidate, Some(phase), call.tick);
                    call.candidate.resumed = true;
                    call.candidate.state.phase_offset = clock.elapsed;
                    call.candidate.state.phase_end = clock
                        .remaining
                        .map(|left| call.tick.saturating_add(left.ticks(self.rate)));
                    call.candidate.state.phase_expired = clock.expired;
                } else {
                    set_phase(call.candidate, self.scenes[scene].first_phase, call.tick);
                }
            }
            Step::EnterPhase(when_scene, when_phase, rule)
                if Self::entering(call.candidate, *when_scene, *when_phase) =>
            {
                self.rule(rule, Landing::EnteringPhase, call.tick, call)?;
            }
            Step::Length(when_scene, when_phase, length)
                if Self::entering(call.candidate, *when_scene, *when_phase) =>
            {
                let time = match length {
                    Duration::Fixed(seconds) => *seconds,
                    Duration::Read(read) => self.invoke(read, Landing::EnteringPhase, call)?,
                };
                call.candidate.state.phase_end =
                    Some(call.tick.saturating_add(time.ticks(self.rate)));
            }
            Step::Finish => {
                let held = call
                    .candidate
                    .requests
                    .lock()
                    .expect("requests")
                    .next
                    .take();
                if call.candidate.state.held.is_none() {
                    call.candidate.state.held = held;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn entering(candidate: &Candidate, scene: usize, phase: usize) -> bool {
        !candidate.resumed
            && candidate.transition.is_some()
            && candidate.state.scene == scene
            && candidate.state.phase == Some(phase)
    }

    pub(crate) fn validate_queries(
        &self,
        world: &WorldRead<'_>,
        scene: usize,
        changed: &[crate::Entity],
        tick: u64,
    ) -> Result {
        let signatures = &self.scenes[scene].signatures;
        if signatures.is_empty() {
            return Ok(());
        }
        for &entity in changed {
            if let Some(handle) = world.handle(entity.stable()) {
                for &(name, ref signature) in signatures {
                    signature
                        .validate_entity(name, entity, |key| world.has_component(handle, key))
                        .map_err(|error| rule_failure(name, tick, &error))?;
                }
            }
        }
        Ok(())
    }
}

struct Call<'a, 'w, 'e> {
    slot: &'a Slot,
    world: &'a mut WorldTxn<'w>,
    executor: Executor,
    tick: u64,
    candidate: &'a mut Candidate,
    edits: &'a mut EditWriter<'e, Candidate>,
}

fn dispatch(
    authority: &Authority,
    world: &mut WorldTxn<'_>,
    executor: Executor,
    tick: u64,
    candidate: &mut Candidate,
    edits: &mut EditWriter<'_, Candidate>,
) -> Result {
    let slot = &authority.program.slots[world.system_id().0 as usize];
    let mut step = slot.step.lock().expect("rule slot");
    authority.program.execute(
        authority,
        &mut step,
        &mut Call {
            slot,
            world,
            executor,
            tick,
            candidate,
            edits,
        },
    )
}

fn set_phase(candidate: &mut Candidate, phase: Option<usize>, tick: u64) {
    candidate.state.phase = phase;
    candidate.state.phase_entered = tick;
    candidate.state.phase_end = None;
    candidate.state.phase_offset = Seconds::ZERO;
    candidate.state.phase_expired = false;
}

fn rule_failure(name: &str, tick: u64, error: &Error) -> Error {
    Error::refused(
        error.code(),
        format!("rule {name} at tick {tick}"),
        error.to_string(),
        "Correct the rule or its inputs, then retry.",
    )
}
