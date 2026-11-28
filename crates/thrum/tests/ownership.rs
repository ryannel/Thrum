use thrum::prelude::*;

use crate::prefabs::rustc;

#[test]
fn choosing_children_and_querying_their_owner() {
    let output = rustc(include_str!("fixtures/ownership_game.rs"), true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[derive(Component)]
struct A(u32);
#[derive(Component)]
struct B;
#[derive(Resource, Default)]
struct Seen(Option<(u32, u32, u32)>);

#[test]
fn optional_query_items_and_intersected_visibility() -> Result {
    let mut game = Game::new("query filters")
        .max_entities(8)
        .resource(Seen::default())
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                let first = entities.spawn((A(1),));
                entities.spawn((B,));
                let third = entities.spawn((A(3), B));
                entities.disable(first);
                entities.disable(third);
            });
            scene.rules((
                |mut all: Query<'_, Option<&mut A>, WithDisabled>| {
                    let mut values = (&mut all).into_iter();
                    values
                        .next()
                        .expect("fixture provides this value")
                        .expect("fixture provides this value")
                        .0 += 10;
                    assert!(
                        values
                            .next()
                            .expect("fixture provides this value")
                            .is_none()
                    );
                    values
                        .next()
                        .expect("fixture provides this value")
                        .expect("fixture provides this value")
                        .0 += 10;
                    assert!(values.next().is_none());
                },
                |all: Query<'_, (Entity, Option<&A>), WithDisabled>,
                 filtered: Query<'_, &A, (With<B>, WithDisabled, Disabled)>,
                 mut seen: ResMut<'_, Seen>| {
                    let all: Vec<_> = all.iter().collect();
                    assert!(all.windows(2).all(|rows| rows[0].0 < rows[1].0));
                    assert!(all[1].1.is_none());
                    assert_eq!(filtered.count(), 1);
                    seen.0 = Some((
                        all[0].1.expect("fixture provides this value").0,
                        all[2].1.expect("fixture provides this value").0,
                        filtered
                            .iter()
                            .next()
                            .expect("fixture provides this value")
                            .0,
                    ));
                },
            ));
        })
        .test()?;
    game.step()?;
    assert_eq!(game.resource::<Seen>()?.0, Some((11, 13, 13)));
    game.step()?;
    let digest = game.digest();
    assert_eq!(game.resource::<Seen>()?.0, Some((21, 23, 23)));
    game.rewind(1)?;
    game.step()?;
    assert_eq!(game.digest(), digest);
    let refused = game
        .run(|_: Query<'_, Option<&mut A>, WithDisabled>, _: Query<'_, &A, WithDisabled>| {})
        .expect_err("an optional write is still a write");
    let error = refused.to_string();
    for part in [
        "T0303",
        "parameter 1",
        "parameter 2",
        "A",
        "one writes",
        "disjoint",
    ] {
        assert!(error.contains(part), "missing {part:?} in {error}");
    }
    assert_eq!(game.digest(), digest);
    Ok(())
}

#[test]
fn under_yields_distinct_mutable_rows_in_attachment_order() -> Result {
    let mut game = Game::new("attachment order")
        .max_entities(4)
        .component::<A>()
        .scene(|_: &mut Scene| {})
        .test()?;
    let parent = game.spawn(())?;
    let earlier = game.spawn((A(1),))?;
    let later = game.spawn((A(2),))?;
    game.run(move |mut entities: Entities<'_>| {
        entities
            .attach(later)
            .to(parent)
            .keep_local(Transform2d::ORIGIN);
        entities
            .attach(earlier)
            .to(parent)
            .keep_local(Transform2d::ORIGIN);
    })?;
    game.run(move |mut query: Query<'_, (Entity, &mut A)>| {
        let mut children = query.under(parent);
        let (first, a) = children.next().expect("fixture provides this value");
        let (second, b) = children.next().expect("fixture provides this value");
        assert_eq!([first, second], [later, earlier]);
        assert!(children.next().is_none());
        std::mem::swap(a, b);
    })?;
    assert_eq!(game.get::<A>(earlier)?, A(2));
    assert_eq!(game.get::<A>(later)?, A(1));
    Ok(())
}
