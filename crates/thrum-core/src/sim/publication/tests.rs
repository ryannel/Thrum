use super::super::registered_world::RegisteredWorld;
use super::{PublicationColumn, PublicationColumns, PublicationDiff};
use crate::{
    StableId, WorldEpoch,
    sim::{ComponentKey, ComponentRegistry, ComponentSchemaId, RegistrationError},
};

// Raw physical rows are only inspected by publication's internal tests.
impl<T> PublicationColumn<T> {
    pub(super) fn current(&self, row: u32) -> Option<&T> {
        self.current.get(row as usize)?.as_ref()
    }

    pub(super) fn previous(&self, row: u32) -> Option<&T> {
        self.previous_at(row as usize)
    }

    pub(super) fn current_values(&self) -> &[Option<T>] {
        &self.current
    }
}

impl PublicationDiff {
    fn component(&self, key: ComponentKey) -> Option<&[u64]> {
        self.columns
            .iter()
            .find_map(|(candidate, words)| (*candidate == key).then_some(words.as_slice()))
    }
}

fn refresh_component(
    page: &mut PublicationColumns,
    world: &RegisteredWorld,
    key: ComponentKey,
    slots: &[u32],
) {
    let mut words = vec![0; page.capacity.div_ceil(64)];
    for &slot in slots {
        words[slot as usize / 64] |= 1 << (slot % 64);
    }
    page.refresh_component_words(world, key, &words, &vec![0; words.len()]);
}

fn refresh(page: &mut PublicationColumns, world: &RegisteredWorld, slots: &[u32]) {
    for index in 0..page.selected.len() {
        refresh_component(page, world, page.selected[index].key, slots);
    }
}

fn diff_into(
    page: &PublicationColumns,
    baseline: &PublicationColumns,
    slots: &[u32],
    output: &mut PublicationDiff,
    pair: bool,
) {
    let mut candidates = vec![0_u64; page.capacity.div_ceil(64)];
    for &slot in slots {
        candidates[slot as usize / 64] |= 1 << (slot % 64);
    }
    for ((column, baseline), (_, words)) in page
        .selected
        .iter()
        .zip(&baseline.selected)
        .zip(&mut output.columns)
    {
        column.values.diff(
            baseline.values.as_ref(),
            &candidates,
            words,
            pair,
            None,
            (None, None),
        );
    }
}

fn endpoint_diff(
    page: &PublicationColumns,
    baseline: &PublicationColumns,
    slots: &[u32],
    output: &mut PublicationDiff,
) {
    diff_into(page, baseline, slots, output, false);
}
fn diff_pair(
    page: &PublicationColumns,
    baseline: &PublicationColumns,
    slots: &[u32],
    output: &mut PublicationDiff,
) {
    diff_into(page, baseline, slots, output, true);
}

fn fixture() -> (RegisteredWorld, PublicationColumns, PublicationColumns) {
    let mut registry = ComponentRegistry::default();
    registry
        .register::<u32>(ComponentSchemaId(1))
        .expect("first type");
    registry
        .register::<u64>(ComponentSchemaId(2))
        .expect("second type");
    registry
        .register_with_snapshot::<Vec<u8>>(ComponentSchemaId(3), Vec::clone, Vec::capacity)
        .expect("owned type");
    registry
        .select_publication::<u32>(true)
        .expect("temporal type");
    registry
        .select_publication::<u64>(false)
        .expect("current type");
    registry
        .select_publication::<Vec<u8>>(true)
        .expect("owned type");
    let mut page = PublicationColumns::new(&registry, 128);
    let mut retained = PublicationColumns::new(&registry, 128);
    let mut world = RegisteredWorld::new(WorldEpoch(1), 128, registry).expect("world capacity");
    for id in 0..2 {
        world
            .spawn(StableId(id), |entity| {
                entity.add(1_u32)?.add(10_u64)?.add(vec![1_u8, 2])?;
                Ok(())
            })
            .expect("fixture entity");
    }
    page.capture(&world);
    retained.capture(&world);
    (world, page, retained)
}

#[test]
fn selection_rejects_unknown_and_duplicate_types_without_changing_policy() {
    let mut registry = ComponentRegistry::default();
    assert_eq!(
        registry.select_publication::<u32>(true),
        Err(RegistrationError::UnregisteredType)
    );
    registry
        .register::<u32>(ComponentSchemaId(1))
        .expect("type registration");
    registry
        .select_publication::<u32>(true)
        .expect("first selection");
    assert_eq!(
        registry.select_publication::<u32>(false),
        Err(RegistrationError::DuplicatePublication)
    );
    let page = PublicationColumns::new(&registry, 1);
    assert!(
        page.column::<u32>()
            .expect("selected column")
            .supports_previous()
    );
    assert!(page.column::<u64>().is_none());
}

#[test]
fn selective_refresh_and_exact_diff_preserve_reverted_and_unselected_values() {
    let (mut world, mut page, retained) = fixture();
    world.replace_prevalidated(0, 0, 7_u32);
    world.replace_prevalidated(0, 0, 1_u32);
    world.replace_prevalidated(1, 1, 20_u64);
    world
        .spawn(StableId(2), |entity| {
            entity.add(3_u32)?.add(30_u64)?;
            Ok(())
        })
        .expect("new membership");
    refresh_component(&mut page, &world, ComponentKey::of::<u32>(), &[0, 1, 2]);
    assert_eq!(page.column::<u64>().expect("column").current(1), Some(&10));
    refresh(&mut page, &world, &[0, 1, 2]);
    let mut diff = PublicationDiff::new(&page);
    endpoint_diff(&page, &retained, &[0, 1, 2], &mut diff);
    assert_eq!(
        diff.component(ComponentKey::of::<u32>())
            .expect("u32 bitset")[0],
        1 << 2
    );
    assert_eq!(
        diff.component(ComponentKey::of::<u64>())
            .expect("u64 bitset")[0],
        (1 << 1) | (1 << 2)
    );
    assert_eq!(
        retained
            .column::<u64>()
            .expect("retained column")
            .current(1),
        Some(&10)
    );
    assert!(
        !page
            .column::<u64>()
            .expect("non temporal column")
            .supports_previous()
    );
    assert!(
        page.column::<u32>()
            .expect("typed column")
            .current(128)
            .is_none()
    );
    diff.clear();
    assert!(
        diff.component(ComponentKey::of::<u32>())
            .expect("cleared bitset")
            .iter()
            .all(|word| *word == 0)
    );

    world.remove_prevalidated::<u32>(0, 1);
    refresh(&mut page, &world, &[1]);
    endpoint_diff(&page, &retained, &[1], &mut diff);
    assert_eq!(
        diff.component(ComponentKey::of::<u32>())
            .expect("removed component")[0],
        1 << 1
    );
    assert!(
        page.column::<u32>()
            .expect("current membership")
            .current(1)
            .is_none()
    );
}

#[test]
fn bitset_refresh_unions_component_and_structural_slots() {
    let (mut world, mut page, retained) = fixture();
    world.replace_prevalidated(0, 0, 7_u32);
    world.replace_prevalidated(0, 1, 8_u32);
    world
        .spawn(StableId(2), |entity| {
            entity.add(9_u32)?.add(30_u64)?;
            Ok(())
        })
        .expect("spawned structural change");
    page.refresh_component_words(&world, ComponentKey::of::<u32>(), &[1, 0], &[1 << 2, 0]);
    let column = page.column::<u32>().expect("typed component");
    assert_eq!(column.current(0), Some(&7));
    assert_eq!(column.current(1), Some(&1));
    assert_eq!(column.current(2), Some(&9));
    assert_eq!(
        retained
            .column::<u32>()
            .expect("retained component")
            .current(0),
        Some(&1)
    );
    assert_eq!(
        page.column::<u64>().expect("unselected refresh").current(2),
        None
    );
}

#[test]
fn temporal_pair_uses_final_tick_first_preimages_after_deferred_ticks() {
    let (mut world, mut page, retained) = fixture();
    world.replace_prevalidated(0, 0, 7_u32);
    let mut undo = world.empty_undo();
    undo.record_component(0, 0, 1, Some(7_u32));
    undo.record_component(0, 0, 2, Some(8_u32));
    world.replace_prevalidated(0, 0, 9_u32);
    refresh(&mut page, &world, &[0]);
    page.reset_previous(&[0]);
    page.apply_previous(&undo, &mut [0; 2]);
    let column = page.column::<u32>().expect("temporal column");
    assert_eq!(column.current(0), Some(&9));
    assert_eq!(column.previous(0), Some(&7));
    assert_eq!(
        retained
            .column::<u32>()
            .expect("immutable retained column")
            .current(0),
        Some(&1)
    );

    page.reset_previous(&[0]);
    page.apply_previous(&world.empty_undo(), &mut [0; 2]);
    assert_eq!(
        page.column::<u32>().expect("stopped component").previous(0),
        Some(&9)
    );
}

#[test]
fn pair_diff_reports_stopped_and_oscillating_previous_values_independently() {
    let (mut world, mut page, mut retained) = fixture();
    let mut undo = world.empty_undo();
    undo.record_component(0, 0, 0, Some(1_u32));
    world.replace_prevalidated(0, 0, 9_u32);
    for page in [&mut page, &mut retained] {
        refresh(page, &world, &[0]);
        page.reset_previous(&[0]);
        page.apply_previous(&undo, &mut [0; 2]);
    }
    let mut endpoint = PublicationDiff::new(&page);
    let mut pair = PublicationDiff::new(&page);
    page.reset_previous(&[0]);
    endpoint_diff(&page, &retained, &[0], &mut endpoint);
    diff_pair(&page, &retained, &[0], &mut pair);
    assert_eq!(
        endpoint
            .component(ComponentKey::of::<u32>())
            .expect("unchanged endpoint")[0],
        0
    );
    assert_eq!(
        pair.component(ComponentKey::of::<u32>())
            .expect("stopped pair")[0],
        1
    );
    assert!(
        pair.component(ComponentKey::of::<u64>())
            .expect("non temporal type")
            .iter()
            .all(|word| *word == 0)
    );

    undo.clear();
    undo.record_component(0, 0, 1, Some(7_u32));
    page.apply_previous(&undo, &mut [0; 2]);
    pair.clear();
    diff_pair(&page, &retained, &[0], &mut pair);
    assert_eq!(
        page.column::<u32>()
            .expect("oscillating current")
            .current(0),
        Some(&9)
    );
    assert_eq!(
        page.column::<u32>().expect("oscillating prior").previous(0),
        Some(&7)
    );
    assert_eq!(
        pair.component(ComponentKey::of::<u32>())
            .expect("oscillating pair")[0],
        1
    );
}

#[test]
fn registered_snapshot_callback_keeps_nested_page_values_independent() {
    let (mut world, mut page, retained) = fixture();
    let mut prior = world.replace_prevalidated(2, 0, vec![9_u8]);
    prior[0] = 42;
    refresh_component(&mut page, &world, ComponentKey::of::<Vec<u8>>(), &[0]);
    let mut undo = world.empty_undo();
    undo.record_component(2, 0, 0, Some(prior.clone()));
    page.reset_previous(&[0]);
    page.apply_previous(&undo, &mut [0; 2]);
    prior[0] = 99;
    let column = page.column::<Vec<u8>>().expect("owned temporal type");
    assert_eq!(column.current(0), Some(&vec![9]));
    assert_eq!(column.previous(0), Some(&vec![42, 2]));
    assert_eq!(
        retained
            .column::<Vec<u8>>()
            .expect("retained owned type")
            .current(0),
        Some(&vec![1, 2])
    );
    assert!(page.reserved_bytes() >= 128 * size_of::<Option<Vec<u8>>>());
}

#[test]
fn fused_capture_matches_separate_refresh_and_exact_pair_oracle() {
    let (mut world, mut page, mut source) = fixture();
    for id in 2..67 {
        world
            .spawn(StableId(id), |entity| {
                entity.add(1_u32)?.add(10_u64)?.add(vec![1_u8, 2])?;
                Ok(())
            })
            .expect("cross-word fixture");
    }
    page.capture(&world);
    source.capture(&world);
    let mut reference = PublicationColumns::new(world.registry(), 128);
    reference.capture(&world);
    let mut source_undo = world.empty_undo();
    source_undo.record_component(0, 0, 0, Some(0_u32));
    source.apply_previous(&source_undo, &mut [0; 2]);
    let mut stale_undo = world.empty_undo();
    stale_undo.record_component(0, 1, 0, Some(99_u32));
    page.apply_previous(&stale_undo, &mut [0; 2]);
    reference.apply_previous(&stale_undo, &mut [0; 2]);

    world.replace_prevalidated(0, 1, 8_u32);
    refresh_component(&mut page, &world, ComponentKey::of::<u32>(), &[1]);
    refresh_component(&mut reference, &world, ComponentKey::of::<u32>(), &[1]);
    world.replace_prevalidated(0, 2, 20_u32);
    refresh_component(&mut source, &world, ComponentKey::of::<u32>(), &[2]);
    world.remove_prevalidated::<u32>(0, 65);
    world.replace_prevalidated(2, 64, vec![7_u8, 8]);
    let mut final_undo = world.empty_undo();
    final_undo.record_component(0, 0, 0, Some(3_u32));
    final_undo.record_component(0, 0, 0, Some(4_u32));
    final_undo.record_component(0, 65, 0, Some(1_u32));
    final_undo.record_component(0, 66, 0, Some(7_u32));
    final_undo.record_component(2, 64, 0, Some(vec![9_u8]));

    let pending = [[1 << 2, 1 << 1], [0, 0], [0, 1]];
    let outbound = [[1 << 1, 1 << 1], [0, 0], [0, 1]];
    let structural = [0, 1 << 2];
    for (index, words) in pending.iter().enumerate() {
        let key = reference.selected[index].key;
        reference.refresh_component_words(&world, key, words, &structural);
    }
    reference.apply_previous(&final_undo, &mut [0; 2]);
    reference.reset_previous(&[66]);
    let pair_valid = [u64::MAX, !(1 << 2)];
    for index in 0..page.selected.len() {
        let mut changed = super::ChangedColumn {
            key: page.selected[index].key,
            slots: Vec::with_capacity(128),
            temporal: Vec::with_capacity(128),
        };
        let mut endpoint = [0; 2];
        let mut pair = [0; 2];
        let mut candidates = [
            pending[index][0] | outbound[index][0],
            pending[index][1] | outbound[index][1] | structural[1],
        ];
        for &slot in source.selected[index]
            .values
            .prior_slots()
            .iter()
            .chain(reference.selected[index].values.prior_slots())
        {
            candidates[slot as usize / 64] |= 1 << (slot % 64);
        }
        let mut expected_endpoint = [0; 2];
        let mut expected_pair = [0; 2];
        reference.selected[index].values.diff(
            source.selected[index].values.as_ref(),
            &candidates,
            &mut expected_endpoint,
            false,
            Some(&structural),
            (None, None),
        );
        reference.selected[index].values.diff(
            source.selected[index].values.as_ref(),
            &candidates,
            &mut expected_pair,
            true,
            Some(&structural),
            (None, None),
        );
        let component = page.selected[index].component;
        page.selected[index]
            .values
            .capture_warm(super::WarmColumnCapture {
                world: &world,
                component,
                source: source.selected[index].values.as_ref(),
                directory: None,
                source_directory: None,
                page_matches_world: true,
                page_matches_source: true,
                page_words: &pending[index],
                page_structural: &structural,
                outbound_words: &outbound[index],
                outbound_structural: &structural,
                identities: &structural,
                pair_valid: &pair_valid,
                final_undo: Some(&final_undo),
                scratch: &mut [0; 2],
                endpoint: &mut endpoint,
                pair: &mut pair,
                changed: &mut changed,
                parallel: false,
            });
        assert_eq!(endpoint, expected_endpoint);
        assert_eq!(pair, expected_pair);
        assert_eq!(
            changed.slots,
            super::dirty_slots(&endpoint, &endpoint, 128).collect::<Vec<_>>()
        );
        assert_eq!(
            changed.temporal,
            super::dirty_slots(&pair, &pair, 128).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        page.column::<u32>()
            .expect("fused current")
            .current_values(),
        reference
            .column::<u32>()
            .expect("oracle current")
            .current_values()
    );
    assert_eq!(
        page.column::<u32>().expect("oscillating pair").previous(0),
        Some(&3)
    );
    assert_eq!(
        page.column::<u32>().expect("generation reset").previous(66),
        Some(&1)
    );
    assert_eq!(
        page.column::<Vec<u8>>()
            .expect("nested current")
            .current(64),
        Some(&vec![7, 8])
    );
    assert_eq!(
        page.column::<Vec<u8>>().expect("nested prior").previous(64),
        Some(&vec![9])
    );
}

#[test]
fn source_endpoint_tracks_same_timeline_and_retired_publications() {
    use super::{PublicationCapture, PublicationPage};
    use crate::sim::SnapshotRing;
    let mut registry = ComponentRegistry::default();
    registry
        .register::<u32>(ComponentSchemaId(1))
        .expect("schema");
    registry.select_publication::<u32>(true).expect("selection");
    let mut pages = SnapshotRing::new(1, 1, || PublicationPage::new(&registry, WorldEpoch(1), 1));
    let world = RegisteredWorld::new(WorldEpoch(1), 1, registry).expect("empty world");
    for (tick, timeline) in [(0, 0), (1, 0), (2, 1)] {
        if timeline == 1 {
            pages.reset_timeline();
        }
        pages
            .capture(|page, cold, page_dirty, outbound_dirty, source| {
                page.capture(
                    &world,
                    PublicationCapture {
                        cold,
                        page_dirty,
                        outbound_dirty,
                        source,
                        tick,
                        timeline,
                        final_undo: None,
                        final_events: &[],
                        parallel: false,
                    },
                );
            })
            .expect("page available");
        let page = pages.latest().expect("published endpoint");
        assert_eq!(
            page.source_endpoint()
                .map(|(id, timeline)| (id.tick, timeline)),
            match tick {
                0 => None,
                1 => Some((0, 0)),
                _ => Some((1, 0)),
            }
        );
        assert_eq!(page.patch_source().is_some(), timeline == 1);
    }
}
