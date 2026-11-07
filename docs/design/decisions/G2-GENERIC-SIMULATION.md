# G2 design: one registered simulation for the running game

5 November 2026. The original G2 correctness path ran in the game; **speed, rewind and multiplayer fitness are open under G2S**, which supersedes G2R. C01/C02 still require checked identity, one atomic fixed tick and immutable leased endpoints. [Storage selection](G2-STORAGE-SELECTION.md) provisionally chooses owned columns; only integrated game measurements can qualify them.

## Ownership and public composition

`thrum-core` owns checked slots/IDs, columns, schedule, atomic edits, change journal, page leases and undo. Games own types, callbacks, input/actions/rules and schema/quantization IDs. V1 separates movers/statics and keeps four controlled actors velocity-capable while idle. Engine APIs name no V1 field. Registrations freeze before tick one; changing them starts a new run until migration exists. A second game compiles without V1.

Registration binds process `TypeId` to stable schema ID and snapshot/undo operations; systems declare stage, ID, dependencies and typed accesses. The engine validates the access union and compiles conflict-free waves; structural edits and external resources need explicit capability. Callbacks see scoped queries and typed batches, never raw storage; sparse lookup checks `Handle`. Independent kernels use deterministic storage order; order-sensitive rules and published lists use canonical slot order. Measure after archetype row moves.

The engine owns a reusable operation queue. Sequential callbacks and ordered parallel merges receive only an
append-only `EditWriter`; each callback's new descriptors must use its own permits. Later callbacks cannot replace
or remove earlier operations. Candidate payloads and batch handles remain shared game-owned scratch, intentionally
evaluated at final preflight; this is capability routing, not per-system payload isolation. Trusted direct kernels
retain in-place sparse undo, without additional per-row validation.

V1's Resolve stages its values in a private parallel output and moves them into the shared payloads only in its
ordered merge, under its own permits. Independent row kernels use `WorldTxn::visit_chunked_indexed`:
- Word-aligned storage chunks own their values and active words.
- Each chunk's first-write preimages merge in storage order, and every chunk's preimages are kept on refusal.
- Serial runs the same chunks on the caller.
- `RayonForced` dispatches the chunks. Requested Rayon waits for a measured crossover.

## Tick, publication and replay

The input gate validates generic `InputFrame<GameAction>` identity, sequence, count and axes; the game validates actions. Frames remain pending until commit. Callbacks read committed state and write reusable typed candidate buffers; dependencies see staged values only through the candidate view. The engine validates outputs, checked identity, game-declared finite values and structural conflicts before applying. It never commits a partial candidate: apply in stable order, mark changes, consume inputs and advance the clock once. Refusal retains due tick and admitted frames. A busy page drops only a view; the next free page gets a baseline. Games declare read-model components; render and replication consume `SnapshotId`, dirty slots and journal, not another authority.

Prediction retains inputs and a caller-selected sparse undo window. Preflight validates and reserves; refusal leaves history intact. Commit moves old component values/ticks and records prior slot/ID/free-stack state. Undo preimages move; candidate value transfer is separate. The game stores post-commit peer state. Rewind checks the full revision chain, applies inverses only to the speculative branch, restores peers exclusively, clears pending admissions and re-admits retained inputs once. Confirmed state stays intact. Rewind bumps branch generation and rejects old projection/worker results and render history before polls. It patches an unleased page from the old revision chain at or after target tick, then publishes under the new branch generation. Undo journals rebuild sorted dirty slots and net structural changes; the patch names its source endpoint, so consumers apply it only to that endpoint or request a baseline. Held leases never mutate. With no eligible page, authority rewinds without allocation, reports a skipped view and may tick until a free page gets a baseline; measure deferred-frame latency. At 10k, eight-tick rewind plus eligible page must be <100 µs with zero warmed allocations; canonical replay matches serial/Rayon after churn. Record nondeterministic results; cross-device float identity is separate.

## Cost and migration gates

M3 Max release recorded tick+page p95 is 61/514 µs at 10k/100k (before sparse undo: 49–51/500–519 µs); eight-tick rewind+eligible page is 81/696 µs with zero warmed allocations (before: 0.82/9.4 ms). The [paired dense SoA spike](G2-STORAGE-SELECTION.md) is faster but omits generic costs. Avoid per-entity erasure, static scans, tick allocations and cold workers. Setup allocates; typed buffers reuse capacity. Measure tick, rewind, allocations, page bytes and serial/parallel crossover at 10k/100k. Engage parallel work only where measured faster.

V1 window, iOS, browser and projection consume one page contract. Next split dense mover/static rows; sparse undo and inverse page patch run in-game. G2R required 25/300 µs tick+page and 100 µs/1 ms eight-tick rewind+eligible page p95 at 10k/100k, zero warmed allocations, serial/Rayon equality and byte-identical replay. Held/no-eligible cases must retain authority ticks and report deferred-frame latency. `./dev check` must pass. G3 then executes the graph; `FrameHost` moves after the display-link loop.
