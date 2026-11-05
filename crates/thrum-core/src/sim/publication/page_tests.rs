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

fn partitioned_fixture(capacity: usize) -> (RegisteredWorld, SnapshotRing<PublicationPage>) {
    let mut registry = ComponentRegistry::default();
    registry
        .register::<u32>(ComponentSchemaId(1))
        .expect("position schema");
    registry
        .register_indexed::<u64>(ComponentSchemaId(2), |value| *value != 0)
        .expect("activity schema");
    registry
        .select_activity_partition::<u64>()
        .expect("activity partition");
    registry
        .select_publication::<u32>(true)
        .expect("temporal position");
    registry
        .select_publication::<u64>(false)
        .expect("activity publication");
    let pages = SnapshotRing::new(capacity, 2, || {
        PublicationPage::new(&registry, WorldEpoch(1), capacity)
    });
    let mut world = RegisteredWorld::new(WorldEpoch(1), capacity, registry).expect("world");
    for slot in 0..4_u32 {
        world
            .spawn(StableId(u128::from(slot)), |entity| {
                entity.add(slot + 10)?.add(u64::from(slot % 2))?;
                Ok(())
            })
            .expect("interleaved entity");
    }
    (world, pages)
}

fn change_partitioned(
    world: &mut RegisteredWorld,
    pages: &mut SnapshotRing<PublicationPage>,
    tick: u64,
    changes: &[(u32, u64, u32)],
) -> WorldUndo {
    world.begin_tick(Some(tick));
    let mut undo = world.empty_undo();
    for &(slot, active, position) in changes {
        let handle = world.live_at_slot(slot).expect("changed live identity").0;
        let old_activity = world.replace_prevalidated(1, slot, active);
        let old_position = world.replace_prevalidated(0, slot, position);
        undo.record_component(1, slot, tick - 1, Some(old_activity));
        undo.record_component(0, slot, tick - 1, Some(old_position));
        world.mark_changed(1, handle, tick);
        world.mark_changed(0, handle, tick);
        pages.mark_component(1, slot);
        pages.mark_component(0, slot);
    }
    world.finalize_activity(&[], &mut undo);
    world.finish_tick(tick);
    undo
}

fn assert_partitioned_values(page: &PublicationPage, positions: &[u32; 4], active: &[u64; 4]) {
    let directory = page.columns.directory.as_ref().expect("page directory");
    assert_eq!(
        directory.active_len(),
        active.iter().filter(|&&value| value != 0).count()
    );
    for slot in 0..4_u32 {
        let handle = page.handle(slot).expect("published identity");
        assert_eq!(
            page.component::<u32>(handle),
            Some(&positions[slot as usize])
        );
        assert_eq!(page.component::<u64>(handle), Some(&active[slot as usize]));
        assert_eq!(
            directory.row_of(slot as usize) < directory.active_len(),
            active[slot as usize] != 0
        );
    }
}

#[test]
fn partitioned_pages_keep_held_values_and_exact_pairs_across_independent_layouts() {
    let (mut world, mut pages) = partitioned_fixture(8);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let held = pages.latest().expect("held baseline");
    assert_partitioned_values(&held, &[10, 11, 12, 13], &[0, 1, 0, 1]);
    let bytes = pages.reserved_bytes(PublicationPage::reserved_bytes);
    let mut positions = [10, 11, 12, 13];
    let mut activity = [0, 1, 0, 1];
    let mut prior_pair = positions;
    let mut source_layout_differed = false;
    let mut authority_layout_differed = false;
    for tick in 1..=8 {
        let old_positions = positions;
        let old_activity = activity;
        // Authority applies these changes in reverse slot order. A warm page
        // partitions its own rows in logical order, producing a different tail.
        let moving = tick % 2 == 0;
        activity[0] = u64::from(!moving);
        activity[3] = u64::from(moving);
        positions[0] += 2;
        positions[3] += 3;
        let undo = change_partitioned(
            &mut world,
            &mut pages,
            tick,
            &[
                (3, activity[3], positions[3]),
                (0, activity[0], positions[0]),
            ],
        );
        let source = pages.latest().expect("source partitioned page");
        publish(&mut pages, &world, tick, 0, Some(&undo), &[]);
        let page = pages.latest().expect("current partitioned page");
        let directory = page.columns.directory.as_ref().expect("page directory");
        let source_directory = source.columns.directory.as_ref().expect("source directory");
        source_layout_differed |=
            (0..8).any(|row| directory.slot_at(row) != source_directory.slot_at(row));
        authority_layout_differed |=
            (0..8).any(|row| directory.slot_at(row) != world.slot_at_row(row));
        assert_partitioned_values(&page, &positions, &activity);
        let expected_endpoint: Vec<_> = (0..4_u32)
            .filter(|&slot| positions[slot as usize] != old_positions[slot as usize])
            .collect();
        let expected_pair: Vec<_> = (0..4_u32)
            .filter(|&slot| {
                positions[slot as usize] != old_positions[slot as usize]
                    || old_positions[slot as usize] != prior_pair[slot as usize]
            })
            .collect();
        let expected_activity: Vec<_> = (0..4_u32)
            .filter(|&slot| activity[slot as usize] != old_activity[slot as usize])
            .collect();
        assert_eq!(page.dirty().component::<u32>(), expected_endpoint);
        assert_eq!(page.dirty().temporal::<u32>(), expected_pair);
        assert_eq!(page.dirty().component::<u64>(), expected_activity);
        for slot in 0..4_u32 {
            assert_eq!(
                page.previous_component::<u32>(page.handle(slot).expect("current handle")),
                Some(&old_positions[slot as usize])
            );
        }
        assert_partitioned_values(&held, &[10, 11, 12, 13], &[0, 1, 0, 1]);
        prior_pair = old_positions;
    }
    assert!(source_layout_differed);
    assert!(authority_layout_differed);
    let empty = world.empty_undo();
    for tick in 9..=12 {
        publish(&mut pages, &world, tick, 0, Some(&empty), &[]);
        let page = pages.latest().expect("stopped partitioned page");
        assert_partitioned_values(&page, &positions, &activity);
        assert_eq!(page.dirty().component::<u32>(), &[]);
        assert_eq!(
            page.dirty().temporal::<u32>(),
            if tick == 9 { &[0, 3][..] } else { &[] }
        );
        for slot in 0..4_u32 {
            assert_eq!(
                page.previous_component::<u32>(page.handle(slot).expect("stopped handle")),
                Some(&positions[slot as usize])
            );
        }
    }
    assert_eq!(pages.reserved_bytes(PublicationPage::reserved_bytes), bytes);
}

#[test]
fn partitioned_rewind_swaps_neighbors_without_reporting_layout_changes() {
    let (mut world, mut pages) = partitioned_fixture(8);
    publish(&mut pages, &world, 0, 0, None, &[]);
    let mut undo = change_partitioned(&mut world, &mut pages, 1, &[(3, 0, 33), (0, 1, 20)]);
    publish(&mut pages, &world, 1, 0, Some(&undo), &[]);
    let held = pages.latest().expect("held changed endpoint");
    publish(&mut pages, &world, 2, 0, None, &[]);
    let mut inverse = Vec::new();
    undo.visit_changed_slots(|slot| inverse.push(slot));
    undo.restore(&mut world, &[]);
    drop(held);
    pages.reset_timeline();
    pages
        .patch_rewind(
            |page| page.source_tick(0, 1, 1),
            |page, _| {
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
            },
        )
        .expect("rewind page");
    let page = pages.latest().expect("restored publication");
    assert_partitioned_values(&page, &[10, 11, 12, 13], &[0, 1, 0, 1]);
    assert_eq!(page.dirty().component::<u32>(), &[0, 3]);
    assert_eq!(page.dirty().component::<u64>(), &[0, 3]);
    assert_eq!(page.dirty().structural(), &[]);
    assert_eq!(page.previous_id(), None);
    assert_eq!(page.source_endpoint().expect("patch source").0.tick, 1);
}

#[test]
fn partitioned_boundary_deletion_and_reuse_preserve_neighbor_identity() {
    let (mut world, mut pages) = partitioned_fixture(8);
    for tick in 0..4 {
        publish(&mut pages, &world, tick, 0, None, &[]);
    }
    let held = pages.latest().expect("held original endpoint");
    let old = held.handle(1).expect("active original identity");
    world.despawn(old).expect("delete active boundary neighbor");
    let new = world
        .spawn(StableId(99), |entity| {
            entity.add(51_u32)?.add(1_u64)?;
            Ok(())
        })
        .expect("reuse active slot");
    pages.mark_structural(old.slot);
    let events = [StructuralChange::Despawn(old), StructuralChange::Spawn(new)];
    let undo = world.empty_undo();
    publish(&mut pages, &world, 4, 0, Some(&undo), &events);
    let page = pages.latest().expect("reused endpoint");
    assert_partitioned_values(&page, &[10, 51, 12, 13], &[0, 1, 0, 1]);
    assert_eq!(page.structural_events(), events);
    assert_eq!(page.dirty().component::<u32>(), &[1]);
    assert_eq!(page.dirty().component::<u64>(), &[1]);
    assert_eq!(page.component::<u32>(old), None);
    assert_eq!(page.previous_component::<u32>(new), None);
    assert_partitioned_values(&held, &[10, 11, 12, 13], &[0, 1, 0, 1]);
}

#[test]
fn partitioned_warm_two_hundred_spawns_keep_reservations_and_existing_neighbors() {
    let (mut world, mut pages) = partitioned_fixture(256);
    for tick in 0..4 {
        publish(&mut pages, &world, tick, 0, None, &[]);
    }
    let bytes = pages.reserved_bytes(PublicationPage::reserved_bytes);
    let mut events = Vec::with_capacity(200);
    for slot in 4..204_u32 {
        let handle = world
            .spawn(StableId(u128::from(slot)), |entity| {
                entity.add(slot + 10)?.add(u64::from(slot % 2))?;
                Ok(())
            })
            .expect("partitioned burst entity");
        pages.mark_structural(handle.slot);
        events.push(StructuralChange::Spawn(handle));
    }
    let undo = world.empty_undo();
    publish(&mut pages, &world, 4, 0, Some(&undo), &events);
    let page = pages.latest().expect("partitioned burst publication");
    assert_eq!(page.len(), 204);
    assert_eq!(page.structural_events(), events);
    assert_eq!(page.dirty().component::<u32>().len(), 200);
    assert_eq!(page.dirty().component::<u64>().len(), 200);
    let directory = page.columns.directory.as_ref().expect("burst directory");
    assert_eq!(directory.active_len(), 102);
    for slot in 0..204_u32 {
        let handle = page.handle(slot).expect("burst identity");
        assert_eq!(page.component::<u32>(handle), Some(&(slot + 10)));
        assert_eq!(page.component::<u64>(handle), Some(&u64::from(slot % 2)));
        if slot < 4 {
            assert_eq!(page.previous_component::<u32>(handle), Some(&(slot + 10)));
        } else {
            assert_eq!(page.previous_component::<u32>(handle), None);
        }
    }
    assert_eq!(pages.reserved_bytes(PublicationPage::reserved_bytes), bytes);
}
