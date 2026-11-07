# G2S: chunked parallel movers and Resolve staging — 7 November 2026

This page records the work that closes G2S. It covers game-side Resolve staging, per-system permits, and real
parallel work behind `ExecutionMode`. It also covers complete authoritative-state equality after correction, structural
churn and refusal. Root approved closing on these terms (7 November):
- the correctness evidence here;
- requested `Rayon` as the measured **serial fallback**;
- `RayonForced` as the tested parallel correctness path.

The quiet ABBA rows below found no crossover, so neither crossover constant is set. Every other timing on this page
comes from a shared Mac and qualifies nothing.

## What changed

- **Chunked in-place kernel.** This is `WorldTxn::visit_chunked_indexed` over the engine's `visit_mut2_chunked`.
  - The kernel splits an indexed `&mut T`/`&U` query into word-aligned chunks of 4,096 rows. Each chunk exclusively
    owns its values and active-index words.
  - Each chunk keeps its first-write preimages privately. The caller merges them into the transaction undo and
    dirty marks in chunk order, which is storage order.
  - On refusal, every chunk's preimages are still merged, so the abort restores rows any chunk wrote. The earliest
    chunk's error is returned.
  - Chunks stop at the last active word. The activity partition keeps movers in a dense prefix, so the 100k
    fixture's 15k movers fill four chunks.
  - Serial runs the same chunk code on the caller. Browser builds always run serially.
- **V1 Simulate** integrates movers through this kernel, and the change count comes from the engine.
- **Resolve staging.**
  - Resolve is now a `RegisteredSystem::parallel` callback. It reads the tentative world after Input and Simulate,
    and this tick's commands. From those it fills a private `ResolveStage`: overrides, base values, velocity
    insert/replace/remove decisions and despawn skips.
  - Simulate no longer writes Resolve's values; it previously staged SetPosition values and despawn marks.
  - The ordered `merge_parallel` moves the stage into the shared payloads that descriptors read at final preflight.
    It appends descriptors with Resolve's permits only, in the old canonical order.
- **Permits.** These are held per system: `InputVelocityPermits`, `InputHealthPermits` and `ResolvePermits`. The
  engine already refuses a whole tick when a callback appends with a foreign permit
  (`g2_edit_ownership.rs`).
- **Rayon policy.**
  - `RayonForced` always dispatches the kernel.
  - Requested `Rayon` is a **serial fallback** today. It would dispatch at or above `PARALLEL_MOVER_CROSSOVER`, but that is
    `None`: the quiet rows below show no win.
  - `Simulation::set_execution_mode` no longer rewrites Rayon to Serial.

## Correctness evidence

| Proof | Result |
|---|---|
| `g2s_chunked_kernel::forced_rayon_runs_independent_chunks_at_the_same_time` | Forced Rayon overlaps two or more chunk callbacks: the peak concurrency probe reads at least 2. Serial reads exactly 1. Heights are identical across modes. |
| `g2s_chunked_kernel::earlier_writers_and_unchanged_rows_restore_exactly_in_every_mode` | An earlier Input system writes rows in three chunks before the kernel visits them, and every fifth row returns unchanged. Refusal and rewind restore the exact pre-tick values, not the earlier writer's. Unchanged rows are neither counted nor given preimages. All three modes are equal, in release builds too. |
| `g2s_chunked_kernel::serial_rayon_and_forced_chunks_commit_refuse_and_rewind_identically` | 8,192 rows interleave movers and statics under a dense activity partition, with one excluded slot. The visit set is exact. A refusal from the last mover restores every chunk. A recorded tick rewinds to the exact prior state and replays identically. All three modes are equal. |
| `simulation::parallel_movers_match_the_straight_corrected_run_after_churn_and_refusal` | 100k fixture, four peers and 12 ticks. A kernel refusal comes from an overflowing mover in a high chunk (25 chunks that tick). A Resolve NaN refusal comes after all four chunks wrote. Each refusal leaves the complete-world hash unchanged. Despawn and spawn churn run every tick. The run rewinds eight ticks, amends peer 2's tick-5 input and replays. Midway through the replay, a Resolve refusal on a deferred-publication tick leaves the hash and the remaining retained ticks intact. In Serial, requested Rayon and RayonForced, the complete authoritative hash equals the straight corrected run, and so does the published page. That hash covers identity, presence, every component value, authority metadata, and the game's peer and input resources. The requested-Rayon row checks fallback routing only; it runs the Serial path. RayonForced is the row that runs the parallel path. |
| `history::tests::a_misrouted_permit_refuses_the_whole_tick` | Resolve's health edit, appended with Input health's permit, refuses the whole tick as `InvalidEdit`, and authority is unchanged. With its own permit the same command commits. |
| `g2s_chunked_kernel::a_kernel_panic_restores_every_chunk_in_every_mode` | A panic in the last chunk's callback still merges every chunk's preimages. The preparation abort restores all rows, and the next tick matches a straight run in Serial and RayonForced. |
| The other `simulation` tests (42 in the file, including the two above and the page-capture row below) | They pass unchanged: 1,000-tick randomized churn replay, 10k corrected replay, four-peer Serial/RayonForced equality, held pages, and zero warmed caller-thread allocations. |

The kernel now reports how many chunks it dispatched (`ChunkedVisit::dispatched_chunks`). V1 counts committed
ticks whose kernel ran on the pool (`Simulation::kernel_dispatches`, a diagnostic that is never captured). The 100k
run asserts which path each mode took:
- `RayonForced` dispatched on all 20 committed ticks: 12 forward and 8 replayed.
- Serial and requested `Rayon` dispatched on none.

The requested-Rayon assertion is written to fail once a crossover is promoted, so that row must then prove the
requested path. Overlap of two chunk callbacks is asserted only in the 8,192-row core fixture.

## Contended crossover rows

These come from `cargo bench -p v1-game --bench g2r -- --crossover` on an M3 Max. Each row times one full tick plus
publication. Other agents and Codex threads were active, with load averages of 21–40. There were two runs each
before and after bounding the chunks; raw logs are in `tmp/g2s-parallel/crossover-{1,2}.log` (local, main checkout).

| Rows | Serial p50 µs | Rayon p50 µs | RayonForced p50 µs | Forced all-thread allocations per run |
|---|---:|---:|---:|---:|
| 10k (one chunk, no dispatch) | 52–55 | 52–57 | 53–55 | 0–28 |
| 100k | 543–568 | 968–2,055 | 1,066–1,408 | 16 |
| 200k | 1,234–1,269 | 1,793–2,899 | 1,946–2,266 | 16 |
| 500k | 3,023–3,356 | 4,047–4,674 | 4,521–5,194 | 16 |

The Rayon column predates the gate in its first run. These rows support neither "parallel is slower" nor "parallel
is faster": with 20–40 runnable threads, pool workers are preempted. Two things hold regardless of contention:
- Publication dominates the tick. The mover kernel is a small share at the fixture's 15% mover density.
- Forced dispatch from the world-tick thread allocates about once per Rayon injection block. The caller thread stays
  allocation-free.

## Contended ABBA rows

`cargo bench -p v1-game --bench g2r -- --crossover-abba` keeps a Serial world and a RayonForced world warm and runs
them in eight A-B-B-A rounds of interleaved blocks, so a drift in host load hits both modes alike. It prints the host
load before and after, plus each mode's lowest and highest per-round p50. A win must clear that spread. The one run
so far is below, taken at load averages of 17–24 with agents active; it qualifies nothing. The raw log is
`tmp/g2s-parallel/abba-contended-2.log` (local, main checkout).

| Rows | Serial p50 µs | Serial round p50 range | Forced p50 µs | Forced round p50 range | Forced allocations |
|---|---:|---:|---:|---:|---:|
| 10k | 52 | 48–113 | 52 | 48–115 | 0 |
| 100k | 526 | 493–544 | 976 | 704–1,781 | 8 |
| 200k | 1,091 | 1,044–1,141 | 1,500 | 1,385–1,794 | 8 |
| 500k | 2,780 | 2,672–2,992 | 3,179 | 3,001–3,598 | 8 |

Under this contention Forced is slower at every row, and its rounds vary far more than Serial's. That is consistent
with preempted pool workers. The quiet run uses the same command with agents paused.

## Contended capture share

`cargo bench -p v1-game --bench g2r -- --capture-share` runs two warm Serial worlds through the same ABBA rounds. In
one world all four pages are held, so the engine skips page capture. Change marking for publication still runs in
both. The difference is page capture's share of the tick. The raw log is `tmp/g2s-parallel/capture-share-3.log`
(local, main checkout), taken at load 20–22, so it qualifies nothing.

| Rows | Captured p50 µs (round range) | Pages held p50 µs (round range) | Capture share |
|---|---:|---:|---:|
| 10k | 51 (48–52) | 26 (25–28) | about half |
| 100k | 518 (494–537) | 262 (249–282) | about half |

Page capture is about half the Serial tick at both sizes, and the mover kernel is only part of the remainder. Parallel
capture, by column and by word range within the position column, is therefore the larger parallel candidate.

The first two deferred ticks after the pages fill allocate once each, as the two swapped undo buffers grow. That cost
comes from keeping deferred-publication undo apart from transaction scratch (ultra #1). Later deferred ticks do not
allocate: the held rows above show zero.

## Parallel page capture

Warm page capture of a selected column can now run on the pool. The engine owns this inside its publication internals,
and the page contract is unchanged.
- A serial pass buckets each candidate slot by the 4,096-row page chunk that owns its row. It records refresh, pair and
  identity flags with each slot.
- Each chunk exclusively owns its rows of current, previous and override storage, and visits only its candidates.
- The merge rebuilds the endpoint and pair words, and the slot-ordered dirty lists, exactly as the serial loop does.
- The engine dispatches a column only when it has at least one chunk's worth of candidates. At the fixture's 15% mover
  density, that is Position from about 30k rows.

`RayonForced` always takes this path. Requested `Rayon` captures serially until `PARALLEL_CAPTURE_CROSSOVER` is set by a
quiet win. Browser builds stay serial.

`simulation::parallel_page_capture_publishes_identical_pages_in_every_mode` runs a 40k fixture in Serial, Rayon and
RayonForced in lockstep:
- despawn, spawn and position-command churn every tick;
- held pages that force reuse and catch-up;
- a five-tick rewind patch, then an amended replay.

After every step it compares the complete reader view of the published page:
- id, timeline and baseline flag;
- every slot's handle, stable id, position, previous position, velocity and health;
- the position, position-pair, velocity, health and structural dirty lists;
- the structural events.

The new `DirtySlots::position_pair` exposes the engine's temporal pair list for this test. Two deliberate breaks in the
parallel path were each caught at the first warm tick: dropping pair-changed flags, and skipping one row's refresh.

**Proof the parallel path ran.** `PublicationPage::parallel_capture_columns` is a diagnostic, not page content. The test
asserts:
- Serial and requested Rayon dispatched no capture.
- RayonForced dispatched on warm forward ticks and again during the amended replay.

In this fixture only the temporal Position column dispatches, always under the activity-partition directory.

`page_tests::parallel_warm_capture_matches_serial_for_endpoint_and_temporal_columns_without_a_directory` covers the
branches the V1 fixture never reaches in parallel:
- an endpoint-only column;
- no directory;
- page, source and world on the same layout.

It runs 9,000 rows in three chunks, the last one partial. Movement then stopping changes pairs without changing
endpoints. Each step compares every row, every previous value, all dirty lists and the events with a serial ring, and
both columns dispatch. A deliberate break of the endpoint-only branch fails it.

Each chunk's candidate list is reserved once, at capacity, on its first parallel capture, so later ticks do not grow it.
The lists count in the page's reserved bytes.

The contended ABBA below compares Serial with RayonForced, which now runs both the parallel kernel and parallel capture.
It ran at a 1-minute load average of 17. The raw log is `tmp/g2s-parallel/abba-capture-2.log` (local).

| Rows | Serial p50 µs (round range) | Forced p50 µs (round range) |
|---|---:|---:|
| 10k (below the candidate floor, no capture dispatch) | 49 (48–49) | 49 (49–49) |
| 100k | 488 (486–491) | 1,207 (1,058–1,482) |
| 200k | 978 (972–1,078) | 1,703 (1,459–2,790) |
| 500k | 2,507 (2,465–2,576) | 3,024 (2,946–3,157) |

Under this load, Forced is slower than Serial at every dispatched row. A 1-thread-pool spot check pointed the same way,
but its output was not kept, so this page draws no conclusion from it.

An earlier version visited every page row in each chunk, rather than only the bucketed candidates. It cost 2.7 ms at
100k at a 1-minute load of about 30 (`abba-capture-1.log`), so it was replaced.

## Quiet ABBA rows

This was an owner-arranged quiet window: every agent was holding, the solvers were suspended and the GPU was idle. There
were two runs of `cargo bench -p v1-game --bench g2r -- --crossover-abba` on main `961541f0`, an M3 Max. The 1-minute
load average was 2.20 before run 1 and 2.10 between the runs. It was 2.01 after run 2. Forced runs both the parallel
kernel and parallel capture. The raw logs are `tmp/g2s-parallel/abba-quiet-{1,2}.log` (local).

| Rows | Serial p50 µs, run 1 / run 2 (round p50 range) | Forced p50 µs, run 1 / run 2 (round p50 range) | Forced allocations |
|---|---:|---:|---:|
| 10k (no dispatch) | 47 / 47 (47–47) | 47 / 47 (47–49) | 0 |
| 100k | 463 / 460 (453–476) | 652 / 660 (629–705) | 16 |
| 200k | 926 / 907 (906–947) | 1,137 / 1,116 (1,101–1,153) | 16 |
| 500k | 2,285 / 2,297 (2,282–2,358) | 2,364 / 2,406 (2,344–2,578) | 16 |

**There is no crossover.** Forced is slower than Serial at every dispatched row, in both runs. At 100k and 200k the gap
is wider than the round spread. The gap is 190–200 µs at 100k and about 210 µs at 200k. At 500k it is 80–110 µs: in run 1
Forced's rounds sit entirely above Serial's, and in run 2 they overlap by about 1 µs. The parallel work does gain at
500k, but it never pays back its fixed cost per tick at these densities.

The 5- and 15-minute load averages, about 14 and 24, show the window had only just gone quiet. A noisier window would
favour neither mode, because the ABBA interleaves them.
`PARALLEL_MOVER_CROSSOVER` and `PARALLEL_CAPTURE_CROSSOVER` stay `None`, and requested `Rayon` remains the documented
serial fallback.

## Open, for root and the performance phase

These do not block G2S, which is closed on the terms above.

1. **Crossover promotion.** No win at 100k–500k. Promoting either crossover later needs a new quiet win.
2. **One install per tick.** This is a performance-phase candidate. Under root's verdict, it should be tried only if a
   profile of a representative tick shows injection or pool wakes as the limiting cost. The resident world lane keeps
   ownership of the tick and the lifecycle.
3. **Caller-thread allocations under forced capture.** The first parallel capture of each column reserves its chunk
   lists once. Requested Rayon never takes this path.
