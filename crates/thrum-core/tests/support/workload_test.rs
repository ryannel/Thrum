//! Fixture correctness and zero-allocation assertion for the representative workload.
mod v1_workload;
use v1_workload as support;

#[global_allocator]
static ALLOCATOR: thrum_test_support::CountingAllocator = thrum_test_support::CountingAllocator;

#[test]
fn representative_scaffold_is_deterministic_and_allocation_free_after_setup() {
    let mut fixture = support::Workload::new();
    assert_eq!(fixture.entity_count(), 10_000);
    for _ in 0..60 {
        fixture.step();
    }
    let start_position = fixture.first_position();
    let start_churn = fixture.churned;
    let ((), counted) = thrum_test_support::allocations::count(|| {
        for _ in 0..600 {
            fixture.step();
        }
    });
    assert_eq!(counted.count, 0);
    assert!(fixture.first_position() > start_position);
    assert_eq!(fixture.churned - start_churn, 1_000);
    assert!(fixture.link.dropped > 0);
    assert!(fixture.link.delivered.iter().all(|count| *count > 0));
    assert_eq!(fixture.link.min_one_way_ticks, 2);
    assert_eq!(fixture.link.max_one_way_ticks, 4);
    let sent = fixture.tick * support::PEER_COUNT as u64;
    let delivered: u64 = fixture.link.delivered.iter().sum();
    assert_eq!(
        delivered + fixture.link.dropped + fixture.link.in_flight() as u64,
        sent
    );
    assert!(fixture.link.dropped * 100 > sent);
    assert!(fixture.link.dropped * 100 < sent * 3);
}
