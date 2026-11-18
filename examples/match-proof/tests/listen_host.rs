//! Listen host and three independently restored replicas share one match.

use match_proof::{MemoryRun, run_memory};

#[test]
fn listen_player_flows_through_the_shared_set() {
    let record = run_memory(MemoryRun {
        profile: "a0",
        ticks: 720,
        bodies: 2_000,
        repeats: 2,
        seed: 29_721,
        costs: false,
        stall: false,
        listen: true,
    })
    .expect("listen host and three replica digests");
    assert_eq!(record["listen_host"], true);
    assert_eq!(record["host_local_admitted"], 720);
    let peers = record["peers"].as_array().unwrap();
    assert_eq!(peers.len(), 3);
    assert_eq!(record["checkpoints"].as_array().unwrap().len(), 24);
    for (index, peer) in (1..=3).zip(peers) {
        assert_eq!(peer["slot"], index);
        assert_eq!(peer["digest_ticks"], 24);
    }
}
