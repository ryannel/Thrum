# G2R decision: owned archetype columns, provisionally selected

5 November 2026. **Owned SoA is provisionally selected for G2R, not yet qualified in the game.** The earlier hecs choice got the generic game running but misses new budgets. C01/C02 still require game-owned types, checked handles, one authority, atomic ticks and immutable pages.

## Evidence and cost

M3 Max full-game tick p50/p95: 129/135 µs at 10k, 1.31/1.35 ms at 100k. Eight-tick rewind: 3.22/3.63 ms at 10k with allocations. At 10k, publication costs roughly 60 µs, erased edit validation/application 51 µs, and motion 2.5 µs. A kernel-only swap cannot meet 25 µs.

A paired release prototype in `tmp/g2r-soa-spike` uses separate mover/static columns, 15% movers, sparse static edits, 1%/s churn, four changed-slot pages and in-place undo:

| Path | Owned SoA p50/p95 | hecs prepared-query p50/p95 |
| --- | ---: | ---: |
| 10k tick plus page | 13.75/17.12 µs | 21.25/26.67 µs |
| 100k tick plus page | 134.00/143.21 µs | 213.54/245.54 µs |
| 10k eight-tick rewind plus page | 49.83/61.08 µs | 167.92/181.00 µs |

All prototype samples allocated zero bytes and passed paired state/page/replay checks. It omits scheduler, validated edits, peers, checked handles, leases and full pages; hecs rewind uses sparse writes. SoA leaves about 8 µs at 10k for that omitted work. These numbers do **not** pass G2R.

## Selected operating model

Keep registration, schedule, permits, game rules and stable slot table. Use archetype-local typed columns, resolving each column once per query/batch. Keep movers and statics separate. Independent kernels use deterministic storage order; order-sensitive rules and published lists use canonical slots. Measure row-move ordering in the complete game. Preflight every typed and structural edit before direct column commits. No unchecked game storage access.

Refresh four immutable pages from bounded changed-slot journals; gaps and replay branches force a full baseline. Record preimages and structural inverses in a preallocated caller-selected undo window; benchmark eight ticks while retaining longer windows. Reverse undo restores rows, generations, IDs, free-list order, components and peer resources. Branches invalidate visual history, never held leases. Cold save capture may still copy.

## Qualification and reversal

Compare the same four-peer game at 10k/100k: churn, refusal, serial/Rayon, full state, sorted dirty lists and byte-identical replay. Measure complete tick+page and eight-tick rewind p50/p95, plus all-sample zero allocations; time baseline publication separately. Targets: 25/300 µs tick and 100 µs/1 ms rewind. Profile misses before final backend selection. Remove hecs from production after the game qualifies.
