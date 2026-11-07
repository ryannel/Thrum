# Fixed-slot rule-created output credits

> Status after the code-quality cleanup: the I1 timing examples cited here were removed from the active tree. Their historical measurements and source remain in Git; quoted example commands are no longer current commands.

1 November 2026. This is a selected protected fixed-slot host profile, not the general process-memory gate. Its rules may create immutable payloads during `decide` and `prepare` for rejection, state and event values. Local and multiplayer authorities call the same `GameRules` methods with borrowed `RuleContext` views of one per-action budget; prototype/ordinary callbacks receive an unqualified context. A protected host declares one cumulative payload allowance per accepted action. The game contract requires every newly created retained payload to use `RuleContext::try_copy_payload`; pre-existing charged action or world owners may be shared. Direct allocation from a captured pool during a protected callback breaks the admitted-input progress promise even though the pool still enforces its global cap.

Before consuming an input sequence, the authority atomically reserves the checked sum of that input's conservative snapshot/group credit and output allowance in one pool operation. Each pending input retains its own reservation. A due tick validates **all** due reservations against their expected combined size before transferring only their snapshot/group portion to the tick reservation. Pool usage never falls between those owners. The residual reservation belongs to that one action's engine-owned `RuleBudget`; the callback receives only a borrowed view. All payload owners, including zero-length ones, charge `PublicationBytes::required_charge(len)` against a cumulative, non-refunding allowance across both callbacks. A dropped temporary owner cannot reset this per-action work limit. A sticky exceeded flag is checked after each callback and faults the authority before that action's edits. Swapping callback views cannot replace the budget or erase the fault. Output values still pass the sealed shape, size and pool-provenance checks before world mutation.

Tests cover two peers in one fully occupied pool: one accepted edit shares a new payload between authoritative state and event, while another produces a charged rejection. A delayed first sequence retains its output credit while sequence 2 resolves under full competing use. A separate test spends one allowance in both decision and preparation. Another holds an old snapshot with a variable payload across replacement and host retirement, then checks release on final reader drop. Combined-credit refusal leaves the same sequence available for retry; zero-length owners count; over-budget preparation faults before an edit; and a callback-visible context swap cannot clear the sticky fault. `RUSTC_WRAPPER= ./dev check` passes.

The `complete output fixture` (historical `git show 84d7673:crates/thrum-core/examples/i1_output_credit_cost.rs`) runs 1,000 warmups and 10,000 measured one-change submit/callback/acknowledge cycles with a newly created 32-byte state payload. Every iteration asserts exactly one completed tick, a committed result and the expected replica payload. Callback timestamps use 16,666,667 ns increments to track the 60 Hz clock; an earlier 17 ms version was discarded because its observations became stale. Local macOS/aarch64 A/B/B/A runs using the final release binary SHA-256 `be2afd1151323ee0dddbd674ca053f342d300ba972f110e809ac41fed6285504` gave:

| Mode | A p50 / p95 ns | B p50 / p95 ns |
| --- | ---: | ---: |
| Ordinary charged callback | 708 / 791 | 708 / 875 |
| Protected output callback | 792 / 875 | 708 / 792 |

Raw runs: [ordinary A](output-credit-ordinary-a.txt), [protected A](output-credit-protected-a.txt), [protected B](output-credit-protected-b.txt), [ordinary B](output-credit-ordinary-b.txt). These modes have different admission behavior, so the pair is an integrated viability comparison, not an isolated output-credit cost.

The existing `no-payload complete fixture` (historical `git show 84d7673:crates/thrum-core/examples/i1_charged_replica_cost.rs`) was also run A/B/B/A against commit `6bf5dfe`, with separate Cargo target directories and distinct release binaries: before SHA-256 `93c2e8b7b1d22dbbae31d9a1e18cf7657049489f82d208d855eeedc9616501ba`, after `90731eeba9f128c4e3604c55e5877d7166989567afbf9dc0dba422d4c5baf87f`. Protected-mode p50 submit/callback/acknowledge nanoseconds were:

| Objects and work | Before A / B | After A / B |
| --- | ---: | ---: |
| 64, one change | 750 / 666 | 875 / 666 |
| 4,096, one change | 917 / 959 | 917 / 958 |
| 64, sixteen changes | 2,458 / 2,291 | 2,708 / 2,416 |
| 4,096, sixteen changes | 2,750 / 2,708 | 2,792 / 2,750 |

Raw runs: [before A](output-credit-no-payload-before-a.txt), [after A](output-credit-no-payload-after-a.txt), [after B](output-credit-no-payload-after-b.txt), [before B](output-credit-no-payload-before-b.txt). Medians vary across order and background conditions; these runs do not establish a whole-path speed improvement or a firm regression. The selected profile adds one stack context view per rule callback and uses one atomic input reservation for the combined future credits. The measurements do not include allocator counters, sustained named-device qualification, game-private scratch, structural growth or remote transport.
