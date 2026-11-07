# G2S storage decision: dense motion rows and sparse history

5 November 2026. Keep owned columns, checked logical handles and sparse history. The fixed S1 workload exposes 15k movers scattered through 100k slots. Provisionally select an engine-owned active/inactive motion partition **in both authority and publication**. C01/C02 retain one authority, atomic ticks, checked identity and exact source/previous page semantics. This is not a general archetype system or a game-maintained mover marker.

## Comparison and qualification

The earlier paired M3 Max prototype measured owned SoA versus hecs at 10k/100k: tick plus page p95 17.12/143.21 versus 26.67/245.54 µs. It omitted the scheduler, peers, checked handles and full pages, so it selected ownership but did not qualify the game. Current integrated shuffled S1 is roughly 0.06/1.4 ms p95 at 10k/100k under concurrent host load; S2 is roughly 0.4 ms. A 4,519-sample Instruments run attributed 1,713 inclusive samples to Position publication and 1,398 to the typed motion query. These are diagnostic, not gate results.

The [G2R evidence page](../../evidence/v1-workload/G2R-CONSOLIDATED-2026-11-05.md) retains the earlier rows. The fixed G2S rows, not that page, own promotion.

## Representation and invariants

Logical slot, generation, ID index and free order stay stable. Bidirectional logical-slot/row directories locate Position, Velocity and their hot change metadata. The registered activity predicate selects an active prefix. An activity transition swaps rows at the partition boundary after a mutable query completes. Sparse undo records inverse swaps and first-write preimages; refusal and rewind restore exact storage order. Typed queries visit dense rows deterministically. Canonical reductions and published dirty lists retain logical-slot order.

Each leased page owns its immutable row directory and values. Refresh repairs moved neighbors by logical identity, while endpoint and adjacent-pair differences compare with the last published page by logical identity. Layout movement alone is not a component change. Final-tick preimages supply the previous pose, including delayed publication. Structural reuse emits Despawn(old) before Spawn(new). An unavailable page defers the view, never the tick; a gap does not force a baseline.

The game owns schema, input and rules. The engine owns layout, page capture, typed access, sparse undo and history. Preallocate migration and structural scratch for the 200-spawn path. Prove authority packing and page packing separately, then together on the **unchanged** S1/S2 rows. Test activation, deactivation, canceled changes, boundary deletion, reuse, held pages and rewind against a slot-based oracle. Parallel contiguous work follows only after an integrated win. G2S remains open until its 25/300 µs S1 and 100 µs S2 p95 limits, zero warmed allocations and full-state replay equality pass with `./dev check` green.
