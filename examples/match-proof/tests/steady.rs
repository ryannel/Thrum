//! Short deterministic proof exercised by the workspace check.

use match_proof::{MemoryRun, run_memory};

#[test]
fn sixteen_bit_controls_keep_five_worlds_equal_on_a_conditioned_link() {
    let run = MemoryRun {
        profile: "a0",
        ticks: 600,
        bodies: 2_000,
        repeats: 2,
        seed: 29_721,
        costs: false,
        stall: false,
        listen: false,
    };
    let first = run_memory(run).expect("first five-world match");
    let second = run_memory(run).expect("same seed and fixture");
    assert_eq!(first, second);
    assert_eq!(first["checkpoints"].as_array().unwrap().len(), 20);
    let peers = first["peers"].as_array().unwrap();
    assert_eq!(peers.len(), 4);
    for peer in peers {
        assert_eq!(peer["restored"][0], 0);
        assert_eq!(peer["restored"][1], first["baseline_digest"]);
        assert_ne!(peer["world_epoch"], first["host_epoch"]);
        assert_eq!(peer["digest_ticks"], 20);
        assert!(peer["off_grid_frames"].as_u64().unwrap() > 500);
        assert!(peer["axes_changes"].as_u64().unwrap() > 40);
        assert!(peer["carrier_sets"]["duplicated"].as_u64().unwrap() > 0);
    }
}
