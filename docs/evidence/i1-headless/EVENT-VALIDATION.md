# Replica event validation without a temporary event tree

1 November 2026. Baseline and live replica admission now use one event-link validator. Events must have strictly increasing IDs within the committed cursor and a valid tick. Each result that names an event binary-searches that sorted column and checks exact origin and resolved tick. Rejected results cannot name events. The number of valid references must equal the event count. Independent result-identity validation prevents two otherwise valid results from consuming one event. The former temporary event-ID `BTreeMap` and its per-event node allocation are gone from both prototype and charged group validation; malformed groups still fail before confirmed state changes.

The [game fixture](../../../crates/thrum-core/tests/i1_game.rs) checks wrong event origin, wrong event tick, an orphan event, duplicate live result reference, malformed baseline event links, and a two-event baseline whose two results both reference the first event. That last case has matching reference and event counts, so only duplicate result identity can reject it. The delayed-gap, two-peer and sealed-host tests still pass under `RUSTC_WRAPPER= ./dev check`. The review-agent found no acceptance or atomicity issue in the change.

The [isolated release fixture](../../../crates/thrum-core/examples/i1_event_validation_cost.rs) constructs 11,000 one-event groups before timing, applies 1,000 warmups and records 10,000 replica `apply` calls. It uses one peer, no changed objects and immediate sequential results; the measured interval includes validation and admitted-result construction but excludes group creation. The same fixture source was run on macOS/aarch64 against previous commit `1b09da1` and the new source. Three runs, including a reversed order, gave p50 nanoseconds:

| Previous event tree | Sorted-column lookup |
| ---: | ---: |
| 167 / 209 / 250 | 125 / 167 / 167 |

The isolated median was lower in each pairing, with local scheduler and allocator variation. The existing [changed-state fixture](../../../crates/thrum-core/examples/i1_charged_replica_cost.rs) has no events; its complete submit/step/replica/ack times varied across runs and cannot attribute a whole-path gain to this change. The charged path calls the same validator, but eventful charged-group and device timing remain unmeasured. Peer-progress cloning, duplicate changed-ID scratch, future output byte credits and the complete process memory gate remain open.
