//! A second-game-shaped consumer of the public generic runtime. Its component
//! and rule types are defined here, never in thrum-core production code.

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId, Edit,
        EntityWriter, Executor, GameRuntime, Read2, ReadPermit, RegisteredSystem,
        RegisteredWorldError, RuntimeError, Stage, StructuralPermit, SystemId, SystemSpec,
        TickError, TickGame, WorldRead, WritePermit,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Height(f64);

#[derive(Clone, Copy)]
struct Drift(f64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Tag(u32);

#[derive(Default)]
struct Page {
    tick: u64,
    rows: Vec<(Handle, StableId, Height)>,
}

struct Candidate {
    motion: Read2<Height, Drift>,
    order_query: Read2<Height, Drift>,
    order: Vec<StableId>,
    capture: Read2<Height, Drift>,
    batch: Vec<(Handle, Height)>,
    edits: Vec<Edit<Candidate>>,
    spawns: Vec<StableId>,
}

fn height_batch(candidate: &Candidate) -> &dyn std::any::Any {
    &candidate.batch
}

fn fill_spawn(
    _candidate: &Candidate,
    _index: usize,
    entity: &mut EntityWriter<'_>,
) -> Result<(), RegisteredWorldError> {
    entity.add(Height(7.0))?;
    entity.add(Drift(1.0))?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GameError {
    Rejected,
    InvalidChurn,
}

struct Rules {
    write: WritePermit<Height>,
    height_read: ReadPermit<Height>,
    bytes_read: ReadPermit<Vec<u8>>,
    tag_read: ReadPermit<Tag>,
    tag_write: WritePermit<Tag>,
    structure: StructuralPermit,
    reject: bool,
    bad_edit: Option<Handle>,
    churn: Option<(Handle, StableId)>,
    optional_edit: Option<(Handle, bool)>,
    duplicate_batch: bool,
}

fn tag_at(_candidate: &Candidate, _index: usize) -> &dyn std::any::Any {
    &Tag(7)
}

impl Rules {
    fn simulate(
        &self,
        world: &WorldRead<'_>,
        _executor: Executor,
        _tick: u64,
        candidate: &mut Candidate,
    ) -> Result<(), GameError> {
        candidate.order.clear();
        candidate
            .order_query
            .visit_canonical(world, |_, id, _, _| candidate.order.push(id));
        candidate.motion.visit(world, |handle, _, height, drift| {
            if self.churn.is_none_or(|(old, _)| old != handle) {
                candidate.batch.push((handle, Height(height.0 + drift.0)));
            }
        });
        if self.duplicate_batch {
            candidate.batch.push(candidate.batch[0]);
        }
        candidate
            .edits
            .push(Edit::replace_batch(&self.write, height_batch));
        if let Some((old, replacement)) = self.churn {
            world
                .with(&self.height_read, old, |height| height.0)
                .map_err(|_| GameError::InvalidChurn)?;
            if world.handle(replacement).is_some() {
                return Err(GameError::InvalidChurn);
            }
            candidate.edits.push(Edit::despawn(&self.structure, old));
            candidate.spawns.push(replacement);
            candidate
                .edits
                .push(Edit::spawn(&self.structure, replacement, 0, fill_spawn));
        }
        if self.reject {
            return Err(GameError::Rejected);
        }
        if let Some(handle) = self.bad_edit {
            candidate.edits.push(Edit::despawn(&self.structure, handle));
            candidate
                .edits
                .push(Edit::spawn(&self.structure, StableId(2), 0, fill_spawn));
        }
        if let Some((handle, insert)) = self.optional_edit {
            candidate.edits.push(if insert {
                Edit::insert(&self.tag_write, handle, 0, tag_at)
            } else {
                Edit::remove(&self.tag_write, handle)
            });
        }
        Ok(())
    }
}

impl TickGame<Page> for Rules {
    type Error = GameError;
    type Candidate = Candidate;
    type ParallelOutput = ();

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.batch.clear();
        candidate.order.clear();
        candidate.edits.clear();
        candidate.spawns.clear();
    }

    fn reset_parallel_output(&self, _output: &mut ()) {}

    fn merge_parallel(
        &self,
        _system: SystemId,
        _output: &mut (),
        _candidate: &mut Candidate,
        _tick: u64,
    ) {
    }

    fn edits<'a>(&self, candidate: &'a Candidate) -> &'a [Edit<Candidate>] {
        &candidate.edits
    }

    fn apply(&mut self, _world: &WorldRead<'_>, _tick: u64, _candidate: &Candidate) {
        self.churn = None;
        self.optional_edit = None;
    }

    fn capture(
        &mut self,
        world: &WorldRead<'_>,
        candidate: &mut Candidate,
        _events: &[thrum_core::sim::StructuralChange],
        page: &mut Page,
        _first: bool,
        tick: u64,
        _timeline: u64,
    ) {
        page.tick = tick;
        page.rows.clear();
        candidate.capture.visit(world, |handle, id, height, _| {
            page.rows.push((handle, id, *height));
        });
    }
}

fn system(
    id: u32,
    stage: Stage,
    accesses: &[(ComponentKey, AccessMode)],
    structural: bool,
) -> SystemSpec {
    let mut declarations: Vec<_> = accesses
        .iter()
        .map(|(key, mode)| Access {
            key: AccessKey::Component(*key),
            mode: *mode,
        })
        .collect();
    if structural {
        declarations.push(Access {
            key: AccessKey::External(7),
            mode: AccessMode::Write,
        });
    }
    SystemSpec {
        id: SystemId(id),
        stage,
        accesses: declarations,
        after: vec![],
    }
}

#[test]
fn game_owned_components_rules_and_pages_commit_once_after_rejection_and_churn() {
    let mut registry = ComponentRegistry::default();
    let height = registry
        .register::<Height>(ComponentSchemaId(0xA1))
        .expect("height schema");
    let drift = registry
        .register::<Drift>(ComponentSchemaId(0xA2))
        .expect("drift schema");
    let payload = registry
        .register_with_snapshot::<Vec<u8>>(ComponentSchemaId(0xA3), Vec::clone, Vec::capacity)
        .expect("independent heap component schema");
    let tag = registry
        .register::<Tag>(ComponentSchemaId(0xA4))
        .expect("optional tag schema");
    let mut runtime = GameRuntime::new(
        WorldEpoch(9),
        2,
        registry,
        vec![
            RegisteredSystem::new(
                system(
                    1,
                    Stage::Simulate,
                    &[
                        (height, AccessMode::Write),
                        (drift, AccessMode::Read),
                        (tag, AccessMode::Write),
                    ],
                    true,
                ),
                Rules::simulate,
            ),
            RegisteredSystem::new(
                system(
                    2,
                    Stage::Publish,
                    &[
                        (height, AccessMode::Write),
                        (drift, AccessMode::Read),
                        (payload, AccessMode::Read),
                    ],
                    false,
                ),
                |_, _, _, _, _| Ok(()),
            ),
        ],
        Page::default,
    )
    .expect("generic game setup");
    assert!(matches!(
        runtime.read2::<Height, Drift>(SystemId(99)),
        Err(RuntimeError::UndeclaredAccess)
    ));
    assert!(matches!(
        runtime.write_permit::<Drift>(SystemId(2)),
        Err(RuntimeError::UndeclaredAccess)
    ));
    let mut candidate = test_candidate(&runtime);
    let mut game = test_rules(&runtime);
    let first = seed_test_world(&mut runtime);
    assert_invalid_journal_access(&runtime);
    assert_eq!(
        runtime.step(&mut game, &mut candidate),
        Err(TickError::BaselineRequired)
    );
    runtime.publish_initial(&mut game, &mut candidate);
    let baseline = runtime.latest().expect("tick-zero page");
    let mut rolling = runtime.capture();
    assert!(rolling.reserved_bytes() >= 1_048_576);
    assert!(rolling.clone_independent().reserved_bytes() >= 1_048_576);
    let mut delta_one = runtime.new_delta();
    let mut delta_two = runtime.new_delta();
    assert!(matches!(
        runtime.capture_delta_into(&mut delta_one),
        Err(RuntimeError::NoCommittedTick)
    ));
    game.reject = true;
    assert_eq!(
        runtime.step(&mut game, &mut candidate),
        Err(TickError::Game(GameError::Rejected))
    );
    assert_eq!(runtime.tick(), 0);
    assert_eq!(baseline.rows[0].2, Height(1.0));
    assert_eq!(runtime.latest().expect("same page").tick, 0);
    game.reject = false;
    assert_eq!(runtime.step(&mut game, &mut candidate), Ok(1));
    runtime
        .capture_delta_into(&mut delta_one)
        .expect("first delta");
    rolling.advance(&delta_one).expect("contiguous first tick");
    let branch_point = rolling.clone_independent();
    assert_eq!(
        runtime.read().with(&game.height_read, first, |h| *h),
        Ok(Height(3.0))
    );
    assert_eq!(runtime.latest().expect("first tick").rows[0].2, Height(3.0));
    assert_duplicate_batch_refuses(&mut runtime, &mut game, &mut candidate, first);
    game.bad_edit = Some(first);
    assert_eq!(
        runtime.step(&mut game, &mut candidate),
        Err(TickError::Edit(thrum_core::sim::EditError::Conflict))
    );
    assert_eq!(runtime.tick(), 1);
    assert_eq!(
        runtime.read().with(&game.height_read, first, |h| *h),
        Ok(Height(3.0))
    );
    game.bad_edit = None;
    game.churn = Some((first, StableId(3)));
    assert_eq!(runtime.step(&mut game, &mut candidate), Ok(2));
    runtime
        .capture_delta_into(&mut delta_two)
        .expect("churn delta");
    rolling.advance(&delta_two).expect("contiguous churn");
    assert_eq!(rolling.advance(&delta_one), Err(RuntimeError::WrongCapture));
    assert!(
        runtime
            .read()
            .with(&game.height_read, first, |h| *h)
            .is_err()
    );
    let replacement = runtime.handle(StableId(3)).expect("replacement identity");
    assert_eq!(replacement.slot, first.slot);
    assert_ne!(replacement.generation, first.generation);
    assert_eq!(
        runtime
            .latest()
            .expect("churn page")
            .rows
            .iter()
            .find(|(_, id, _)| *id == StableId(3))
            .map(|(_, _, height)| *height),
        Some(Height(7.0))
    );
    assert_eq!(
        baseline.rows[0].2,
        Height(1.0),
        "leased baseline stays complete"
    );
    let saved = rolling;
    runtime
        .step(&mut game, &mut candidate)
        .expect("tick after churn");
    let order = candidate.order.clone();
    assert_eq!(order, [StableId(3), StableId(2)]);
    runtime
        .restore(&saved, Page::default)
        .expect("same runtime capture");
    assert_eq!(
        runtime.capture_delta_into(&mut delta_two),
        Err(RuntimeError::NoCommittedTick),
        "restored future edits cannot become a mislabeled delta"
    );
    runtime.publish_initial(&mut game, &mut candidate);
    runtime
        .step(&mut game, &mut candidate)
        .expect("replayed tick");
    assert_eq!(candidate.order, order);

    runtime
        .restore(&branch_point, Page::default)
        .expect("restore branch point");
    let restored_bytes = runtime
        .read()
        .with(&game.bytes_read, first, |bytes| {
            (bytes.len(), bytes.first().copied(), bytes.last().copied())
        })
        .expect("restored non-Copy component");
    assert_eq!(restored_bytes, (1_048_576, Some(7), Some(7)));
    runtime.publish_initial(&mut game, &mut candidate);
    game.churn = Some((first, StableId(4)));
    runtime
        .step(&mut game, &mut candidate)
        .expect("alternate churn");
    let mut alternate = runtime.new_delta();
    runtime
        .capture_delta_into(&mut alternate)
        .expect("alternate tick-two delta");
    runtime
        .step(&mut game, &mut candidate)
        .expect("alternate next tick");
    let mut alternate_next = runtime.new_delta();
    runtime
        .capture_delta_into(&mut alternate_next)
        .expect("alternate tick-three delta");
    let mut first_branch = branch_point.clone_independent();
    first_branch.advance(&delta_two).expect("original churn");
    assert_eq!(
        first_branch.advance(&alternate_next),
        Err(RuntimeError::WrongCapture),
        "a numerically adjacent tick from another branch is not contiguous"
    );
    assert_optional_component_history(&mut runtime, &mut game, &mut candidate);
    assert_undo_is_one_shot_and_busy_tick_cannot_fake_patch(
        &mut runtime,
        &mut game,
        &mut candidate,
    );
}

fn assert_undo_is_one_shot_and_busy_tick_cannot_fake_patch(
    runtime: &mut GameRuntime<Page, Rules>,
    game: &mut Rules,
    candidate: &mut Candidate,
) {
    runtime.publish_initial(game, candidate);
    let mut undo = runtime.new_undo();
    runtime
        .step_recording(game, candidate, &mut undo)
        .expect("record one committed revision");
    let future = runtime.capture();
    let future_height = runtime
        .read()
        .with(
            &game.height_read,
            runtime.handle(StableId(2)).expect("entity"),
            |h| *h,
        )
        .expect("future height");
    runtime
        .rewind_many(std::slice::from_mut(&mut undo))
        .expect("first inverse application");
    runtime
        .restore(&future, Page::default)
        .expect("same numeric revision restored cold");
    assert_eq!(
        runtime.rewind_many(std::slice::from_mut(&mut undo)),
        Err(RuntimeError::WrongCapture),
        "a drained preimage must never reverse a second time"
    );
    assert_eq!(
        runtime.read().with(
            &game.height_read,
            runtime.handle(StableId(2)).expect("entity"),
            |h| *h
        ),
        Ok(future_height)
    );
    runtime.publish_initial(game, candidate);
    let mut held = vec![runtime.latest().expect("published baseline")];
    while runtime.can_publish_next() {
        runtime.step(game, candidate).expect("lease tick");
        held.push(runtime.latest().expect("published tick"));
    }
    runtime.step(game, candidate).expect("busy tick commits");
    assert!(!runtime.has_current_publication());
    assert!(!runtime.patch_rewound_page(&[], |_| Some(0), |_, _, _, _, _, _| {}));
    assert!(!runtime.has_current_publication());
    drop(held);
}

fn assert_duplicate_batch_refuses(
    runtime: &mut GameRuntime<Page, Rules>,
    game: &mut Rules,
    candidate: &mut Candidate,
    first: Handle,
) {
    game.duplicate_batch = true;
    assert_eq!(
        runtime.step(game, candidate),
        Err(TickError::Edit(thrum_core::sim::EditError::Conflict))
    );
    assert_eq!(runtime.tick(), 1);
    assert_eq!(
        runtime.read().with(&game.height_read, first, |h| *h),
        Ok(Height(3.0))
    );
    game.duplicate_batch = false;
}

fn test_rules(runtime: &GameRuntime<Page, Rules>) -> Rules {
    Rules {
        write: runtime
            .write_permit::<Height>(SystemId(1))
            .expect("declared height write"),
        height_read: runtime
            .read_permit::<Height>(SystemId(1))
            .expect("declared sparse read"),
        bytes_read: runtime
            .read_permit::<Vec<u8>>(SystemId(2))
            .expect("declared payload read"),
        tag_read: runtime
            .read_permit::<Tag>(SystemId(1))
            .expect("declared tag read"),
        tag_write: runtime
            .write_permit::<Tag>(SystemId(1))
            .expect("declared tag write"),
        structure: runtime
            .structural_permit(SystemId(1), 7)
            .expect("declared structure"),
        reject: false,
        bad_edit: None,
        churn: None,
        optional_edit: None,
        duplicate_batch: false,
    }
}

fn assert_optional_component_history(
    runtime: &mut GameRuntime<Page, Rules>,
    game: &mut Rules,
    candidate: &mut Candidate,
) {
    let handle = runtime.handle(StableId(2)).expect("unchanged entity");
    let before = runtime.capture();
    game.optional_edit = Some((handle, true));
    runtime.step(game, candidate).expect("insert optional tag");
    let inserted = runtime.capture();
    assert_eq!(
        runtime.read().with(&game.tag_read, handle, |tag| *tag),
        Ok(Tag(7))
    );
    let mut insert_delta = runtime.new_delta();
    runtime
        .capture_delta_into(&mut insert_delta)
        .expect("insert delta");
    let mut replayed = before.clone_independent();
    replayed.advance(&insert_delta).expect("apply insert delta");
    runtime
        .restore(&replayed, Page::default)
        .expect("restore inserted tag");
    assert_eq!(
        runtime.read().with(&game.tag_read, handle, |tag| *tag),
        Ok(Tag(7))
    );
    runtime.publish_initial(game, candidate);
    game.optional_edit = Some((handle, false));
    runtime.step(game, candidate).expect("remove optional tag");
    let mut remove_delta = runtime.new_delta();
    runtime
        .capture_delta_into(&mut remove_delta)
        .expect("remove delta");
    replayed.advance(&remove_delta).expect("apply remove delta");
    assert_eq!(
        runtime.read().with(&game.tag_read, handle, |tag| *tag),
        Err(RegisteredWorldError::MissingComponent)
    );
    runtime
        .restore(&replayed, Page::default)
        .expect("restore removed tag");
    assert_eq!(
        runtime.read().with(&game.tag_read, handle, |tag| *tag),
        Err(RegisteredWorldError::MissingComponent)
    );
    runtime
        .restore(&inserted, Page::default)
        .expect("restore saved insert");
    assert_eq!(
        runtime.read().with(&game.tag_read, handle, |tag| *tag),
        Ok(Tag(7))
    );
}

fn seed_test_world(runtime: &mut GameRuntime<Page, Rules>) -> Handle {
    let first = runtime
        .seed(StableId(1), |entity| {
            entity.add(Height(1.0))?;
            entity.add(Drift(2.0))?;
            entity.add(vec![7_u8; 1_048_576])?;
            Ok(())
        })
        .expect("first bundle");
    runtime
        .seed(StableId(2), |entity| {
            entity.add(Height(10.0))?;
            entity.add(Drift(0.0))?;
            Ok(())
        })
        .expect("second bundle");
    first
}

fn test_candidate(runtime: &GameRuntime<Page, Rules>) -> Candidate {
    Candidate {
        motion: runtime
            .read2::<Height, Drift>(SystemId(1))
            .expect("declared motion read"),
        order_query: runtime
            .read2::<Height, Drift>(SystemId(1))
            .expect("declared canonical read"),
        order: Vec::new(),
        capture: runtime
            .read2::<Height, Drift>(SystemId(2))
            .expect("declared capture read"),
        batch: Vec::new(),
        edits: Vec::new(),
        spawns: Vec::new(),
    }
}

fn assert_invalid_journal_access(runtime: &GameRuntime<Page, Rules>) {
    assert!(runtime.read().live_at_slot(u32::MAX).is_none());
    assert!(!runtime.read().visit_changes_since(0, 99, |_| {}));
}
