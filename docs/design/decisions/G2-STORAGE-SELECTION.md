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

Keep registration, schedule, permits, rules and slots. Integrated slot columns use presence bitmaps and checked handles; page fallback scans all slots and edits remain per-item erased. Split movers/statics and resolve columns once per typed batch; measure row moves if archetypes replace slot indexing. Preflight all edits before commit.

Refresh four immutable pages from bounded changed-slot journals; an old page gets a full refresh, while skipped publication or a replay branch starts a baseline. Record preimages and structural inverses in a preallocated caller-selected undo window; benchmark eight ticks while retaining longer windows. Reverse undo restores rows, generations, IDs, free-list order, components and peer resources. Branches invalidate visual history, never held leases. Cold save capture may still copy.

## Qualification and reversal

M3 Max owned-column full-game p50/p95: 68/69–71 µs at 10k, 700/708–710 µs at 100k, versus paired hecs 123/127 and 1240/1254–1274 µs. Sampled allocations peak at zero; churn, refusal, serial/Rayon and page tests pass. Both tick budgets remain open. Compare the same game and exact replay after typed batches and in-place undo; targets remain 25/300 µs tick and 100 µs/1 ms eight-tick rewind. Profile misses before final layout selection.
