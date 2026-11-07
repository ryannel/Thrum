# N1 joint mandatory identity credit

3 November 2026. The [production N1 gate](../../BUILD-PLAN.md) requires a
per-peer owner to preserve accepted mandatory output under burst and slow-ACK
pressure. The [structural pressure test](../HISTORICAL-SOURCE.md)
uses the actual `OrdinaryPeer`, its unsent queue and sent-frame ledger. Its
validator intentionally accepts only structural R4 framing; it is not a game
schema or a qualified traffic recipe.

## Change and proof

The queue's small copied metadata now counts queued result and event records.
Before any whole-peer batch copies payload bytes, the peer compares those
counts with the sent ledger's remaining 256-result and 128-event journal
credit. A refusal leaves the queue's prior payload bytes, order and cursors
untouched, assigns no application sequence, and puts this ordinary-only peer
in its explicit close state. Successful transfer moves one unit's count from
queue to sent storage; an exact admission ACK frees the sent journal credit.
Standalone `UnsentQueue` remains a construction slice without a paired sent
owner and does not claim aggregate credit enforcement.

Five no-socket tests pass:

- Four partitions with 32 events each fill the joint 128-event credit. A
  second complete four-group tick refuses atomically; the first four groups
  remain queued, sequence stays zero, and only that peer requires close.
- One controlled partition queues 64 results per tick. The fifth tick
  refuses before exceeding 256 queued results, with four earlier units and
  sequence zero retained.
- With 64 events in two submitted frames and 64 in two unsent units, the
  next complete tick refuses. This distinguishes the joint credit from a
  queue-only limit. A failed authority ACK also keeps the sent credit
  occupied; retrying that exact ACK frees it, after which the same 64-event
  next tick is admitted.
- Three chronological four-group ticks, each with 64 results in the
  controlled partition and 32 events in every partition, pass when each
  prior tick is exactly written and ACKed. All 12 framed payloads match their
  original bytes, ticks and partition order; the authority result frontier
  reaches 192, and the peer's reserved Rust owner capacity is unchanged.

Closed peers deliberately expose no readback API. The refusal cases assert
retained counts, sequence and phase; the existing [atomic queue proof](N1-ATOMIC-CAPTURE-BATCH-2026-11-03.md)
checks exact payload bytes and cursors after a failed batch preflight. This
joint proof checks exact framed bytes on the successful ACK/reuse path.

`RUSTC_WRAPPER= cargo test -p thrum-network --test joint_burst` and
`RUSTC_WRAPPER= ./dev check` passed in the isolated N1 worktree. The latter
also checks repository policy and the other crates, but it does not prove
production game closure, event semantics, authenticated ACKs, transport,
baseline/sync credit, four-peer CPU tails or the network bandwidth target.
The selected game recipe still needs joint actor, lifecycle, result, event
and bytes-per-window bounds before N1 is ready.

## Paired ordinary-path cost

The [raw release samples](N1-JOINT-CREDIT-COST-SAMPLES-2026-11-03.csv) compare
the pre-change `1c7ac5e` control with the candidate on the local Apple M3
Max, Rust 1.99.1. The candidate is exactly that control checkout with the
three changed production source files below; the new `joint_burst` test and
documentation do not enter the measured `w1_capture` executable. Each binary
was built with `RUSTC_WRAPPER= cargo test --release -p thrum-network --test
w1_capture --no-run`, then run directly with `--ignored --exact` for each
named test and `--nocapture`. The fixture file SHA-256 in both checkouts is
`65e8d3bf36040b78bef12e8141e51df7821eb44b45c6f05045e7b8c7b6257855`.

| Measured input | SHA-256 |
| --- | --- |
| Control `w1_capture` executable | `1f49b8bd44f7ff1b5472c827a0ef72bc4910842a6bd8b987d4ac45c5cb830745` |
| Candidate `w1_capture` executable | `634f4fe3b3959b402767a650d3508d94ee22b278b8357927db1bf9298b4810eb` |
| Candidate `outbound.rs` | `c573faafa4ea558d231b6986e5f8c00d7872905bd09cf5d02103e753ca44dce9` |
| Candidate `peer.rs` | `bdaa468de0c6625196113bd78c1952982eb5e11cc53aea89159b6e7d41fa7dfa` |
| Candidate `sent.rs` | `8214aef771646cadb56b19c89f65c9e29f629ab548707a702690bd4ada9813e8` |

Each of seven alternating fresh-process pairs runs the
existing four-peer real-W1 owned-projection fixture for 10,000 three-tick
windows after 10 warmup windows. The matched fixtures emit identical
frame counts, complete bytes and checksums across control and candidate.
The state-only median is 46.594 µs/window for control and 46.598 µs for the
candidate, with a paired median change of −0.16%. The result-traffic median
is 66.639 µs versus 66.969 µs, with a paired median change of +0.77%.
Pair changes span −2.51% to +1.20% and −1.06% to +1.77%, respectively;
these observations establish neither a speedup nor a regression.
Four peer owners plus shared scratch reserve 1,197,392 Rust bytes in control
and 1,197,456 in the candidate, exactly 16 additional inline metadata bytes
per peer and no new payload arena.

This timing includes W1 callback, projection, queue, sent transfer, framed
checksum and ACK for the selected event-free fixture. It does not measure
event-heavy captures, real transport, baseline/sync overlap, physical RSS,
allocator calls or a complete game callback tail. Those costs remain open.
