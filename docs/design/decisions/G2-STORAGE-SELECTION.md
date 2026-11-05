# G2R decision: owned slot columns and sparse history

5 November 2026. Use owned columns behind checked handles and prepared queries. Keep mover and static rows separate in the game workload; do not add archetype migration before a measured consumer needs it. C01/C02 keep game-owned types, one authority, atomic ticks and immutable pages.

## Comparison and qualification

An earlier paired M3 Max prototype measured owned SoA versus hecs prepared queries at 10k/100k: tick plus page p95 17.12/143.21 versus 26.67/245.54 µs; 10k eight-tick rewind p95 61.08 versus 181.00 µs. That prototype omitted the scheduler, validated edits, peers, checked handles and full pages, so it selected a direction but did not qualify the game.

The integrated game uses the same columns for serial and Rayon execution. The [single G2R evidence page](../../evidence/v1-workload/G2R-CONSOLIDATED-2026-11-05.md) owns all current p50/p95/p99, allocation counts, budgets and host-load qualification. The earlier churn excursion belongs to the p99 tail. The CPU budget remains open until one controlled full-command run passes every budgeted path.

## Representation and invariants

Typed `&mut A`/`&B` queries bind disjoint columns and an undo writer once, then visit storage order without game-owned staging. A tick transaction records first-write values and prior change ticks; structural inverses retain slot generation, identity, free order and component ticks. Refusal restores tentative writes; only a complete tick merges engine-owned per-component and structural dirty unions. Rewind spends each record once and contributes reversed changes. A released page refreshes affected rows from its own version, while publication diffs against the last published endpoint and retains the final tick's preimages for an adjacent pair. Page lifecycle output is the net endpoint transition, with Despawn(old) before Spawn(new) on reuse; separate consumers need a bounded per-tick event log or resync. An unavailable page defers the view, never the tick; a gap does not force a full baseline.

The game owns schema, input and rules. The engine owns generic columns, page layout, publication, validation, schedule and bounded history. Serial and Rayon produce equal complete state after four-peer input, refusal, churn and replay. G2S remains open until the fixed real-game rows pass.
