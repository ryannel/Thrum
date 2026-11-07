# G2S: chunked parallel movers and Resolve staging — 7 November 2026

This page closes the remaining G2S items: game-side Resolve staging, per-system permits, and real parallel work behind
`ExecutionMode`. It also covers complete authoritative-state equality after correction, structural churn and
refusal. These are correctness results. All timings below are from a heavily shared Mac, so they qualify nothing.

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
  - Requested `Rayon` dispatches only at or above `PARALLEL_MOVER_CROSSOVER`. That is `None` until a quiet measured
    win promotes it (see the rows below).
  - `Simulation::set_execution_mode` no longer rewrites Rayon to Serial.

## Correctness evidence

| Proof | Result |
|---|---|
| `g2s_chunked_kernel::forced_rayon_runs_independent_chunks_at_the_same_time` | Forced Rayon overlaps two or more chunk callbacks: the peak concurrency probe reads at least 2. Serial reads exactly 1. Heights are identical across modes. |
| `g2s_chunked_kernel::earlier_writers_and_unchanged_rows_restore_exactly_in_every_mode` | An earlier Input system writes rows in three chunks before the kernel visits them, and every fifth row returns unchanged. Refusal and rewind restore the exact pre-tick values, not the earlier writer's. Unchanged rows are neither counted nor given preimages. All three modes are equal, in release builds too. |
| `g2s_chunked_kernel::serial_rayon_and_forced_chunks_commit_refuse_and_rewind_identically` | 8,192 rows interleave movers and statics under a dense activity partition, with one excluded slot. The visit set is exact. A refusal from the last mover restores every chunk. A recorded tick rewinds to the exact prior state and replays identically. All three modes are equal. |
| `simulation::parallel_movers_match_the_straight_corrected_run_after_churn_and_refusal` | 100k fixture, four peers and 12 ticks. A kernel refusal comes from an overflowing mover in a high chunk (25 chunks that tick). A Resolve NaN refusal comes after all four chunks wrote. Each refusal leaves the complete-world hash unchanged. Despawn and spawn churn run every tick. The run rewinds eight ticks, amends peer 2's tick-5 input and replays. In Serial, Rayon and RayonForced alike, the complete authoritative hash (identity, presence, every component value and authority metadata) and the published page equal the straight corrected run. |
| Existing 39 `simulation` tests | They pass unchanged: 1,000-tick randomized churn replay, 10k corrected replay, four-peer Serial/RayonForced equality, held pages, and zero warmed caller-thread allocations. |

A temporary chunk-count trace showed the 100k `RayonForced` run dispatching 21 kernel runs: 20 over four chunks and
one over 25. Requested Rayon dispatched none. The trace was removed.

## Contended crossover rows

These come from `cargo bench -p v1-game --bench g2r -- --crossover` on an M3 Max. Each row times one full tick plus
publication. Other agents and Codex threads were active, with load averages of 21–40. There were two runs each
before and after bounding the chunks; raw logs are in `tmp/g2s-parallel/crossover-{1,2}.log` (local).

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

## Open, for root and the performance phase

1. **Quiet crossover.** Promote requested Rayon only on an owner-quiet ABBA win: the same command at 100k–500k,
   agents paused.
2. **Injector allocations.** Running the world tick inside the Rayon pool would remove these. That is the world-thread
   adapter, so it is a root seam.
3. **Parallel publication capture.** This is the larger independent work at 100k and later the better parallel
   candidate. It is not attempted here.
