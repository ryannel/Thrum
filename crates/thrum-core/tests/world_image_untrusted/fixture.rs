use thrum_core::{
    StableId, WorldEpoch,
    sim::{
        ComponentRegistry, ComponentSchemaId, EditWriter, GameRuntime, RuleSetIdentity, SystemId,
        TickGame, WorldRead,
    },
};

pub(crate) const IDENTITY: RuleSetIdentity = RuleSetIdentity([17; 32]);
pub(crate) const LIMIT: usize = 4096;

pub(crate) struct Rules {
    pub(crate) resource: u64,
}

impl TickGame for Rules {
    type Error = std::convert::Infallible;
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
    fn apply(&mut self, _: &WorldRead<'_>, _: u64, (): &()) {}
}

pub(crate) fn runtime(epoch: u128, seed: bool) -> (GameRuntime<Rules>, Rules) {
    let mut registry = ComponentRegistry::default();
    registry
        .register_indexed_profile::<i32>(ComponentSchemaId(12), |value| *value > 0)
        .expect("indexed schema");
    registry
        .select_activity_partition::<i32>()
        .expect("partition");
    let mut world = GameRuntime::new_published(WorldEpoch(epoch), 6, registry, Vec::new())
        .expect("test runtime");
    if seed {
        for (id, value) in [(10, -10_i32), (20, 20_i32)] {
            world
                .seed(StableId(id), |writer| {
                    writer.add(value)?;
                    Ok(())
                })
                .expect("seed test body");
        }
    }
    (
        world,
        Rules {
            resource: u64::try_from(epoch).expect("fixture epoch"),
        },
    )
}

pub(crate) fn image(world: &GameRuntime<Rules>, game: &Rules) -> Vec<u8> {
    let mut bytes = Vec::new();
    world
        .write_world_image(IDENTITY, &game.resource, &mut bytes)
        .expect("portable world");
    bytes
}

pub(crate) fn digest(world: &GameRuntime<Rules>, game: &Rules) -> u64 {
    world
        .world_digest(&game.resource, |_, _| {})
        .expect("portable digest")
}
