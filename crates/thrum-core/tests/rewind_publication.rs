//! Compare complete selected endpoint and adjacent-pose bytes with a fresh
//! engine capture after corrections inside and across deferred publication.

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
        EditWriter, GameRuntime, PublicationMode, PublicationPage, RegisteredSystem, Stage,
        SystemId, SystemSpec, TickGame, WorldRead, WritePermit,
    },
};

#[derive(Clone, Copy, PartialEq)]
struct Position(u64);
#[derive(Clone, Copy, PartialEq)]
struct Health(u32);

struct Game {
    handles: Vec<Handle>,
    position: WritePermit<Position>,
    health: WritePermit<Health>,
    correction: u64,
}

impl TickGame for Game {
    type Error = ();
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
        .register::<Position>(ComponentSchemaId(1))
        .expect("position");
    registry
        .register::<Health>(ComponentSchemaId(2))
        .expect("health");
    registry
        .select_publication::<Position>(true)
        .expect("temporal position");
    registry
        .select_publication::<Health>(false)
        .expect("endpoint health");
    let system = RegisteredSystem::direct(
        SystemSpec {
            id: SystemId(1),
            stage: Stage::Simulate,
            accesses: vec![ComponentKey::of::<Position>(), ComponentKey::of::<Health>()]
                .into_iter()
                .map(|key| Access {
                    key: AccessKey::Component(key),
                    mode: AccessMode::Write,
                })
                .collect(),
            after: vec![],
        },
        |game: &Game, world, _, tick, (), _| {
            for (id, &handle) in game.handles.iter().enumerate() {
                world
                    .put(
                        &game.position,
                        handle,
                        Position(
                            tick * 17 + game.correction + u64::try_from(id).expect("small id"),
                        ),
                    )
                    .expect("position write");
                world
                    .put(
                        &game.health,
                        handle,
                        Health(u32::try_from(tick).expect("short run") + 80),
                    )
                    .expect("health write");
            }
            Ok(())
        },
    );
    let mut runtime =
        GameRuntime::new_published(WorldEpoch(7), 8, registry, vec![system]).expect("runtime");
    for id in 0..3 {
        runtime
            .seed(StableId(id), |entity| {
                entity.add(Position(0))?.add(Health(80))?;
                Ok(())
            })
            .expect("seed");
    }
    let mut game = Game {
        handles: (0..3)
            .map(|id| runtime.handle(StableId(id)).expect("seeded identity"))
            .collect(),
        position: runtime.write_permit(SystemId(1)).expect("position permit"),
        health: runtime.write_permit(SystemId(1)).expect("health permit"),
        correction: 0,
    };
    runtime.publish_initial(&mut game, &mut ());
    (runtime, game)
}

/// Canonical complete page payload, without process addresses, padding or
/// publication-history provenance. Every slot, identity, selected component
/// and temporal value is encoded, including absent slots and columns.
fn bytes(page: &PublicationPage) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(page.id().epoch.0.to_le_bytes());
    out.extend(page.id().tick.to_le_bytes());
    out.extend(page.timeline().to_le_bytes());
    out.extend(u64::try_from(page.len()).expect("len").to_le_bytes());
    out.extend(
        page.previous_id()
            .expect("adjacent tick")
            .tick
            .to_le_bytes(),
    );
    for slot in 0..u32::try_from(page.slot_capacity()).expect("capacity") {
        let handle = page.handle(slot);
        out.push(u8::from(handle.is_some()));
        if let Some(handle) = handle {
            encode_handle(&mut out, handle);
            out.extend(page.stable_id(slot).expect("identity").0.to_le_bytes());
            out.extend(
                page.component::<Position>(handle)
                    .expect("position")
                    .0
                    .to_le_bytes(),
            );
            out.extend(
                page.component::<Health>(handle)
                    .expect("health")
                    .0
                    .to_le_bytes(),
            );
            out.push(u8::from(page.pair_valid(slot)));
            out.extend(
                page.previous_component::<Position>(handle)
                    .expect("previous position")
                    .0
                    .to_le_bytes(),
            );
            assert!(page.previous_component::<Health>(handle).is_none());
        }
    }
    out
}

fn encode_handle(out: &mut Vec<u8>, handle: Handle) {
    out.extend(handle.epoch.0.to_le_bytes());
    out.extend(handle.slot.to_le_bytes());
    out.extend(handle.generation.to_le_bytes());
}

fn corrected(target: usize, latest: usize) {
    let (mut runtime, mut game) = fixture();
    let mut records = Vec::new();
    for tick in 1..=latest {
        let mut undo = runtime.new_undo();
        runtime
            .step_recording(
                &mut game,
                &mut (),
                &mut undo,
                if (2..=4).contains(&tick) {
                    PublicationMode::Defer
                } else {
                    PublicationMode::Capture
                },
            )
            .expect("tick");
        records.push(undo);
    }
    let held = runtime.latest().expect("old branch");
    let held_bytes = bytes(&held);
    assert_eq!(runtime.has_current_publication(), latest == 6);
    runtime.rewind_many(&mut records[target..]).expect("rewind");
    game.correction = 1_000;
    for tick in target + 1..=latest {
        let mut undo = runtime.new_undo();
        runtime
            .step_recording(&mut game, &mut (), &mut undo, PublicationMode::Defer)
            .expect("replay");
        assert_eq!(runtime.tick(), u64::try_from(tick).expect("tick"));
    }
    assert!(!runtime.has_current_publication());
    assert!(runtime.publish_current_if_available(&mut game, &mut ()));
    let page = runtime.latest().expect("corrected endpoint");

    // A separate runtime reconstructs the corrected world directly. Its
    // first unused ring page is a cold capture, independent of dirty unions
    // and all pages reused during the rewound runtime's replay.
    let (mut reference, mut reference_game) = fixture();
    let mut branch = reference.new_undo();
    reference
        .step_recording(
            &mut reference_game,
            &mut (),
            &mut branch,
            PublicationMode::Defer,
        )
        .expect("reference branch");
    reference
        .rewind_many(std::slice::from_mut(&mut branch))
        .expect("same timeline generation");
    for tick in 1..=latest {
        reference_game.correction = if tick > target { 1_000 } else { 0 };
        let mut undo = reference.new_undo();
        reference
            .step_recording(
                &mut reference_game,
                &mut (),
                &mut undo,
                if tick == latest {
                    PublicationMode::Capture
                } else {
                    PublicationMode::Defer
                },
            )
            .expect("reference tick");
    }
    let fresh = reference.latest().expect("fresh endpoint");
    assert_eq!(
        bytes(&page),
        bytes(&fresh),
        "whole selected page payload, byte for byte"
    );
    assert_eq!(page.source_endpoint().expect("replay source").0, held.id());
    assert_eq!(page.patch_source(), page.source_endpoint());
    assert!(!page.is_baseline());
    assert_eq!(page.structural_events(), []);
    assert_eq!(page.dirty().component::<Position>(), &[0, 1, 2]);
    assert_eq!(page.dirty().temporal::<Position>(), &[0, 1, 2]);
    assert_eq!(
        page.dirty().component::<Health>(),
        if latest == 6 { &[][..] } else { &[0, 1, 2][..] }
    );
    assert_eq!(page.dirty().structural(), []);
    assert_eq!(bytes(&held), held_bytes, "old leases stay immutable");
}

#[test]
fn correction_inside_deferred_publication() {
    corrected(3, 4);
}
#[test]
fn correction_across_deferred_publication() {
    corrected(1, 6);
}
