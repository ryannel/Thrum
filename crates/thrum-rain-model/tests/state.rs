//! M1.7: the rain state is plain data. It encodes and decodes to the same bytes, and
//! stepping it from the same state and inputs gives the same bytes on one thread and on
//! eight.
#![expect(
    clippy::cast_possible_truncation,
    reason = "test counts of cells, hollows and nodes are far below u32::MAX"
)]
use crate::support;
use support::{many_hollows, mm_h, prepare};
use thrum_rain_model::{GroundClass, Rain, RainError, RainState, TickReport, Tree};

fn steps(
    tree: &Tree,
    state: &mut RainState,
    threads: usize,
    from: usize,
    to: usize,
) -> Vec<TickReport> {
    let mut out = Vec::new();
    let mut report = TickReport::default();
    for t in from..to {
        let rain = if t < 1500 {
            Rain {
                flux: mm_h(40.0),
                drying: 0.0,
            }
        } else {
            Rain {
                flux: 0.0,
                drying: mm_h(90.0),
            }
        };
        state
            .step(
                tree,
                rain,
                GroundClass::bare(mm_h(2.0)),
                1.0,
                threads,
                &mut report,
            )
            .expect("valid test case");
        out.push(report.clone());
    }
    out
}

#[test]
fn the_state_round_trips_and_steps_the_same_on_one_thread_and_eight() {
    let tree = prepare(&many_hollows(120));
    assert!(
        tree.hollow_count() >= 64,
        "{} hollows, enough to split over eight threads",
        tree.hollow_count()
    );
    let mut one = RainState::new(&tree);
    let mut eight = RainState::new(&tree);
    let (mut merges, mut partings) = (0, 0);
    for (a, b) in steps(&tree, &mut one, 1, 0, 2500)
        .iter()
        .zip(steps(&tree, &mut eight, 8, 0, 2500).iter())
    {
        assert_eq!(a, b, "the reports match tick for tick");
        merges += a.merges;
        partings += a.partings;
    }
    assert!(
        merges > 0 && partings > 0,
        "merges {merges}, partings {partings}"
    );
    assert_eq!(one.encode(), eight.encode());
    // Through bytes and back, mid-run: the same bytes, and the same future.
    let mut mid = RainState::new(&tree);
    steps(&tree, &mut mid, 3, 0, 1200);
    let bytes = mid.encode();
    let mut back = RainState::decode(&bytes, &tree).expect("valid test case");
    assert_eq!(back.encode(), bytes);
    assert_eq!(back, mid);
    steps(&tree, &mut back, 8, 1200, 2500);
    assert_eq!(back.encode(), one.encode());
}

#[test]
fn bytes_for_another_tree_or_damaged_bytes_are_refused() {
    let tree = prepare(&many_hollows(120));
    let other = prepare(&many_hollows(60));
    let mut state = RainState::new(&tree);
    steps(&tree, &mut state, 1, 0, 100);
    let bytes = state.encode();
    assert!(matches!(
        RainState::decode(&bytes, &other),
        Err(RainError::State(_))
    ));
    assert!(RainState::decode(&bytes[..bytes.len() - 1], &tree).is_err());
    let mut bad = bytes.clone();
    bad[0] = b'X';
    assert!(RainState::decode(&bad, &tree).is_err());
    // Two puddles over one hollow.
    let mut bad = bytes;
    let leaf = (0..tree.hollow_count() as u32)
        .find(|&h| tree.node(h).parent.is_some())
        .expect("valid test case");
    let parent = tree.node(leaf).parent.expect("valid test case") as usize;
    let flag = |k: usize| 8 + 8 + 8 + k * 9;
    bad[flag(leaf as usize)] = 1;
    bad[flag(parent)] = 1;
    assert!(RainState::decode(&bad, &tree).is_err());
}
