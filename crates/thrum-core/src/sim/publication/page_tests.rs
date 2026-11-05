use super::super::registered_world::{RegisteredWorld, WorldUndo};
use super::{PublicationCapture, PublicationPage};
use crate::{
    Handle, StableId, WorldEpoch,
    sim::{ComponentRegistry, ComponentSchemaId, SnapshotRing, StructuralChange},
};

fn fixture(capacity: usize) -> (RegisteredWorld, SnapshotRing<PublicationPage>) {
    let mut registry = ComponentRegistry::default();
    registry
        .register::<u32>(ComponentSchemaId(1))
        .expect("position schema");
    registry
        .register::<u64>(ComponentSchemaId(2))
        .expect("health schema");
    registry
        .select_publication::<u32>(true)
        .expect("temporal selection");
    registry
        .select_publication::<u64>(false)
        .expect("endpoint selection");
    let pages = SnapshotRing::new(capacity, 2, || {
        PublicationPage::new(&registry, WorldEpoch(1), capacity)
    });
    let mut world = RegisteredWorld::new(WorldEpoch(1), capacity, registry).expect("world");
    for id in 0..2 {
        world
            .spawn(StableId(id), |entity| {
                entity.add(1_u32)?.add(10_u64)?;
                Ok(())
            })
            .expect("seed");
    }
    (world, pages)
}

fn publish(
    pages: &mut SnapshotRing<PublicationPage>,
    world: &RegisteredWorld,
    tick: u64,
    timeline: u64,
    undo: Option<&WorldUndo>,
    events: &[StructuralChange],
) {
    pages
        .capture(|page, cold, page_dirty, outbound_dirty, source| {
            page.capture(
                world,
                PublicationCapture {
                    cold,
                    page_dirty,
                    outbound_dirty,
                    source,
                    tick,
                    timeline,
                    final_undo: undo,
                    final_events: events,
                },
            );
        })
        .expect("available page");
}

#[test]
fn cold_page_captures_checked_identity_and_generic_selected_columns() {
    let (world, mut pages) = fixture(65);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let page = pages.latest().expect("baseline");
    let handle = page.handle(0).expect("live handle");
    assert_eq!(page.component::<u32>(handle), Some(&1));
    assert_eq!(page.component::<u64>(handle), Some(&10));
    assert_eq!(page.component::<u16>(handle), None);
    assert_eq!(
        page.component::<u32>(Handle {
            generation: 2,
            ..handle
        }),
        None
    );
    assert_eq!(
        page.component::<u32>(Handle {
            epoch: WorldEpoch(2),
            ..handle
        }),
        None
    );
    assert_eq!(page.previous_id(), None);
    assert_eq!(page.previous_component::<u32>(handle), None);
    assert_eq!(page.dirty().component::<u32>(), &[0, 1]);
    assert_eq!(page.dirty().temporal::<u64>(), &[]);
    assert_eq!(page.dirty().structural(), &[0, 1]);
    assert_eq!(page.len(), 2);
    assert_eq!(page.slot_capacity(), 65);
    assert!(page.is_baseline());
    let mut visited = Vec::new();
    page.visit(|handle, id| visited.push((handle.slot, id)));
    assert_eq!(visited, [(0, StableId(0)), (1, StableId(1))]);
}

#[test]
fn held_pages_deferred_union_and_reverted_endpoint_keep_exact_temporal_pair() {
    let (mut world, mut pages) = fixture(128);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let first = pages.latest().expect("first retained page");
    let mut second = None;
    for tick in 1..=3 {
        let before =
            world.replace_prevalidated(0, 0, u32::try_from(tick + 1).expect("small value"));
        let mut undo = world.empty_undo();
        undo.record_component(0, 0, tick - 1, Some(before));
        pages.mark_component(0, 0);
        publish(&mut pages, &world, tick, 0, Some(&undo), &[]);
        if tick == 1 {
            second = pages.latest();
        }
    }
    assert!(!pages.can_capture());
    world.replace_prevalidated(0, 0, 5_u32);
    pages.mark_component(0, 0);
    world.replace_prevalidated(0, 0, 4_u32);
    pages.mark_component(0, 0);
    let mut final_undo = world.empty_undo();
    final_undo.record_component(0, 0, 4, Some(5_u32));
    assert!(
        pages
            .capture(|_, _, _, _, _| panic!("held page cannot be filled"))
            .is_err()
    );
    assert_eq!(
        first.component::<u32>(first.handle(0).expect("held identity")),
        Some(&1)
    );
    drop(first);
    publish(&mut pages, &world, 5, 0, Some(&final_undo), &[]);
    let page = pages.latest().expect("deferred publication");
    let handle = page.handle(0).expect("current identity");
    assert!(!page.is_baseline());
    assert_eq!(page.previous_id().expect("adjacent preimage tick").tick, 4);
    assert_eq!(page.component::<u32>(handle), Some(&4));
    assert_eq!(page.previous_component::<u32>(handle), Some(&5));
    assert_eq!(page.dirty().component::<u32>(), &[]);
    assert_eq!(page.dirty().temporal::<u32>(), &[0]);
    let second = second.expect("second lease remains immutable");
    assert_eq!(
        second.component::<u32>(second.handle(0).expect("held second identity")),
        Some(&2)
    );
}

#[test]
fn stopped_rows_reset_previous_when_a_warm_page_has_no_new_world_dirty_bits() {
    let (mut world, mut pages) = fixture(128);
    publish(&mut pages, &world, 0, 0, None, &[]);
    for tick in 1..=4 {
        let prior = world.replace_prevalidated(0, 0, u32::try_from(tick + 1).expect("small value"));
        let mut undo = world.empty_undo();
        undo.record_component(0, 0, tick - 1, Some(prior));
        pages.mark_component(0, 0);
        publish(&mut pages, &world, tick, 0, Some(&undo), &[]);
    }
    let empty = world.empty_undo();
    for tick in 5..=9 {
        publish(&mut pages, &world, tick, 0, Some(&empty), &[]);
        let page = pages.latest().expect("stopped endpoint");
        let handle = page.handle(0).expect("live stopped entity");
        assert_eq!(page.component::<u32>(handle), Some(&5));
        assert_eq!(page.previous_component::<u32>(handle), Some(&5));
        assert_eq!(page.dirty().component::<u32>(), &[]);
        assert_eq!(
            page.dirty().temporal::<u32>(),
            if tick == 5 { &[0][..] } else { &[] }
        );
    }
}

#[test]
fn generation_reuse_emits_ordered_lifecycle_even_when_component_values_match() {
    let (mut world, mut pages) = fixture(128);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let retained = pages.latest().expect("retained old entity");
    let old = retained.handle(0).expect("old handle");
    world.despawn(old).expect("retire generation");
    let new = world
        .spawn(StableId(99), |entity| {
            entity.add(1_u32)?.add(10_u64)?;
            Ok(())
        })
        .expect("reused slot");
    assert_eq!(old.slot, new.slot);
    let events = [StructuralChange::Despawn(old), StructuralChange::Spawn(new)];
    pages.mark_structural(new.slot);
    let empty = world.empty_undo();
    publish(&mut pages, &world, 1, 0, Some(&empty), &events);
    let page = pages.latest().expect("reused page");
    assert_eq!(page.structural_events(), &events);
    assert_eq!(page.dirty().structural(), &[0]);
    assert_eq!(page.dirty().component::<u32>(), &[0]);
    assert_eq!(page.component::<u32>(old), None);
    assert_eq!(page.component::<u32>(new), Some(&1));
    assert_eq!(page.previous_component::<u32>(new), None);
    assert!(!page.pair_valid(new.slot));
    assert_eq!(retained.component::<u32>(old), Some(&1));
}

#[test]
fn transient_identity_is_absent_from_net_lifecycle_and_component_diff() {
    let (mut world, mut pages) = fixture(128);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let transient = world
        .spawn(StableId(99), |entity| {
            entity.add(7_u32)?;
            Ok(())
        })
        .expect("transient spawn");
    world.despawn(transient).expect("transient despawn");
    pages.mark_structural(transient.slot);
    let events = [
        StructuralChange::Spawn(transient),
        StructuralChange::Despawn(transient),
    ];
    publish(&mut pages, &world, 2, 0, None, &events);
    let page = pages.latest().expect("net endpoint");
    assert!(!page.is_baseline());
    assert_eq!(page.structural_events(), &[]);
    assert_eq!(page.dirty().structural(), &[]);
    assert_eq!(page.dirty().component::<u32>(), &[]);
    assert_eq!(page.len(), 2);
    assert_eq!(page.previous_id(), None);
}

#[test]
fn warm_two_hundred_spawn_burst_preserves_page_reservations() {
    let (mut world, mut pages) = fixture(256);
    for tick in 0..4 {
        publish(&mut pages, &world, tick, 0, None, &[]);
    }
    let bytes = pages.reserved_bytes(PublicationPage::reserved_bytes);
    let mut events = Vec::new();
    for id in 2..202 {
        let handle = world
            .spawn(StableId(id), |entity| {
                entity.add(3_u32)?.add(20_u64)?;
                Ok(())
            })
            .expect("burst entity");
        events.push(StructuralChange::Spawn(handle));
        pages.mark_structural(handle.slot);
    }
    publish(&mut pages, &world, 4, 0, None, &events);
    assert_eq!(pages.reserved_bytes(PublicationPage::reserved_bytes), bytes);
    let page = pages.latest().expect("burst page");
    assert_eq!(page.len(), 202);
    assert_eq!(page.structural_events().len(), 200);
    assert_eq!(page.dirty().component::<u32>().len(), 200);
}

#[test]
fn sparse_inverse_patch_compares_selected_source_and_rejects_stale_generations() {
    let (mut world, mut pages) = fixture(128);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let mut undo = world.empty_undo();
    let before = world.replace_prevalidated(0, 0, 9_u32);
    undo.record_component(0, 0, 0, Some(before));
    pages.mark_component(0, 0);
    publish(&mut pages, &world, 1, 0, Some(&undo), &[]);
    let latest = pages.latest().expect("source page");
    assert_eq!(latest.source_tick(0, 0, 1), Some(1));
    assert_eq!(latest.source_tick(1, 0, 1), None);
    drop(latest);
    let mut inverse = Vec::new();
    undo.visit_changed_slots(|slot| inverse.push(slot));
    undo.restore(&mut world, &[]);
    pages.reset_timeline();
    pages
        .patch_rewind(
            |page| page.source_tick(0, 0, 1),
            |page, source| {
                let old = page.id();
                page.patch_changed(
                    &world,
                    |mark| {
                        for &slot in &inverse {
                            mark(slot);
                        }
                    },
                    0,
                    1,
                );
                assert_eq!(page.patch_source(), Some((old, 0)));
                assert_eq!(source, old.tick);
            },
        )
        .expect("unleased inverse source");
    let patched = pages.latest().expect("patched endpoint");
    assert_eq!(patched.id().tick, 0);
    assert_eq!(patched.timeline(), 1);
    assert_eq!(patched.previous_id(), None);
    assert_eq!(
        patched.component::<u32>(patched.handle(0).expect("identity")),
        Some(&1)
    );
}
