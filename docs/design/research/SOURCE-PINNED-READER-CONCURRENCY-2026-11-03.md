# Accepted-source reader progress during admission

3 November 2026. Performance design for the [selected source owner](SOURCE-OWNER-RECOVERY-RECONCILIATION-2026-11-03.md), under [C10](../CONTRACTS.md) and the [performance policy](../AI-PERFORMANCE.md#performance-investment-from-the-first-design). This is a proposed next implementation slice, not a completed concurrency guarantee.

## Observed cost and required behavior

The [local owner cost record](../../evidence/i2a/SOURCE-OWNER-LOCAL-COST-2026-11-03.md) times a 12 MiB admission at about 86–89 ms. A coordinated existing A lease read waits about 87–89 ms behind B because public admission holds the one SQLite-core mutex from preflight through file staging and final commit. This is authoring/build work, outside world ticks, but larger edits can make an old admitted build wait. The local fixture does not measure representative reader tails or memory peaks.

An improvement must retain one cooperating process owner, one final root/outcome transaction, exact A reads while B stages, and a recovery-required barrier after an uncertain final COMMIT. The append-only first profile keeps old blobs; no reader may observe an uncommitted B as selected. A read pin remains durable through process interruption. A reader may not hold a SQLite transaction across consumer output, because that can strand WAL checkpoint debt. Space planning must count WAL/main overlap and retained old closure. The design adds no operation to simulation ticks.

## Options

| Shape | Benefit | Cost and failure surface |
| --- | --- | --- |
| Keep one mutex through admission | Smallest recovery state and one connection | Reader wait grows with staged bytes, hash work and SQLite sync; the local 12 MiB case already approaches 90 ms. |
| One admission permit, release the core mutex between completed file or chunk transactions | Existing SQLite connection, owner lock and final quarantine remain single; old pinned reads can interleave at bounded phase boundaries | Requires explicit admission state and phase ownership, careful tiny-file batch and retry handling, and tests that pin cleanup/checkpoint cannot invalidate in-progress admission. More mutex handoffs are paid per staged chunk, not per tick. |
| Separate owner read-only SQLite connection | Writer admission changes less and old leases can read in parallel with staging | Adds one connection/cache and WAL reader state; a shared recovery latch and exact close/quarantine ordering become more complex. Read-only open and close must be proved not to checkpoint uncertain WAL. |

Select the **admission-permit/short-core-phase prototype** first. A single writer permit excludes overlapping admissions from preflight through final commit, while the core mutex protects each SQLite phase. Release it only after a committed blob batch/chunk and before CPU hashing or consumer work; never across an open SQLite transaction or uncertain final COMMIT. Keep pin registration/release serialized with core SQL and preserve the current writer's final generation CAS. Do not convert `PinnedSource` into a second database authority. If the prototype requires holding the core mutex for one whole large file or adds excessive transactions, compare the read-only connection before adopting it.

The cost account is one extra mutex handoff per committed stage phase, a small admission-permit owner, and no extra retained file buffer beyond the current 4 MiB scratch. The expected upside is giving an existing pin opportunities to read between writer phases. Mutex reacquisition can still starve a reader across many phases, so phase length alone cannot establish a wait bound. Admission wall time, SQLite WAL growth, allocation count, retained scratch, RSS, reader wait distribution and final-commit fault behavior all need paired evidence.

## Verification before promotion

1. Keep A pinned; stage B with at least three 4 MiB chunks while an A read starts after B's preflight. Check A's exact bytes and measure the reader wait distribution, repeated writer reacquisition/starvation and B admission time against the current owner in repeated optimized runs. Include tiny-file batches and an old pin's close. No timing threshold is declared from one local host.
2. Inject stale generation, source drift, `SQLITE_FULL`, reader-blocked checkpoint and pre/post-sync final-COMMIT faults at each new phase boundary. Precommit refusal leaves A selected; an uncertain final commit quarantines every later owner SQL and resolves the exact B key only on fresh open.
3. Kill a process after a completed B blob phase and after public B commit. Reopen distinguishes no B outcome from committed B; same-key retry is exact and A's retained source remains readable.
4. Report stage/validation/admission/cook timing, allocator requests and peak RSS with retained A plus B at the selected larger profile. Compare selected-root identity, source bytes and WAL/main high-water, not only mean throughput. Qualify the initial macOS profile first; other platforms require their own filesystem and lock measurements.

Do not change the public accepted-source/root/lease identities to implement this optimization. The current single-mutex path remains valid until the prototype passes its recovery and performance gates.
