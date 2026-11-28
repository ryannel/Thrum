use thrum::prelude::*;

prefabs! {
    const BAT: Prefab = "bat";
    const ELITE_BAT: Prefab = "elite bat";
    const GEM: Prefab = "gem";
    const SHOT: Prefab<(Shot, Velocity)> = "shot";
}

const EYE: Part = Part::new("eye");
const BACK: Part = Part::new("back");

#[derive(Component)]
struct Health(u32);
#[derive(Component)]
struct Appearance {
    tint: u32,
    layer: u32,
}
#[derive(Component)]
struct Gem;
#[derive(Component)]
struct Shot(u32);
#[derive(Component)]
struct Velocity(Vec2);
#[derive(Component)]
struct DefaultOnly(u32);
#[derive(Component)]
struct SpawnOnly(u32);
#[derive(Resource, Default)]
struct Actors(Option<[Entity; 4]>);

fn prefabs(game: Game) -> Game {
    game.prefab(
        ELITE_BAT,
        BAT.variant()
            .with(Health(8))
            .tweak(|look: &mut Appearance| look.tint = 9),
    )
    .prefab(
        BAT,
        PrefabDef::new((Health(3), Appearance { tint: 2, layer: 4 }))
            .with(Health(4))
            .with(DefaultOnly(17))
            .child(GEM.at_local(1, 0))
            .part(EYE, GEM.at_local(0, 2))
            .part(BACK, ()),
    )
    .prefab(GEM, (Gem,))
    .prefab(SHOT, ())
}

fn populate(mut entities: Entities, mut actors: ResMut<Actors>) {
    let bat = entities.spawn(BAT.at(2, 3));
    let elite = entities.spawn(
        ELITE_BAT
            .at_point(vec2(4, 5))
            .with(Health(10))
            .with(SpawnOnly(11))
            .tweak(|look: &mut Appearance| look.layer = 7),
    );
    let gem = entities.spawn(GEM.at_point(vec2(8, 9)));
    let shot = entities.spawn(
        SHOT.with((Shot(12), Velocity(vec2(6, 0))))
            .at_point(vec2(4, 5)),
    );
    actors.0 = Some([bat, elite, gem, shot]);
}

fn main() -> Result {
    let mut game = prefabs(
        Game::new("prefabs")
            .max_entities(16)
            .resource(Actors::default())
            .component::<SpawnOnly>()
            .scene(|scene: &mut Scene| scene.on_enter(populate)),
    )
    .test()?;
    game.step()?;
    let [bat, elite, gem, shot] = game.resource::<Actors>()?.0.unwrap();
    assert_eq!(game.get::<Health>(bat)?, Health(4));
    assert_eq!(game.get::<Health>(elite)?, Health(10));
    assert_eq!(game.get::<DefaultOnly>(elite)?, DefaultOnly(17));
    assert_eq!(
        game.get::<Appearance>(bat)?,
        Appearance { tint: 2, layer: 4 }
    );
    assert_eq!(
        game.get::<Appearance>(elite)?,
        Appearance { tint: 9, layer: 7 }
    );
    assert_eq!(game.get::<SpawnOnly>(elite)?, SpawnOnly(11));
    assert_eq!(game.get::<Shot>(shot)?, Shot(12));
    assert_eq!(game.get::<Velocity>(shot)?, Velocity(vec2(6, 0)));
    assert_eq!(game.get::<Transform2d>(gem)?, Transform2d::at(8, 9));
    assert_eq!(game.get::<Transform2d>(bat)?, Transform2d::at(2, 3));
    let children = game.run(move |tree: Family| {
        let children: Vec<_> = tree.children(bat).collect();
        assert_eq!(children.len(), 3);
        assert_eq!(tree.part(bat, EYE), children[1]);
        assert_eq!(tree.part(bat, BACK), children[2]);
        assert_eq!(tree.world2d(children[0]), Transform2d::at(3, 3));
        assert_eq!(tree.world2d(children[1]), Transform2d::at(2, 5));
        children
    })?;
    assert_eq!(game.get::<Transform2d>(children[0])?, Transform2d::at(1, 0));
    assert_eq!(game.get::<Transform2d>(children[1])?, Transform2d::at(0, 2));
    assert!(game.despawn(bat)?);
    assert!(children.into_iter().all(|child| !game.alive(child)));
    let more = game.spawn(ELITE_BAT.at(6, 7))?;
    assert_eq!(game.get::<Health>(more)?, Health(8));
    assert_eq!(
        game.get::<Appearance>(more)?,
        Appearance { tint: 9, layer: 4 }
    );
    assert_eq!(game.get::<Transform2d>(more)?, Transform2d::at(6, 7));
    allocations()?;
    retry_after_tweak_panic()
}

// Count allocation only inside the rule's spawn calls. The first ticks warm
// the structural edit queue shared with bare tuple spawns.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct CountAllocations;

thread_local! {
    static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

// SAFETY: Every request is forwarded unchanged to System, the sole allocator.
unsafe impl GlobalAlloc for CountAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        // SAFETY: The caller made this allocation request through GlobalAlloc.
        unsafe { System.alloc(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        // SAFETY: The pointer and layouts are forwarded to their allocator.
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout are forwarded to their allocator.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountAllocations = CountAllocations;

fn count_allocation() {
    let _ = ALLOCATIONS.try_with(|count| {
        if let Some(current) = count.get() {
            count.set(Some(current + 1));
        }
    });
}

fn spawn_repeatedly(tick: Tick, existing: Query<Entity, WithDisabled>, mut entities: Entities) {
    for entity in &existing {
        entities.despawn(entity);
    }
    let measure = tick.seconds() >= seconds(4);
    ALLOCATIONS.set(measure.then_some(0));
    entities.spawn(GEM.at_point(vec2(3, 4)));
    entities.spawn(
        SHOT.with((Shot(3), Velocity(vec2(1, 0))))
            .at(0, 0)
            .tweak(|shot: &mut Shot| {
                shot.0 += 1;
            }),
    );
    if measure {
        assert_eq!(ALLOCATIONS.replace(None), Some(0));
    }
}

fn allocations() -> Result {
    let mut game = prefabs(
        Game::new("spawn allocation guard")
            .max_entities(8)
            .tick_rate(1)
            .scene(|scene: &mut Scene| scene.rules(spawn_repeatedly)),
    )
    .test()?;
    game.step_ticks(8)?;
    assert_eq!(game.count::<WithDisabled>(), 2);
    Ok(())
}

fn retry_after_tweak_panic() -> Result {
    let mut game = prefabs(
        Game::new("tweak rollback")
            .max_entities(4)
            .scene(|_: &mut Scene| {}),
    )
    .test()?;
    let before = game.digest();
    let failure = game
        .run(|mut entities: Entities| {
            entities.spawn(
                SHOT.with((Shot(3), Velocity(vec2(1, 0))))
                    .tweak(|_: &mut Shot| panic!("intentional tweak failure")),
            );
        })
        .expect_err("the tweak refuses the edit");
    assert_eq!(failure.code(), "T0307");
    assert_eq!(game.digest(), before);
    let shot = game.spawn(
        SHOT.with((Shot(10), Velocity(vec2(2, 0))))
            .tweak(|shot: &mut Shot| shot.0 += 1),
    )?;
    assert_eq!(game.get::<Shot>(shot)?, Shot(11));
    Ok(())
}
