//! Count bytes copied by the registered snapshot function during deferred replay.

use std::cell::Cell;
use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
        EditWriter, GameRuntime, PublicationMode, RegisteredSystem, Stage, SystemId, SystemSpec,
        TickGame, WorldRead, WritePermit,
    },
};

#[derive(Clone, PartialEq)]
struct Pose([u64; 8]);

thread_local! {
    static COPIED: Cell<usize> = const { Cell::new(0) };
}

struct Game {
    handle: Handle,
    pose: WritePermit<Pose>,
    refuse: bool,
}

impl TickGame for Game {
    type Error = std::io::Error;
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

fn fixture() -> (GameRuntime<Game>, Game) {
    let mut registry = ComponentRegistry::default();
    registry
        .register_with_snapshot(
            ComponentSchemaId(1),
            |pose: &Pose| {
                COPIED.update(|bytes| bytes + size_of::<Pose>());
                pose.clone()
            },
            |_| 0,
        )
        .expect("pose registration");
    registry
        .select_publication::<Pose>(true)
        .expect("temporal pose");
    let system = RegisteredSystem::direct(
        SystemSpec {
            id: SystemId(1),
            stage: Stage::Simulate,
            accesses: vec![Access {
                key: AccessKey::Component(ComponentKey::of::<Pose>()),
                mode: AccessMode::Write,
            }],
            after: vec![],
        },
        |game: &Game, world, _, tick, (), _| {
            world
                .put(&game.pose, game.handle, Pose([tick; 8]))
                .expect("pose update");
            if game.refuse {
                Err(std::io::Error::other("refused"))
            } else {
                Ok(())
            }
        },
    );
    let mut runtime =
        GameRuntime::new_published(WorldEpoch(1), 1, registry, vec![system]).expect("runtime");
    runtime
        .seed(StableId(0), |entity| {
            entity.add(Pose([0; 8]))?;
            Ok(())
        })
        .expect("seed");
    let mut game = Game {
        handle: runtime.handle(StableId(0)).expect("seeded handle"),
        pose: runtime.write_permit(SystemId(1)).expect("write permit"),
        refuse: false,
    };
    runtime.publish_initial(&mut game, &mut ());
    (runtime, game)
}

#[test]
fn undo_copy_bytes_do_not_grow_with_replay_depth() {
    let totals = [1, 8, 32].map(|depth| {
        let (mut runtime, mut game) = fixture();
        let mut records: Vec<_> = (0..depth).map(|_| runtime.new_undo()).collect();
        for record in &mut records {
            runtime
                .step_recording(&mut game, &mut (), record, PublicationMode::Capture)
                .expect("original tick");
        }
        runtime.rewind_many(&mut records).expect("rewind");
        COPIED.set(0);
        for record in &mut records {
            runtime
                .step_recording(&mut game, &mut (), record, PublicationMode::Defer)
                .expect("replay tick");
        }
        let copied = COPIED.get();
        println!("deferred replay depth {depth}: {copied} component preimage bytes copied");
        assert!(runtime.publish_current_if_available(&mut game, &mut ()));
        let page = runtime.latest().expect("replayed endpoint");
        let handle = page.handle(0).expect("published handle");
        assert_eq!(page.component::<Pose>(handle).unwrap().0, [depth; 8]);
        assert_eq!(
            page.previous_component::<Pose>(handle).unwrap().0,
            [depth - 1; 8]
        );
        copied
    });
    assert_eq!(totals, [0, 0, 0]);
}

#[test]
fn refusing_to_reuse_a_deferred_record_keeps_it_available_for_publish_and_rewind() {
    let (mut runtime, mut game) = fixture();
    let mut record = runtime.new_undo();
    runtime
        .step_recording(&mut game, &mut (), &mut record, PublicationMode::Defer)
        .expect("deferred tick");
    game.refuse = true;
    assert!(
        runtime
            .step_recording(&mut game, &mut (), &mut record, PublicationMode::Defer)
            .is_err()
    );
    assert!(runtime.publish_current_if_available(&mut game, &mut ()));
    let page = runtime.latest().expect("deferred endpoint");
    let handle = page.handle(0).expect("published handle");
    assert_eq!(page.component::<Pose>(handle).unwrap().0, [1; 8]);
    assert_eq!(page.previous_component::<Pose>(handle).unwrap().0, [0; 8]);
    runtime
        .rewind_many(std::slice::from_mut(&mut record))
        .expect("rewind");
    assert!(runtime.publish_current_if_available(&mut game, &mut ()));
    let page = runtime.latest().expect("rewound endpoint");
    assert_eq!(page.component::<Pose>(handle).unwrap().0, [0; 8]);
}
