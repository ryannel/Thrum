# Accepted-source reader progress during admission

3 November 2026. Performance design for the [selected source owner](SOURCE-OWNER-RECOVERY-RECONCILIATION-2026-11-03.md), under [C10](../CONTRACTS.md) and the [performance policy](../AI-PERFORMANCE.md#performance-investment-from-the-first-design). The first admission-permit/short-core-phase prototype is implemented and locally qualified in the [phased proof](../../evidence/i2a/SOURCE-PHASED-ACCEPTED-ADMISSION-2026-11-03.md). Larger-source tails and complete whole-path cost remain open.

## Observed cost and required behavior

The [serialized owner cost record](../../evidence/i2a/SOURCE-OWNER-LOCAL-COST-2026-11-03.md) timed a 12 MiB admission at about 86–89 ms. A coordinated existing A lease read waited about 87–89 ms behind B because admission held the one SQLite-core mutex from preflight through file staging and final commit. The first phased path gave old readers opportunities to copy between committed chunks; a 9–10-read local sample saw maximum copy times around 23–27 ms with similar B wall time. This is authoring/build work, outside world ticks. Neither fixture measures representative tails or memory peaks.

An improvement must retain one cooperating process owner, one final root/outcome transaction, exact A reads while B stages, and a recovery-required barrier after an uncertain final COMMIT. The append-only first profile keeps old blobs; no reader may observe an uncommitted B as selected. A read pin remains durable through process interruption. A reader may not hold a SQLite transaction across consumer output, because that can strand WAL checkpoint debt. Space planning must count WAL/main overlap and retained old closure. The design adds no operation to simulation ticks.

## Options

| Shape | Benefit | Cost and failure surface |
| --- | --- | --- |
| Keep one mutex through admission | Smallest recovery state and one connection; historical baseline | Reader wait grows with staged bytes, hash work and SQLite sync; the local 12 MiB case approached 90 ms. |
| One admission permit, release the core mutex between completed file or chunk transactions | Existing SQLite connection, owner lock and final quarantine remain single; old pinned reads can interleave at bounded phase boundaries | Requires explicit admission state and phase ownership, careful tiny-file batch and retry handling, and tests that pin cleanup/checkpoint cannot invalidate in-progress admission. More mutex handoffs are paid per staged chunk, not per tick. |
| Separate owner read-only SQLite connection | Writer admission changes less and old leases can read in parallel with staging | Adds one connection/cache and WAL reader state; a shared recovery latch and exact close/quarantine ordering become more complex. Read-only open and close must be proved not to checkpoint uncertain WAL. |

The implemented first **admission-permit/short-core-phase** path holds one writer permit from preflight through final commit, while the core mutex protects each SQLite phase. It releases the core after committed blob batch/chunk operations and around staged-byte reading/hashing; it never releases across an open SQLite transaction or uncertain final COMMIT. Pin registration/release take the admission permit, and the writer retains its final generation CAS. `PinnedSource` remains a read lease on the same owner, not a second database authority. Complete-blob verification and manifest staging can still hold the core for longer phases; the separate read-only connection remains an option if these tails prove unacceptable.

The cost account is one extra mutex handoff per committed stage phase, a small admission-permit owner, and one reusable scratch vector with at most 4 MiB active length per chunk; retained capacity depends on allocator growth. The expected upside is giving an existing pin opportunities to read between writer phases. Mutex reacquisition can still starve a reader across many phases, so phase length alone cannot establish a wait bound. Admission wall time, SQLite WAL growth, allocation count, retained scratch, RSS, reader wait distribution and final-commit fault behavior all need paired evidence.

## Verification before promotion

1. Keep A pinned; stage B with at least three 4 MiB chunks while an A read starts after B's preflight. Check A's exact bytes and measure the reader wait distribution, repeated writer reacquisition/starvation and B admission time against the current owner in repeated optimized runs. Include tiny-file batches and an old pin's close. No timing threshold is declared from one local host.
2. Inject stale generation, source drift, `SQLITE_FULL`, reader-blocked checkpoint and pre/post-sync final-COMMIT faults at each new phase boundary. Precommit refusal leaves A selected; an uncertain final commit quarantines every later owner SQL and resolves the exact B key only on fresh open.
3. Kill a process after a completed B blob phase and after public B commit. Reopen distinguishes no B outcome from committed B; same-key retry is exact and A's retained source remains readable.
4. Report stage/validation/admission/cook timing, allocator requests and peak RSS with retained A plus B at the selected larger profile. Compare selected-root identity, source bytes and WAL/main high-water, not only mean throughput. Qualify the initial macOS profile first; other platforms require their own filesystem and lock measurements.

Do not change the public accepted-source/root/lease identities to implement this optimization. The current single-mutex path remains valid until the prototype passes its recovery and performance gates.
