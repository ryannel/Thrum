//! A portable world crosses epochs without copying runtime-local history.

mod fixture;

use fixture::{IDENTITY, LIMIT, digest, image, runtime, source};
use thrum_core::StableId;

fn take<const N: usize>(bytes: &mut &[u8]) -> [u8; N] {
    let (value, remainder) = bytes.split_first_chunk().expect("fixed field");
    *bytes = remainder;
    *value
}

fn u64_field(bytes: &mut &[u8]) -> u64 {
    u64::from_le_bytes(take(bytes))
}

fn u32_field(bytes: &mut &[u8]) -> u32 {
    u32::from_le_bytes(take(bytes))
}

fn u128_field(bytes: &mut &[u8]) -> u128 {
    u128::from_le_bytes(take(bytes))
}

fn assert_metadata(bytes: &mut &[u8]) {
    assert_eq!(take::<8>(bytes), *b"THRWIMG1");
    assert_eq!(take::<32>(bytes), IDENTITY.0);
    assert_eq!(u64_field(bytes), bytes.len() as u64, "payload size");
    assert_eq!(u64_field(bytes), 1, "committed tick");
    assert_eq!(u64_field(bytes), 6, "slot capacity");
    let slots = [
        (1, 2, 45, 1),
        (0, 2, 0, 0),
        (1, 1, 30, 0),
        (1, 1, 40, 0),
        (0, 1, 0, 0),
        (0, 1, 0, 0),
    ];
    for (live, generation, identity, structure) in slots {
        assert_eq!(take::<1>(bytes)[0], live, "live flag");
        assert_eq!(u64_field(bytes), generation, "generation");
        assert_eq!(u128_field(bytes), identity, "stable identity");
        assert_eq!(u64_field(bytes), structure, "structural change tick");
    }
    assert_eq!(u64_field(bytes), 3, "free-list size");
    for free in [5, 4, 1] {
        assert_eq!(u32_field(bytes), free, "free-list order");
    }
    for (present, slot) in [(1, 0), (0, 0), (1, 2), (1, 3), (0, 0), (0, 0)] {
        assert_eq!(take::<1>(bytes)[0], present, "identity index presence");
        assert_eq!(u32_field(bytes), slot, "identity index lookup");
    }
    assert_eq!(take::<1>(bytes), [1], "row directory present");
    for map in [[0, 2, 1, 3, 4, 5]; 2] {
        assert_eq!(u64_field(bytes), 6, "row directory map size");
        let actual = std::array::from_fn(|_| u64_field(bytes));
        assert_eq!(actual, map, "row directory map");
    }
    assert_eq!(u64_field(bytes), 2, "active prefix length");
}

fn assert_columns(bytes: &mut &[u8]) {
    assert_eq!(
        u128_field(bytes),
        12,
        "schema order differs from registration order"
    );
    let change_ticks: [u64; 6] = std::array::from_fn(|_| u64_field(bytes));
    assert_eq!(change_ticks, [0; 6], "unchanged u8 change ticks");
    assert_eq!(
        take::<7>(bytes),
        [0, 0, 1, 7, 0, 0, 0],
        "u8 presence and value"
    );
    assert_eq!(u128_field(bytes), 104, "indexed schema");
    let change_ticks: [u64; 6] = std::array::from_fn(|_| u64_field(bytes));
    assert_eq!(
        change_ticks,
        [0, 0, 1, 0, 0, 0],
        "indexed component change ticks"
    );
    assert_eq!(take::<1>(bytes), [1], "spawned component presence");
    assert_eq!(
        i32::from_le_bytes(take(bytes)),
        12,
        "spawned component value"
    );
    assert_eq!(take::<2>(bytes), [0, 1], "surviving component presence");
    assert_eq!(
        i32::from_le_bytes(take(bytes)),
        19,
        "indexed component value"
    );
    assert_eq!(take::<3>(bytes), [0, 0, 0], "absent component slots");
    assert_eq!(u64_field(bytes), 18, "game resource");
    assert_eq!(*bytes, []);
}

#[test]
fn canonical_fields_and_fresh_runtime_round_trip() {
    let (mut authority, mut rules, undo) = source();
    let bytes = image(&authority, &rules);
    let mut fields = bytes.as_slice();
    assert_metadata(&mut fields);
    assert_columns(&mut fields);

    let (mut restored, mut game) = runtime(2, false);
    game.resource = 0;
    restored
        .restore_world_image(IDENTITY, &mut game.resource, &bytes, LIMIT)
        .expect("restore");
    assert_ne!(restored.epoch(), authority.epoch());
    assert!(
        restored.rewind_many(&mut [undo]).is_err(),
        "source history is foreign"
    );
    assert_eq!(digest(&authority, &rules), digest(&restored, &game));
    assert_eq!(image(&restored, &game), bytes);
    assert_eq!(game.resource, 18);
    assert_eq!(restored.tick(), 1);
    restored.publish_initial(&mut game, &mut ());

    authority
        .step(&mut rules, &mut ())
        .expect("source spawns again");
    restored
        .step(&mut game, &mut ())
        .expect("restored world spawns again");
    let view = restored.read();
    for (slot, id, generation) in [(1, 50, 2), (4, 60, 1)] {
        let live = view.live_at_slot(slot).expect("reused free slot");
        assert_eq!(live.stable_id(), StableId(id));
        assert_eq!(live.handle().generation, generation);
    }
    assert_eq!(digest(&authority, &rules), digest(&restored, &game));
    assert_eq!(image(&authority, &rules), image(&restored, &game));
}
