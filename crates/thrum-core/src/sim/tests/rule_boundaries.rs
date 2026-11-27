use std::sync::Mutex;

use crate::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId, Edit,
        EditWriter, GameRuntime, PublicationMode, ReadPermit, RegisteredSystem, Stage,
        StructuralPermit, SystemId, SystemSpec, TickError, TickGame, WorldRead, WorldTxn,
        WritePermit,
    },
};

use super::TestError;

const STRUCTURE: u64 = 1;
const ORIGINAL: StableId = StableId(1);
const FIRST: StableId = StableId(2);
const SECOND: StableId = StableId(3);

struct Game {
    edit: [StructuralPermit; 2],
    write: WritePermit<u32>,
    read: [ReadPermit<u32>; 2],
    original: Handle,
    refusal: Mutex<Option<TestError>>,
}

impl TickGame for Game {
    type Error = TestError;
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

fn replace_life(
    edits: &mut EditWriter<'_, ()>,
    permit: &StructuralPermit,
    handle: Handle,
    id: StableId,
) {
    edits.push(Edit::despawn(permit, handle));
    edits.push(Edit::spawn(permit, id, 0, |(), _, bundle| {
        bundle.add(21_u32).map(|_| ())
    }));
}

fn fixture() -> (GameRuntime<Game>, Game) {
    let mut registry = ComponentRegistry::default();
    registry
        .register_profile::<u32>(ComponentSchemaId(1))
        .unwrap();
    let spec = |id, after| SystemSpec {
        id: SystemId(id),
        stage: Stage::Simulate,
        accesses: vec![
            Access {
                key: AccessKey::Component(ComponentKey::of::<u32>()),
                mode: AccessMode::Write,
            },
            Access {
                key: AccessKey::External(STRUCTURE),
                mode: AccessMode::Write,
            },
        ],
        after,
    };
    let first = RegisteredSystem::direct(spec(1, vec![]), |game: &Game, world, _, _, (), edits| {
        replace_life(edits, &game.edit[0], game.original, FIRST);
        assert!(world.read().handle(ORIGINAL).is_some());
        assert!(world.read().handle(FIRST).is_none());
        Ok(())
    });
    let second = RegisteredSystem::direct(
        spec(2, vec![SystemId(1)]),
        |game: &Game, world, _, _, (), edits| {
            let first = world.read().handle(FIRST).unwrap();
            assert_eq!(
                world
                    .read()
                    .with(&game.read[0], first, |value: &u32| *value),
                Ok(21)
            );
            assert_eq!(first.slot, game.original.slot);
            assert_ne!(first.generation, game.original.generation);
            world.put(&game.write, first, 34).unwrap();
            replace_life(edits, &game.edit[1], first, SECOND);
            Ok(())
        },
    );
    let third = RegisteredSystem::direct(
        spec(3, vec![SystemId(2)]),
        |game: &Game, world: &mut WorldTxn<'_>, _, _, (), _| {
            let second = world.read().handle(SECOND).unwrap();
            assert_eq!(
                world
                    .read()
                    .with(&game.read[1], second, |value: &u32| *value),
                Ok(21)
            );
            assert!(world.read().handle(FIRST).is_none());
            assert!(world.read().handle(ORIGINAL).is_none());
            game.refusal.lock().unwrap().take().map_or(Ok(()), Err)
        },
    );
    let mut runtime = GameRuntime::new_published_with_rule_boundaries(
        WorldEpoch(1),
        1,
        registry,
        vec![first, second, third],
    )
    .unwrap();
    runtime
        .seed(ORIGINAL, |bundle| bundle.add(13_u32).map(|_| ()))
        .unwrap();
    let mut game = Game {
        edit: [SystemId(1), SystemId(2)]
            .map(|id| runtime.structural_permit(id, STRUCTURE).unwrap()),
        write: runtime.write_permit(SystemId(2)).unwrap(),
        read: [SystemId(2), SystemId(3)].map(|id| runtime.read_permit(id).unwrap()),
        original: runtime.handle(ORIGINAL).unwrap(),
        refusal: Mutex::new(None),
    };
    runtime.publish_initial(&mut game, &mut ());
    (runtime, game)
}

#[test]
fn later_rules_see_new_lives_and_refusal_restores_the_original() {
    let (mut runtime, mut game) = fixture();
    let original_digest = runtime.world_digest(&0_u32, |_, _| {}).unwrap();
    *game.refusal.get_mut().unwrap() = Some(TestError("later rule refused"));
    let mut undo = runtime.new_undo();
    assert!(matches!(
        runtime.step_recording(&mut game, &mut (), &mut undo, PublicationMode::Capture),
        Err(TickError::Game(_))
    ));
    assert_eq!(runtime.tick(), 0);
    assert_eq!(runtime.handle(ORIGINAL), Some(game.original));
    assert_eq!(runtime.world_digest(&0_u32, |_, _| {}), Ok(original_digest));
    assert_eq!(
        runtime
            .read()
            .with(&game.read[0], game.original, |value: &u32| *value),
        Ok(13)
    );
    runtime
        .step_recording(&mut game, &mut (), &mut undo, PublicationMode::Capture)
        .unwrap();
    assert_eq!(runtime.tick(), 1);
    let second = runtime.handle(SECOND).unwrap();
    assert_eq!(
        runtime
            .read()
            .with(&game.read[1], second, |value: &u32| *value),
        Ok(21)
    );
    runtime.rewind_many(&mut [undo]).unwrap();
    assert_eq!(runtime.handle(ORIGINAL), Some(game.original));
    assert_eq!(runtime.world_digest(&0_u32, |_, _| {}), Ok(original_digest));
}
