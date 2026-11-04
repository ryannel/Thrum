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

#[derive(Default)]
struct Page {
    tick: u64,
    rows: Vec<(Handle, StableId, Height)>,
}

struct Candidate {
    motion: Read2<Height, Drift>,
    positions: Vec<Option<Height>>,
    handles: Vec<Option<Handle>>,
    edits: Vec<Edit<Candidate>>,
    spawns: Vec<StableId>,
}

fn position_at(candidate: &Candidate, row: usize) -> &dyn std::any::Any {
    candidate.positions[row]
        .as_ref()
        .expect("prepared position")
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
    capture: Read2<Height, Drift>,
    height_read: ReadPermit<Height>,
    structure: StructuralPermit,
    reject: bool,
    bad_edit: Option<Handle>,
    churn: Option<(Handle, StableId)>,
}

impl Rules {
    fn simulate(
        &self,
        world: &WorldRead<'_>,
        _executor: Executor,
        _tick: u64,
        candidate: &mut Candidate,
    ) -> Result<(), GameError> {
        candidate.motion.visit(world, |handle, _, height, drift| {
            candidate.positions[handle.slot as usize] = Some(Height(height.0 + drift.0));
            candidate.handles[handle.slot as usize] = Some(handle);
            if self.churn.is_none_or(|(old, _)| old != handle) {
                candidate.edits.push(Edit::replace(
                    &self.write,
                    handle,
                    handle.slot as usize,
                    position_at,
                ));
            }
        });
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
        Ok(())
    }
}

impl TickGame<Page> for Rules {
    type Error = GameError;
    type Candidate = Candidate;

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.positions.fill(None);
        candidate.handles.fill(None);
        candidate.edits.clear();
        candidate.spawns.clear();
    }

    fn edits<'a>(&self, candidate: &'a Candidate) -> &'a [Edit<Candidate>] {
        &candidate.edits
    }

    fn apply(&mut self, _world: &WorldRead<'_>, _tick: u64, _candidate: &Candidate) {
        self.churn = None;
    }

    fn capture(&mut self, world: &WorldRead<'_>, page: &mut Page, _first: bool, tick: u64) {
        page.tick = tick;
        page.rows.clear();
        self.capture.visit(world, |handle, id, height, _| {
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
    let mut runtime = GameRuntime::new(
        WorldEpoch(9),
        2,
        registry,
        vec![
            RegisteredSystem::new(
                system(
                    1,
                    Stage::Simulate,
                    &[(height, AccessMode::Write), (drift, AccessMode::Read)],
                    true,
                ),
                Rules::simulate,
            ),
            RegisteredSystem::new(
                system(
                    2,
                    Stage::Publish,
                    &[(height, AccessMode::Write), (drift, AccessMode::Read)],
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
    let mut candidate = Candidate {
        motion: runtime
            .read2::<Height, Drift>(SystemId(1))
            .expect("declared motion read"),
        positions: vec![None; 2],
        handles: vec![None; 2],
        edits: Vec::new(),
        spawns: Vec::new(),
    };
    let mut game = Rules {
        write: runtime
            .write_permit::<Height>(SystemId(1))
            .expect("declared height write"),
        capture: runtime
            .read2::<Height, Drift>(SystemId(2))
            .expect("declared capture read"),
        height_read: runtime
            .read_permit::<Height>(SystemId(1))
            .expect("declared sparse read"),
        structure: runtime
            .structural_permit(SystemId(1), 7)
            .expect("declared structure"),
        reject: false,
        bad_edit: None,
        churn: None,
    };
    let first = runtime
        .seed(StableId(1), |entity| {
            entity.add(Height(1.0))?;
            entity.add(Drift(2.0))?;
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
    assert_eq!(
        runtime.step(&mut game, &mut candidate),
        Err(TickError::BaselineRequired)
    );
    runtime.publish_initial(&mut game);
    let baseline = runtime.latest().expect("tick-zero page");
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
    assert_eq!(
        runtime.read().with(&game.height_read, first, |h| *h),
        Ok(Height(3.0))
    );
    assert_eq!(runtime.latest().expect("first tick").rows[0].2, Height(3.0));
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
}
