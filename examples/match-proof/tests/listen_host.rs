//! Listen host and three independently restored replicas share one match.

use match_proof::{MemoryRun, run_memory};

#[test]
fn listen_player_flows_through_the_shared_set() {
    let record = run_memory(MemoryRun {
        profile: "a0",
        ticks: 600,
        bodies: 256,
        repeats: 2,
        seed: 29_721,
        costs: false,
        stall: false,
        listen: true,
    })
    .expect("listen host and three replica digests");
    assert_eq!(record["listen_host"], true);
    assert_eq!(record["host_local_admitted"], 600);
    let peers = record["peers"].as_array().unwrap();
    assert_eq!(peers.len(), 3);
    assert_eq!(record["checkpoints"].as_array().unwrap().len(), 20);
    let mut slots = peers
        .iter()
        .map(|peer| peer["slot"].as_u64().unwrap())
        .collect::<Vec<_>>();
    slots.sort_unstable();
    assert_eq!(slots, [1, 2, 3]);
    for peer in peers {
        assert_eq!(peer["digest_ticks"], 20);
    }
}
