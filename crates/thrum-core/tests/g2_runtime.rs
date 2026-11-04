//! A second-game-shaped consumer of the public generic runtime. Its component
//! and rule types are defined here, never in thrum-core production code.

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
        GameRuntime, Read2, ReadPermit, RuntimeError, Stage, StructuralPermit, SystemId,
        SystemSpec, TickError, TickGame, WorldCommit, WorldRead, Write1,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GameError {
    Rejected,
    InvalidChurn,
}

struct Rules {
    write: Write1<Height>,
    capture: Read2<Height, Drift>,
    height_read: ReadPermit<Height>,
    structure: StructuralPermit,
    reject: bool,
    churn: Option<(Handle, StableId)>,
}

impl TickGame<Page> for Rules {
    type Error = GameError;
    type Candidate = Candidate;

    fn prepare(
        &self,
        world: &WorldRead<'_>,
        _schedule: &thrum_core::sim::Schedule,
        _tick: u64,
        candidate: &mut Candidate,
    ) -> Result<(), GameError> {
        candidate.positions.fill(None);
        candidate.motion.visit(world, |handle, _, height, drift| {
            candidate.positions[handle.slot as usize] = Some(Height(height.0 + drift.0));
        });
        if let Some((old, replacement)) = self.churn {
            world
                .with(&self.height_read, old, |height| height.0)
                .map_err(|_| GameError::InvalidChurn)?;
            if world.handle(replacement).is_some() {
                return Err(GameError::InvalidChurn);
            }
        }
        if self.reject {
            return Err(GameError::Rejected);
        }
        Ok(())
    }

    fn apply(&mut self, world: &mut WorldCommit<'_>, _tick: u64, candidate: &Candidate) {
        self.write.visit(world, |handle, _, height| {
            if let Some(next) = candidate.positions[handle.slot as usize] {
                *height = next;
            }
        });
        if let Some((old, replacement)) = self.churn.take() {
            world
                .despawn(&self.structure, old)
                .expect("validated old handle");
            world
                .spawn(&self.structure, replacement, |entity| {
                    entity.add(Height(7.0))?;
                    entity.add(Drift(1.0))?;
                    Ok(())
                })
                .expect("validated replacement bundle and capacity");
        }
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
            system(
                1,
                Stage::Simulate,
                &[(height, AccessMode::Write), (drift, AccessMode::Read)],
                false,
            ),
            system(
                2,
                Stage::Publish,
                &[(height, AccessMode::Write), (drift, AccessMode::Read)],
                true,
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
        runtime.write1::<Drift>(SystemId(2)),
        Err(RuntimeError::UndeclaredAccess)
    ));
    let mut candidate = Candidate {
        motion: runtime
            .read2::<Height, Drift>(SystemId(1))
            .expect("declared motion read"),
        positions: vec![None; 2],
    };
    let mut game = Rules {
        write: runtime
            .write1::<Height>(SystemId(2))
            .expect("declared height write"),
        capture: runtime
            .read2::<Height, Drift>(SystemId(2))
            .expect("declared capture read"),
        height_read: runtime
            .read_permit::<Height>(SystemId(1))
            .expect("declared sparse read"),
        structure: runtime
            .structural_permit(SystemId(2), 7)
            .expect("declared structure"),
        reject: false,
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
