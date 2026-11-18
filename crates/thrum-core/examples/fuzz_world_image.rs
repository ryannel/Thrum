//! Stdin fuzz target for bounded portable world-image restore.

use std::{hint::black_box, io::Read};
use thrum_core::{
    WorldEpoch,
    sim::{
        ComponentRegistry, ComponentSchemaId, EditWriter, GameRuntime, RuleSetIdentity, SystemId,
        TickGame, WorldRead,
    },
};

struct Rules;

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

fn main() -> std::io::Result<()> {
    const LIMIT: usize = 64 * 1024;
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(u64::try_from(LIMIT + 1).expect("stdin byte limit"))
        .read_to_end(&mut bytes)?;
    let mut registry = ComponentRegistry::default();
    registry
        .register_indexed_profile::<u32>(ComponentSchemaId(1), |value| *value != 0)
        .expect("profile schema");
    registry
        .select_activity_partition::<u32>()
        .expect("indexed column");
    let mut world = GameRuntime::<Rules>::new_published(WorldEpoch(1), 8, registry, Vec::new())
        .expect("fuzz world");
    let _ =
        black_box(world.restore_world_image(RuleSetIdentity([0; 32]), &mut 0_u64, &bytes, LIMIT));
    Ok(())
}
