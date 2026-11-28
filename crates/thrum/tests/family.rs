use thrum::prelude::*;

#[derive(Component)]
struct PickUp(Timer);

#[derive(Resource, Default)]
struct Actors(Option<[Entity; 5]>);

fn begin(mut entities: Entities<'_>, mut ids: ResMut<'_, Actors>) {
    let quarter = Angle::degrees(90);
    let root = entities.spawn((
        Transform2d::at(10, 20).turned(quarter).scaled(2),
        Transform3d::at(10, 20, 30).turned_y(quarter).scaled(2),
        PickUp(Timer::IDLE),
    ));
    let middle = entities.spawn((
        Transform2d::at(1, 2).turned(quarter).scaled(2),
        Transform3d::at(1, 2, 3).turned_y(quarter).scaled(2),
        PickUp(Timer::IDLE),
    ));
    let leaf = entities.spawn((
        Transform2d::at(2, 3),
        Transform3d::at(2, 3, 4),
        PickUp(Timer::IDLE),
    ));
    let loose = entities.spawn((Transform2d::at(8, 26), Transform3d::at(18, 28, 26)));
    let fraction = entities.spawn((Transform2d::at(Num::ONE / Num::from(10), 20),));
    entities
        .attach(middle)
        .to(root)
        .keep_local(Transform2d::at(1, 2).turned(quarter).scaled(2));
    entities
        .attach(leaf)
        .to(middle)
        .keep_local(Transform2d::at(2, 3));
    ids.0 = Some([root, middle, leaf, loose, fraction]);
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK lends rule parameters by value"
)]
fn edit(tick: Tick, mut entities: Entities<'_>, ids: Res<'_, Actors>) {
    let [root, middle, leaf, loose, fraction] = ids.0.expect("fixture provides this value");
    if tick.seconds() == seconds(1) {
        let before2 = entities.world2d(loose);
        let before3 = entities.world3d(loose);
        assert_eq!(entities.children(root).collect::<Vec<_>>(), [middle]);
        assert_eq!(
            entities.world2d(leaf),
            Transform2d::at(-2, 10)
                .turned(Angle::degrees(180))
                .scaled(4)
        );
        assert_eq!(
            entities.world3d(leaf),
            Transform3d::at(8, 36, 12)
                .turned_y(Angle::degrees(180))
                .scaled(4)
        );
        entities.attach(loose).to(root).keep_world();
        entities.attach(fraction).to(root).keep_world();
        assert_eq!(entities.parent(loose), None);
        assert!(entities.world2d(loose).same_encoding(&before2));
        assert!(entities.world3d(loose).same_encoding(&before3));
        assert!(entities.disable(root));
        assert!(!entities.disable(root));

        let reserved = entities.spawn((Transform2d::at(1, 1), PickUp(Timer::IDLE)));
        entities
            .attach(reserved)
            .to(middle)
            .keep_local(Transform2d::at(1, 1));
        assert_eq!(entities.children(middle).collect::<Vec<_>>(), [leaf]);
    } else if tick.seconds() == seconds(2) {
        assert_eq!(
            entities.children(root).collect::<Vec<_>>(),
            [middle, loose, fraction]
        );
        assert_eq!(entities.children(middle).len(), 2);
        assert_eq!(entities.parent(leaf), Some(middle));
        assert!(
            entities
                .world2d(loose)
                .same_encoding(&Transform2d::at(8, 26))
        );
        assert!(
            entities
                .world3d(loose)
                .same_encoding(&Transform3d::at(18, 28, 26))
        );
        let delta = entities.world2d(fraction).position.x - Num::ONE / Num::from(10);
        let bound = Num::from(10) / Num::from(1 << 17);
        assert!((-bound..=bound).contains(&delta));
        entities.detach(loose).keep_world();
        entities.detach(fraction).at(Transform2d::at(5, 3));
        assert!(entities.enable(root));
        assert!(!entities.enable(root));
        assert_eq!(entities.parent(loose), Some(root));
    } else if tick.seconds() == seconds(3) {
        assert_eq!(entities.parent(loose), None);
        assert_eq!(entities.parent(fraction), None);
        assert!(
            entities
                .world2d(loose)
                .same_encoding(&Transform2d::at(8, 26))
        );
        assert!(
            entities
                .world3d(loose)
                .same_encoding(&Transform3d::at(18, 28, 26))
        );
        assert_eq!(entities.world2d(fraction), Transform2d::at(5, 3));
        assert!(entities.despawn(root));
        assert!(!entities.despawn(leaf));
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK lends rule parameters by value"
)]
fn inspect(
    tick: Tick,
    ordinary: Query<'_, (Entity, &PickUp)>,
    sleeping: Query<'_, (Entity, &PickUp), Disabled>,
) {
    if tick.seconds() == seconds(1) {
        assert!(ordinary.is_empty());
        assert_eq!(sleeping.count(), 4);
        assert!(sleeping.iter().all(|(_, pickup)| pickup.0 == Timer::IDLE));
    } else if tick.seconds() == seconds(2) {
        assert_eq!(ordinary.count(), 4);
        assert!(sleeping.is_empty());
    } else if tick.seconds() == seconds(3) {
        assert!(ordinary.is_empty());
        assert!(sleeping.is_empty());
    }
}

#[test]
fn transforms_and_barriers_replay_bit_for_bit() -> Result {
    let mut game = Game::new("family")
        .max_entities(8)
        .tick_rate(1)
        .resource(Actors::default())
        .scene(|scene: &mut Scene| {
            scene.on_enter(begin);
            scene.rules((edit, inspect));
        })
        .test()?;
    let [root, middle, leaf, loose, fraction] = game
        .resource::<Actors>()?
        .0
        .expect("fixture provides this value");
    let before = game.digest();
    game.step()?;
    let first = game.digest();
    game.step()?;
    let second = game.digest();
    game.step()?;
    let third = game.digest();
    assert!(!game.alive(root) && !game.alive(middle) && !game.alive(leaf));
    assert!(game.alive(loose) && game.alive(fraction));
    game.rewind(3)?;
    assert_eq!(game.digest(), before);
    for expected in [first, second, third] {
        game.step()?;
        assert_eq!(game.digest(), expected);
    }
    Ok(())
}

#[test]
fn the_parent_changes_the_digest_with_identical_components() -> Result {
    let mut game = Game::new("parent digest")
        .max_entities(4)
        .scene(|_: &mut Scene| {})
        .test()?;
    let first = game.spawn(())?;
    let second = game.spawn(())?;
    let local = Transform2d::at(1, 2);
    let child = game.spawn((local,))?;
    game.run(move |mut entities: Entities<'_>| entities.attach(child).to(first).keep_local(local))?;
    let digest = game.digest();
    game.run(move |mut entities: Entities<'_>| {
        entities.attach(child).to(second).keep_local(local);
    })?;
    assert_ne!(game.digest(), digest);
    assert!(game.get::<Transform2d>(child)?.same_encoding(&local));
    game.run(move |mut entities: Entities<'_>| entities.attach(child).to(first).keep_local(local))?;
    assert_eq!(game.digest(), digest);
    Ok(())
}

#[test]
fn prefab_under_lands_at_the_next_rule() {
    let output = crate::prefabs::rustc(
        r#"
        use thrum::prelude::*;
        prefabs! { const CHILD: Prefab = "child"; }
        #[derive(Resource, Default)]
        struct Handles(Option<(Entity, Entity)>);
        fn birth(mut entities: Entities<'_>, mut handles: ResMut<'_, Handles>) {
            let parent = entities.spawn(());
            let child = entities.spawn(CHILD.at_local(2, 3).under(parent));
            assert_eq!(entities.children(parent).len(), 0);
            handles.0 = Some((parent, child));
        }
        fn inspect(family: Family<'_>, handles: Res<'_, Handles>) {
            let (parent, child) = handles.0.expect("the preceding rule spawned both");
            assert_eq!(family.children(parent).collect::<Vec<_>>(), [child]);
            assert_eq!(family.parent(child), Some(parent));
            assert_eq!(family.world2d(child), Transform2d::at(2, 3));
        }
        fn main() -> Result {
            let mut game = Game::new("spawn under").max_entities(4)
                .prefab(CHILD, ()).resource(Handles::default())
                .scene(|scene: &mut Scene| scene.rules((birth, inspect))).test()?;
            game.step()
        }
        "#,
        true,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
