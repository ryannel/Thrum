//! Descriptor ownership is per callback; payloads stay shared candidate scratch.

use std::any::Any;

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditError,
        EditWriter, EntityWriter, ExecutionMode, GameRuntime, PublicationMode, ReadPermit,
        RegisteredSystem, RegisteredWorldError, Stage, StructuralPermit, SystemId, SystemSpec,
        TickError, TickGame, WorldRead, WritePermit,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Height(u32);

#[derive(Debug, Default, Eq, PartialEq)]
struct Page {
    tick: u64,
    height: Option<Height>,
    spawned: bool,
}

#[derive(Default)]
struct Candidate {
    rows: Vec<(Handle, Height)>,
    wrong_type: bool,
    merged: Vec<SystemId>,
}

#[derive(Clone, Copy)]
enum Fault {
    None,
    WrongSystem,
    WrongType,
    WrongHandle,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Callbacks {
    Serial,
    Parallel,
    Direct,
}

struct Rules {
    handle: Handle,
    read: ReadPermit<Height>,
    write: WritePermit<Height>,
    structure: StructuralPermit,
    fault: Fault,
}

fn rows(candidate: &Candidate) -> &dyn Any {
    if candidate.wrong_type {
        &candidate.wrong_type
    } else {
        &candidate.rows
    }
}

fn spawn(
    _candidate: &Candidate,
    _index: usize,
    entity: &mut EntityWriter<'_>,
) -> Result<(), RegisteredWorldError> {
    entity.add(Height(7))?;
    Ok(())
}

impl Rules {
    fn stage(&self, candidate: &mut Candidate, edits: &mut EditWriter<'_, Candidate>) {
        candidate.rows.push((self.handle, Height(2)));
        edits.push(Edit::replace_batch(&self.write, rows));
        edits.push(Edit::spawn(&self.structure, StableId(2), 0, spawn));
    }

    fn resolve(&self, candidate: &mut Candidate, edits: &mut EditWriter<'_, Candidate>) {
        // A's accessor intentionally observes B's final candidate payload.
        candidate.rows[0].1 = Height(9);
        match self.fault {
            Fault::None => {}
            Fault::WrongSystem => edits.push(Edit::despawn(&self.structure, self.handle)),
            Fault::WrongType => candidate.wrong_type = true,
            Fault::WrongHandle => candidate.rows[0].0.generation += 1,
        }
    }
}

impl TickGame<Page> for Rules {
    type Error = ();
    type Candidate = Candidate;
    type ParallelOutput = Option<SystemId>;

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.rows.clear();
        candidate.wrong_type = false;
        candidate.merged.clear();
    }

    fn reset_parallel_output(&self, output: &mut Self::ParallelOutput) {
        *output = None;
    }

    fn merge_parallel(
        &self,
        system: SystemId,
        output: &mut Self::ParallelOutput,
        candidate: &mut Candidate,
        _tick: u64,
        edits: &mut EditWriter<'_, Candidate>,
    ) {
        assert_eq!(*output, Some(system));
        candidate.merged.push(system);
        if system == SystemId(1) {
            self.stage(candidate, edits);
        } else {
            self.resolve(candidate, edits);
        }
    }

    fn apply(&mut self, _world: &WorldRead<'_>, _tick: u64, _candidate: &Candidate) {}

    fn capture(
        &mut self,
        world: &WorldRead<'_>,
        _candidate: &mut Candidate,
        _events: &[thrum_core::sim::StructuralChange],
        page: &mut Page,
        _cold: bool,
        _page_dirty: thrum_core::sim::DirtyPage<'_>,
        _outbound_dirty: thrum_core::sim::DirtyPage<'_>,
        _source: Option<&Page>,
        tick: u64,
        _timeline: u64,
    ) {
        page.tick = tick;
        page.height = world.with(&self.read, self.handle, |height| *height).ok();
        page.spawned = world.handle(StableId(2)).is_some();
    }
}

fn fixture(
    callbacks: Callbacks,
    mode: ExecutionMode,
) -> (GameRuntime<Page, Rules>, Rules, Candidate) {
    let mut registry = ComponentRegistry::default();
    let height = registry
        .register::<Height>(ComponentSchemaId(0xE01))
        .expect("height schema");
    // Distinct external keys put parallel workers in the same conflict-free
    // wave. Their private outputs are merged in compiled system order.
    let first = SystemSpec {
        id: SystemId(1),
        stage: Stage::Resolve,
        accesses: vec![
            Access {
                key: AccessKey::Component(height),
                mode: AccessMode::Write,
            },
            Access {
                key: AccessKey::External(7),
                mode: AccessMode::Write,
            },
        ],
        after: vec![],
    };
    let second = SystemSpec {
        id: SystemId(2),
        stage: Stage::Resolve,
        accesses: vec![Access {
            key: AccessKey::External(8),
            mode: AccessMode::Write,
        }],
        after: vec![],
    };
    let systems = if callbacks == Callbacks::Parallel {
        vec![
            RegisteredSystem::parallel(first, |_, _, _, _, output| {
                *output = Some(SystemId(1));
                Ok(())
            }),
            RegisteredSystem::parallel(second, |_, _, _, _, output| {
                *output = Some(SystemId(2));
                Ok(())
            }),
        ]
    } else if callbacks == Callbacks::Direct {
        vec![
            RegisteredSystem::direct(first, |rules: &Rules, world, _, _, candidate, edits| {
                assert!(
                    world
                        .put(&rules.write, rules.handle, Height(4))
                        .expect("tentative write")
                );
                rules.stage(candidate, edits);
                Ok(())
            }),
            RegisteredSystem::new(second, |rules: &Rules, _, _, _, candidate, edits| {
                if matches!(rules.fault, Fault::WrongSystem) {
                    edits.push(Edit::replace_batch(&rules.write, rows));
                } else {
                    rules.resolve(candidate, edits);
                }
                Ok(())
            }),
        ]
    } else {
        vec![
            RegisteredSystem::new(first, |rules: &Rules, _, _, _, candidate, edits| {
                rules.stage(candidate, edits);
                Ok(())
            }),
            RegisteredSystem::new(second, |rules: &Rules, _, _, _, candidate, edits| {
                rules.resolve(candidate, edits);
                Ok(())
            }),
        ]
    };
    let mut runtime = GameRuntime::new(WorldEpoch(11), 2, registry, systems, Page::default)
        .expect("edit ownership runtime");
    runtime.set_execution_mode(mode);
    let handle = runtime
        .seed(StableId(1), |entity| {
            entity.add(Height(1))?;
            Ok(())
        })
        .expect("baseline entity");
    let mut rules = Rules {
        handle,
        read: runtime.read_permit(SystemId(1)).expect("height read"),
        write: runtime.write_permit(SystemId(1)).expect("height write"),
        structure: runtime
            .structural_permit(SystemId(1), 7)
            .expect("A's structure permit"),
        fault: Fault::None,
    };
    let mut candidate = Candidate::default();
    runtime.publish_initial(&mut rules, &mut candidate);
    (runtime, rules, candidate)
}

fn assert_refusal_and_retry(
    callbacks: Callbacks,
    mode: ExecutionMode,
    fault: Fault,
    error: EditError,
) {
    let (mut runtime, mut rules, mut candidate) = fixture(callbacks, mode);
    let held = runtime.latest().expect("held baseline page");
    rules.fault = fault;
    assert_eq!(
        runtime.step(&mut rules, &mut candidate),
        Err(TickError::Edit(error))
    );
    assert_eq!(runtime.tick(), 0);
    assert_eq!(runtime.handle(StableId(1)), Some(rules.handle));
    assert!(runtime.handle(StableId(2)).is_none());
    assert_eq!(
        runtime
            .read()
            .with(&rules.read, rules.handle, |height| *height),
        Ok(Height(1))
    );
    let baseline = Page {
        tick: 0,
        height: Some(Height(1)),
        spawned: false,
    };
    assert_eq!(*held, baseline);
    assert_eq!(*runtime.latest().expect("unchanged publication"), baseline);
    if callbacks == Callbacks::Parallel {
        assert_eq!(candidate.merged, [SystemId(1), SystemId(2)]);
    }

    rules.fault = Fault::None;
    assert_eq!(runtime.step(&mut rules, &mut candidate), Ok(1));
    assert_eq!(
        runtime
            .read()
            .with(&rules.read, rules.handle, |height| *height),
        Ok(Height(9))
    );
    assert!(runtime.handle(StableId(2)).is_some());
    assert_eq!(
        *runtime.latest().expect("committed page"),
        Page {
            tick: 1,
            height: Some(Height(9)),
            spawned: true,
        }
    );
    assert_eq!(*held, baseline);
    if callbacks == Callbacks::Parallel {
        assert_eq!(candidate.merged, [SystemId(1), SystemId(2)]);
    }
}

#[test]
fn later_callback_cannot_append_another_systems_structural_edit() {
    assert_refusal_and_retry(
        Callbacks::Serial,
        ExecutionMode::Serial,
        Fault::WrongSystem,
        EditError::WrongSystem,
    );
}

#[test]
fn final_preflight_checks_payload_type_and_handle_after_later_callback_changes() {
    for (fault, error) in [
        (Fault::WrongType, EditError::WrongValueType),
        (
            Fault::WrongHandle,
            EditError::World(RegisteredWorldError::InvalidHandle),
        ),
    ] {
        assert_refusal_and_retry(Callbacks::Serial, ExecutionMode::Serial, fault, error);
    }
}

#[test]
fn ordered_parallel_merge_cannot_append_another_systems_structural_edit() {
    for mode in [ExecutionMode::Serial, ExecutionMode::RayonForced] {
        assert_refusal_and_retry(
            Callbacks::Parallel,
            mode,
            Fault::WrongSystem,
            EditError::WrongSystem,
        );
    }
}

#[test]
fn later_callback_updates_are_the_final_payload_of_an_earlier_descriptor() {
    let (mut runtime, mut rules, mut candidate) = fixture(Callbacks::Serial, ExecutionMode::Serial);
    assert_eq!(runtime.step(&mut rules, &mut candidate), Ok(1));
    assert_eq!(
        runtime
            .read()
            .with(&rules.read, rules.handle, |height| *height),
        Ok(Height(9))
    );
}

#[test]
fn foreign_component_edit_rolls_back_direct_write_and_retry_retains_first_preimage() {
    let (mut runtime, mut rules, mut candidate) = fixture(Callbacks::Direct, ExecutionMode::Serial);
    let held = runtime.latest().expect("held baseline page");
    let baseline = Page {
        tick: 0,
        height: Some(Height(1)),
        spawned: false,
    };
    let mut undo = runtime.new_undo();
    rules.fault = Fault::WrongSystem;
    assert_eq!(
        runtime.step_recording(
            &mut rules,
            &mut candidate,
            &mut undo,
            PublicationMode::Capture
        ),
        Err(TickError::Edit(EditError::WrongSystem))
    );
    assert_eq!(runtime.tick(), 0);
    assert_eq!(runtime.handle(StableId(1)), Some(rules.handle));
    assert!(runtime.handle(StableId(2)).is_none());
    assert_eq!(
        runtime
            .read()
            .with(&rules.read, rules.handle, |height| *height),
        Ok(Height(1))
    );
    assert_eq!(*held, baseline);
    assert_eq!(*runtime.latest().expect("unchanged publication"), baseline);

    rules.fault = Fault::None;
    assert_eq!(
        runtime.step_recording(
            &mut rules,
            &mut candidate,
            &mut undo,
            PublicationMode::Capture
        ),
        Ok(1)
    );
    let committed = runtime.latest().expect("committed page");
    assert_eq!(
        *committed,
        Page {
            tick: 1,
            height: Some(Height(9)),
            spawned: true
        }
    );
    assert_eq!(
        runtime
            .read()
            .with(&rules.read, rules.handle, |height| *height),
        Ok(Height(9))
    );
    assert!(runtime.handle(StableId(2)).is_some());
    runtime
        .rewind_many(std::slice::from_mut(&mut undo))
        .expect("rewind direct and staged changes");
    assert_eq!(runtime.tick(), 0);
    assert_eq!(runtime.handle(StableId(1)), Some(rules.handle));
    assert!(runtime.handle(StableId(2)).is_none());
    assert_eq!(
        runtime
            .read()
            .with(&rules.read, rules.handle, |height| *height),
        Ok(Height(1))
    );
    assert_eq!(*held, baseline);
    assert_eq!(
        *committed,
        Page {
            tick: 1,
            height: Some(Height(9)),
            spawned: true
        }
    );
}
