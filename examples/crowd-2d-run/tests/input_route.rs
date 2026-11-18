//! Exhaustive receiver-grid and rounding-boundary comparison using whole batches.
use thrum_core::profile::{Digest, Number, StateEncode};

fn compare(axis: f32) {
    let mut frames = crowd_2d::frames(1, false);
    for frame in &mut frames {
        frame.axes = [axis; 3];
    }
    crowd_2d_run::wire_route(&mut frames);
    let decoded = frames[0].axes[0];
    let mut actual = Digest::default();
    Number::from_axis(axis).encode(&mut actual);
    let mut expected = Digest::default();
    decoded.to_bits().encode(&mut expected);
    assert_eq!(actual.finish(), expected.finish(), "axis {axis:?}");
    let mut idempotent = Digest::default();
    Number::from_axis(decoded).encode(&mut idempotent);
    assert_eq!(idempotent.finish(), expected.finish());
}

#[test]
fn every_wire_value_and_both_sides_of_every_rounding_boundary() {
    for value in -32767_i32..=32767 {
        compare(value as f32 / 32767.0);
    }
    for value in -32767_i32..32767 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "probe the binary32 neighbours of each exact wire boundary"
        )]
        let boundary = ((f64::from(value) + 0.5) / 32767.0) as f32;
        compare(boundary.next_down());
        compare(boundary);
        compare(boundary.next_up());
    }
    for axis in [-1.0, 1.0, -0.0, 0.0, 0.1, 0.5, -0.5] {
        compare(axis);
    }
}

#[test]
fn local_and_codec_frames_commit_the_retained_slice() {
    let local =
        crowd_2d_run::read_result(&crowd_2d_run::conformance(2000, 240, false, false, false))
            .expect("local");
    let wire = crowd_2d_run::read_result(&crowd_2d_run::conformance(2000, 240, false, true, false))
        .expect("wire");
    assert_eq!(local["timeline"], wire["timeline"]);
    assert!(wire["changed_axes"].as_u64().expect("count") > 0);
    let retained = crowd_2d::RETAINED_CROWD_TRACE;
    for line in retained
        .lines()
        .filter(|line| line.starts_with("slice 2000 "))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        let tick: usize = fields[2].parse().expect("tick");
        assert_eq!(local["timeline"][tick - 1], fields[3]);
    }
}
