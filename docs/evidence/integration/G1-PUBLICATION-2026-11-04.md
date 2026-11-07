# G1: one world page into the V1 game

4 November 2026. Apple M3 Max, macOS Metal, Rust 1.99.1, 10,000 entities/1,500 movers. This is one local development target, not iPhone or V1 frame-budget qualification. The decision is [G1 publication spine](../../design/decisions/G1-PUBLICATION-SPINE.md).

`Simulation` now seals tick zero, then publishes one immutable complete slot-indexed page and per-component dirty sets on every tick. Generation checks and an ordered structural journal survive destruction/reuse. A held page can refuse a tick with `PublicationBusy` before authority mutation, leaving the clock's due tick for exact retry. The game host renders that page and, when enabled, copies each tick's projection to an ordered four-slot outbox; an overrun requires an explicit full baseline. Render history keys on slot/generation and the last **applied** scene controls retire/hide, including discarded worker results. The old visual pool, full-record snapshot API and stable-ID render indexes are gone. Up to eight actions fit inline in `InputFrame`; no action allocation was observed after warmup.

| Diagnostic | Result |
| --- | --- |
| G0 same-process W1 tick + visual-only capture | 44.41 + 32.52 = 76.93 µs median; historical separate run |
| First G1 full-copy tick + publication | 119.5 µs median; rejected representation |
| Final G1 incremental tick + publication, Divan | 76.16 µs median; 71.87 µs fastest, 8.015 ms slowest in 5,398 samples |
| Isolated 10k render extraction, Divan | 71.58 µs median; older ID-map diagnostic 99.70 µs in a separate run |
| Reserved four-page payload/owners | 10,001,488 bytes on this 64-bit build, excluding allocator bookkeeping |
| Optional outbox after baseline + three moving ticks | 4,073,984 reserved bytes; local game leaves it disabled without a session |
| Scoped allocator assertions | Zero allocations after warmup for 10k tick/churn publication and admitted action frames |

Two separate release window processes each presented 600 frames on the real Metal surface. Run A/B respectively: callback CPU p50/p95 **0.264/0.818** and **0.240/0.621 ms**; tick CPU **104.625/193.167** and **103.666/129.833 µs**; scene GPU **0.375/1.977** and **0.374/2.420 ms**; surface GPU **0.091/0.378** and **0.091/0.440 ms**. They committed 628/605 ticks and dropped 22/2 optional views with no surface faults or dropped reports. The wide GPU tails and differing dropped-view counts remain unqualified pacing behavior; these runs do not establish a V1 budget or a before/after speedup.

`./dev check` covers strict workspace Clippy, repo guards, Wasm compilation and Rust tests. The explicit Metal host test passed with tick-zero plus three ordered outbox projections alongside GPU frames; the Metal upload test verified retired pixels disappear and reappear on reuse. Read-only review-agent cross-module passes found no remaining concrete defect after fixes for first-tick structural history, outbox ordering/gaps, sparse high slots and worker-discard retirement. G2 still owns generic game schema, restore/replay, longer history and the 10k/100k hecs-versus-SoA decision. G6 owns real transport and codec; a physical iPhone has not run this game.
