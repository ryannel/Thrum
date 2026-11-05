# G2R decision: owned slot columns and sparse history

5 November 2026. Use owned columns behind checked handles and prepared queries. Keep mover and static rows separate in the game workload; do not add archetype migration before a measured consumer needs it. C01/C02 keep game-owned types, one authority, atomic ticks and immutable pages.

## Comparison and qualification

An earlier paired M3 Max prototype measured owned SoA versus hecs prepared queries at 10k/100k: tick plus page p95 17.12/143.21 versus 26.67/245.54 µs; 10k eight-tick rewind p95 61.08 versus 181.00 µs. That prototype omitted the scheduler, validated edits, peers, checked handles and full pages, so it selected a direction but did not qualify the game.

The integrated game now uses the same columns for serial and Rayon execution. Release measurements on M3 Max, 10k with 1.5k movers and 100k with 15k movers, warmed history and page ring:

| Path | Before p95, 10k/100k | After p95, 10k/100k | Budget |
| --- | ---: | ---: | ---: |
| Recorded tick plus page | 61/514 µs | 21/213 µs | 25/300 µs |
| Recorded tick plus page, one static despawn/spawn each tick | 35/257 µs before final undo packing | 24.9/216 µs | 25/300 µs |
| Eight-tick rewind plus eligible page | 81/696 µs | 73/703 µs | 100/1000 µs |

The churn run sampled 10,000/2,500 ticks at 10k/100k. One shorter 10k run measured 27.2 µs p95; the other four measured 21.8–22.7 µs. The probe counted zero allocations in every warmed sample. A committed Divan churn bench and a 10k recorded-churn allocation test retain the workload. These are CPU measurements, not device frame or visual qualification.

## Representation and invariants

Typed batches bind a column and undo writer once. Undo rows retain moved old values and prior change ticks together; structural inverses retain slot generation, identity, free order and component ticks. Rewind spends each record once. Four page journals hold changed-slot bitsets and sorted structural slots. A reused page applies the current validated position batch when it covers earlier nonstructural changes; structural slots refresh separately. Otherwise the bounded union or full fallback refreshes the page. Published dirty slots stay sorted; unchanged components are not republished. An unavailable page defers the view, never the tick.

The game owns schema, input, rules and page layout. The engine owns generic columns, validation, schedule and history. Serial and Rayon produce equal state after four-peer input, refusal, churn and replay. The next performance gate is the executing render graph, not another storage rewrite.
