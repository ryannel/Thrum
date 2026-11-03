# I1 bounded pending input storage

> Status after the code-quality cleanup: the I1 timing examples cited here were removed from the active tree. Their historical measurements and source remain in Git; quoted example commands are no longer current commands.

1 November 2026. Each peer's pending inputs now occupy an ordered vector with a hard eight-entry logical limit. A newly registered peer on a charged authority reserves and charges the vector's requested eight-slot storage before entering the peer map. An uncharged prototype peer allocates the same bounded vector on its first accepted input. Admitted sequence numbers append in increasing order; binary search preserves duplicate and removal lookup, while the due-tick scan retains canonical peer/sequence order. Removing an early due input can shift at most seven pending values. Reconnect discards old inputs and outcomes but reuses the charged vector, so it does not need a second reservation while the old peer still owns one.

A focused test fills every unreserved pool byte after authority activation. The first peer join returns `AdmissionError::PublicationCapacity` without changing usage. After capacity returns, the peer joins and its requested buffer charge remains held. With the pool full again, a valid action queues, reconnect succeeds by reusing storage, the old epoch is rejected, and the new epoch queues sequence 1. Dropping the authority releases the pool to zero. The existing delayed gap, duplicate, acknowledgement, two-peer and host tests pass under `RUSTC_WRAPPER= ./dev check` (19 core, 6 clock and 42 game integration tests).

The `release pending fixture` (historical `git show 84d7673:crates/thrum-core/examples/i1_pending_cost.rs`) prepares 128 authorities with eight inputs each, then times only the first tick, where sequence 1 resolves and seven future inputs remain. Each run has 10 warmup and 100 measured batches. The same fixture source ran in a temporary worktree at old-tree commit `b0ef938` and on the vector path. Three local macOS/aarch64 runs gave these median ns per tick values:

| Action size | Old tree | Bounded vector |
| ---: | ---: | ---: |
| 16 inline bytes | 262 / 260 / 215 | 228 / 188 / 155 |
| 4,096 inline bytes | 1,186 / 1,187 / 1,081 | 1,090 / 947 / 913 |

The vector moved up to seven values during removal, yet none of these local paired runs showed a median regression for the tested sizes. The tails varied, and this fixture excludes peer setup, admission and replica work. A separate 1,000-tick complete-path batch in the `headless cost fixture` (historical `git show 84d7673:crates/thrum-core/examples/i1_cost.rs`) measured charged median 657/574 ns on the vector path versus 648/594 ns on the old tree in two runs; those ranges overlap and establish no complete-path speedup. The eight pending action payload values still keep their own charged owners; this change accounts only for the requested queue buffer. The peer map, completed history, due-key scratch, future execution credits, group and replica storage remain outside the whole-tick byte gate.
