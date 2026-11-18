use thrum_core::{
    StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditWriter,
        EntityWriter, Executor, GameRuntime, RegisteredSystem, RegisteredWorldError,
        RuleSetIdentity, RuntimeUndo, Stage, StructuralPermit, SystemId, SystemSpec, TickGame,
        WorldRead, WritePermit,
    },
};

pub(crate) const IDENTITY: RuleSetIdentity = RuleSetIdentity([17; 32]);
pub(crate) const LIMIT: usize = 4096;

pub(crate) struct Rules {
    pub(crate) resource: u64,
    write: WritePermit<i32>,
    structure: StructuralPermit,
}

impl TickGame for Rules {
    type Error = RegisteredWorldError;
    type Candidate = ();
    type ParallelOutput = ();

    fn reset_candidate(&self, (): &mut ()) {}
    fn reset_parallel_output(&self, (): &mut ()) {}
    fn merge_parallel(
        &self,
        _: SystemId,
        (): &mut (),
        (): &mut (),
        _: u64,
        _: &mut EditWriter<'_, ()>,
    ) {
    }
    fn apply(&mut self, _: &WorldRead<'_>, _: u64, (): &()) {
        self.resource += 1;
    }
}

fn spawn(game: &Rules, identity: u128) -> Edit<()> {
    Edit::spawn(
        &game.structure,
        StableId(identity),
        0,
        |(), _, writer: &mut EntityWriter<'_>| {
            writer.add(12_i32)?;
            Ok(())
        },
    )
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "registered rules use a fallible signature"
)]
fn step(
    game: &Rules,
    world: &WorldRead<'_>,
    _: Executor,
    tick: u64,
    (): &mut (),
    edits: &mut EditWriter<'_, ()>,
) -> Result<(), RegisteredWorldError> {
    if tick == 1 {
        for slot in [1, 0] {
            let handle = world.live_at_slot(slot).expect("seeded body").handle();
            edits.push(Edit::despawn(&game.structure, handle));
        }
        let handle = world.live_at_slot(2).expect("surviving body").handle();
        edits.push(Edit::replace(&game.write, handle, 0, |(), _| &19_i32));
        edits.push(spawn(game, 45));
    } else if tick == 2 {
        for identity in [50, 60] {
            edits.push(spawn(game, identity));
        }
    }
    Ok(())
}

pub(crate) fn runtime(epoch: u128, seed: bool) -> (GameRuntime<Rules>, Rules) {
    let mut registry = ComponentRegistry::default();
    let indexed = registry
        .register_indexed_profile::<i32>(ComponentSchemaId(104), |value| *value > 0)
        .expect("indexed schema");
    registry
        .select_activity_partition::<i32>()
        .expect("partition");
    registry
        .register_profile::<u8>(ComponentSchemaId(12))
        .expect("second schema");
    let system = SystemId(1);
    let mut world = GameRuntime::new_published(
        WorldEpoch(epoch),
        6,
        registry,
        vec![RegisteredSystem::new(
            SystemSpec {
                id: system,
                stage: Stage::Simulate,
                accesses: vec![
                    Access {
                        key: AccessKey::Component(indexed),
                        mode: AccessMode::Write,
                    },
                    Access {
                        key: AccessKey::External(1),
                        mode: AccessMode::Write,
                    },
                ],
                after: vec![],
            },
            step,
        )],
    )
    .expect("test runtime");
    let game = Rules {
        resource: 17,
        write: world.write_permit(system).expect("write permit"),
        structure: world
            .structural_permit(system, 1)
            .expect("structural permit"),
    };
    if seed {
        for (id, value) in [(10, Some(10)), (20, Some(20)), (30, Some(-30)), (40, None)] {
            world
                .seed(StableId(id), |writer| {
                    if let Some(value) = value {
                        writer.add(value)?;
                    }
                    if id == 30 {
                        writer.add(7_u8)?;
                    }
                    Ok(())
                })
                .expect("seed test body");
        }
    }
    (world, game)
}

pub(crate) fn image(world: &GameRuntime<Rules>, game: &Rules) -> Vec<u8> {
    let mut bytes = Vec::new();
    world
        .write_world_image(IDENTITY, &game.resource, &mut bytes)
        .expect("portable world");
    bytes
}

pub(crate) fn source() -> (GameRuntime<Rules>, Rules, RuntimeUndo) {
    let (mut world, mut game) = runtime(1, true);
    world.publish_initial(&mut game, &mut ());
    let mut undo = world.new_undo();
    world
        .step_recording(
            &mut game,
            &mut (),
            &mut undo,
            thrum_core::sim::PublicationMode::Capture,
        )
        .expect("edit, despawn and repack at tick one");
    (world, game, undo)
}

pub(crate) fn digest(world: &GameRuntime<Rules>, game: &Rules) -> u64 {
    world
        .world_digest(&game.resource, |_, _| {})
        .expect("portable digest")
}
