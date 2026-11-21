//! Public authority digest equality across epochs and corrected replay.
use super::profile_game;
use profile_game as support;
use thrum_core::profile::Number;

#[test]
fn independent_epochs_and_rewound_worlds_agree_each_tick() {
    let (mut a, mut ga, mut ca) = support::setup(1);
    let (mut b, mut gb, mut cb) = support::setup(999);
    assert_ne!(a.epoch(), b.epoch());
    assert_eq!(support::digest(&a, &ga), support::digest(&b, &gb));
    for tick in 1..=32 {
        let value = Number::from_i32(tick);
        ga.next = value;
        gb.next = value;
        let mut undo = [a.new_undo()];
        a.step_recording(
            &mut ga,
            &mut ca,
            &mut undo[0],
            thrum_core::sim::PublicationMode::Capture,
        )
        .expect("record");
        b.step(&mut gb, &mut cb).expect("straight");
        let expected = support::digest(&b, &gb);
        assert_eq!(support::digest(&a, &ga), expected);
        a.rewind_many(&mut undo).expect("rewind");
        ga.resource -= 1;
        a.step(&mut ga, &mut ca).expect("replay");
        assert_eq!(support::digest(&a, &ga), expected);
        ga.resource += 1;
        assert_ne!(support::digest(&a, &ga), expected);
        ga.resource -= 1;
    }
    assert_ne!(
        support::number_digest(Number::ONE),
        support::number_digest(Number::ZERO)
    );
}
