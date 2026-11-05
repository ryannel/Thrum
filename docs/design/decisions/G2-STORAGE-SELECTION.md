# G2R decision: owned slot columns and sparse history

5 November 2026. Use owned columns behind checked handles and prepared queries. Keep mover and static rows separate in the game workload; do not add archetype migration before a measured consumer needs it. C01/C02 keep game-owned types, one authority, atomic ticks and immutable pages.

## Comparison and qualification

An earlier paired M3 Max prototype measured owned SoA versus hecs prepared queries at 10k/100k: tick plus page p95 17.12/143.21 versus 26.67/245.54 µs; 10k eight-tick rewind p95 61.08 versus 181.00 µs. That prototype omitted the scheduler, validated edits, peers, checked handles and full pages, so it selected a direction but did not qualify the game.

The integrated game uses the same columns for serial and Rayon execution. The [single G2R evidence page](../../evidence/v1-workload/G2R-CONSOLIDATED-2026-11-05.md) owns all current p50/p95/p99, allocation counts, budgets and host-load qualification. The earlier churn excursion belongs to the p99 tail. The CPU budget remains open until one controlled full-command run passes every budgeted path.

## Representation and invariants

Typed batches bind a column and undo writer once. Undo rows retain moved old values and prior change ticks together; structural inverses retain slot generation, identity, free order and component ticks. Rewind spends each record once. Four page journals hold changed-slot bitsets and sorted structural slots. A reused page applies the current validated position batch when it covers earlier nonstructural changes; structural slots refresh separately. Otherwise the bounded union or full fallback refreshes the page. Published dirty slots stay sorted; unchanged components are not republished. An unavailable page defers the view, never the tick.

The game owns schema, input, rules and page layout. The engine owns generic columns, validation, schedule and bounded history. Serial and Rayon produce equal state after four-peer input, refusal, churn and replay. Revisit the executing render graph after the G2R CPU gate closes.
