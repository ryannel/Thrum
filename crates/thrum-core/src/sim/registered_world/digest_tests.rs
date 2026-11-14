use super::*;
use crate::profile::{Digest, StateEncode};

fn fixture() -> RegisteredWorld {
    let mut registry = ComponentRegistry::default();
    registry
        .register_profile::<u32>(ComponentSchemaId(7))
        .expect("schema");
    let mut world = RegisteredWorld::new(WorldEpoch(1), 5, registry).expect("world");
    world
        .spawn(StableId(1), |writer| {
            writer.add(1_u32)?;
            Ok(())
        })
        .expect("seed");
    world.directory = Some(DenseDirectory::new(5));
    world
}
fn digest(world: &RegisteredWorld, tick: u64) -> u64 {
    let mut digest = Digest::default();
    tick.encode(&mut digest);
    world
        .digest(&mut digest, &mut |_, _| {})
        .expect("portable schema");
    digest.finish()
}
#[test]
fn every_covered_metadata_kind_changes_the_digest_independently() {
    type Mutation = (&'static str, fn(&mut RegisteredWorld));
    let mutations: &[Mutation] = &[
        ("live flag", |world| world.slots[0].stable = None),
        ("generation", |world| world.slots[0].generation += 1),
        ("stable identity", |world| {
            world.slots[0].stable = Some(StableId(2));
        }),
        ("structural tick", |world| {
            world.slots[0].structural_tick += 1;
        }),
        ("free-list order", |world| world.free.swap(0, 1)),
        ("component presence", |world| {
            world.column_mut_index::<u32>(0).remove(0);
        }),
        ("component value", |world| {
            world.column_mut_index::<u32>(0).insert(0, 2);
        }),
        ("change tick", |world| world.change_ticks[0] += 1),
        ("identity index", |world| world.ids.remove(StableId(1))),
        ("row directory", |world| {
            world
                .directory
                .as_mut()
                .expect("directory")
                .transition(1, true);
        }),
    ];
    for &(name, mutate) in mutations {
        let mut world = fixture();
        let before = digest(&world, 0);
        mutate(&mut world);
        assert_ne!(digest(&world, 0), before, "{name}");
    }
    let world = fixture();
    assert_ne!(digest(&world, 0), digest(&world, 1));
}

#[test]
fn integer_mixer_has_published_fnv1a_vectors() {
    let mut empty = Digest::default();
    assert_eq!(empty.finish(), 0xcbf2_9ce4_8422_2325);
    empty.bytes(b"a");
    assert_eq!(empty.finish(), 0xaf63_dc4c_8601_ec8c);
    let mut hello = Digest::default();
    hello.bytes(b"hello");
    assert_eq!(hello.finish(), 0xa430_d846_80aa_bd0b);
}
