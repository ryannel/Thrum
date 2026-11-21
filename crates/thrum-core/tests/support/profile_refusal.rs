//! Atomic profile refusal through a registered rule and ordinary runtime commit.
use super::profile_game;
use profile_game as support;
use thrum_core::{
    profile::Number,
    sim::{ComponentSchemaId, EditError, RegisteredWorldError, TickError},
};

#[test]
fn nested_infinity_and_nan_refuse_atomically_then_a_valid_tick_commits() {
    for invalid in [Number::ONE / Number::ZERO, Number::ZERO / Number::ZERO] {
        let (mut runtime, mut game, mut candidate) = support::setup(1);
        let before = support::digest(&runtime, &game);
        game.next = invalid;
        assert_eq!(
            runtime.step(&mut game, &mut candidate),
            Err(TickError::Edit(EditError::World(
                RegisteredWorldError::NonFinite {
                    schema: ComponentSchemaId(123),
                    slot: game.handle.slot
                }
            )))
        );
        assert_eq!(runtime.tick(), 0);
        assert_eq!(game.resource, 0);
        assert_eq!(support::digest(&runtime, &game), before);
        game.next = Number::from_i32(2);
        runtime
            .step(&mut game, &mut candidate)
            .expect("next valid tick");
        assert_eq!(runtime.tick(), 1);
        assert_ne!(support::digest(&runtime, &game), before);
        assert_ne!(
            support::number_digest(Number::ONE),
            support::number_digest(game.next)
        );
    }
}
