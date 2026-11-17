# SIM-2D M2 review

Reviewed 17 November 2026 at `511927183f158ef5c3fa16ca668c0c0afbe1f6fc`, with builder endpoint `6c9d5f2cea36d09b90c0d3a583fe3b89d43720e6`.

**PASS: all twelve items pass.** No Must Fix findings, Suggestions or Could Not Judge items.

## Scope and proof basis

I formed my judgements from the criteria, design, plan and implementation before reading the builder's evidence and earlier reviews. I read M2's index and predicates, oracle/domain/predicate/allocation tests, crowd rules and swept-hit test, trace and native/Wasm runners, cost measurement code, conformance harness, browser changes and harness regression tests. I followed steering into the engine's `Executor::for_each_mut` implementation. Path history from accepted M1 (`2aad52d4e`) identifies the implementation beginning at `7d2c025fc` and subsequent fixes. `git show 6c9d5f2cea36` is the latest evidence refresh; it does not replace review of the milestone's earlier implementation. I inspected that refresh and its measurement rows, source manifests, receipts and explanation.

The script's checks on `6c9d5f2cea36` are the proof of record. I read the saved receipts and output tails for all eleven item commands and `./dev check` in `.git/orchestrate/checks/3dc50d5777200d70ffadb418-*`. Every exit is zero. The exact-images receipt accepts the shared main-branch run on `f080d0242ab3`, which contains the builder endpoint. I did not repeat these passing builds, tests or device campaigns.

My additional read-only validation recomputed the runner's dependency/source identity at this checkout, compared both cost manifests, checked Crowd against accepted M1 bytes, and compared all 36 historical target rows' folds and checkpoints against their retained references. [Validation evidence](M2-review/validation.json) records these results and the script's command receipts. Historical target records retain their actual older provenance; the script's fresh successful runs establish current correctness.

## Inspection list and target limits

M2 has no `inspect` proofs and no human-feedback items. Its visual inspection list is empty: these are headless CPU query and conformance programs, with no drawing. Low sun, storm, camera distance/speed, display refresh, simulator rotation and application backgrounding do not apply to these APIs. The applicable edges are empty/single/dense indexes, coincident centres, zero and oversize radii, touching/just-apart shapes, equal distances, invalid inputs, domain limits and very wide queries. Their coverage is judged below from the tests and supplied successful executions.

No screenshots, new target runs or new timing campaign were needed. This review claims no physical iPhone, x86, Android, other-browser, visual or display-cadence qualification. Simulator proof is function only.

## Item verdicts

| Item | Verdict | Judgement |
| --- | --- | --- |
| M2.1 | pass | `crates/thrum-sim2d/tests/oracle.rs` enumerates every input body independently of the index cells/tables. It deliberately shares the shape predicate, then independently sorts oversize bodies by slot and ordinary bodies by row/column/slot; nearest results sort squared centre distance then slot. Generated scenes include empty, one body, 10,000 bodies packed three deep, coincident centres, zero through several-cell radii, oversize bodies, 16,384-unit positions, fractional/tiny coordinates and wide shapes. Far-away padding changes table capacity without changing the queried bodies. The script's oracle test passes. |
| M2.2 | pass | `tests/predicates.rs` writes literal true/false answers for touching, just-apart, zero-radius, point/line boxes and zero-length capsules. Nearest ties, cutoff and k=0 have explicit expected lists. Expected answers are not computed by the tested predicates. Tiny-scale tests additionally prevent distance underflow from inventing overlaps. The script's predicate test passes. |
| M2.3 | pass | `examples/crowd-2d/tests/swept_hit.rs` sweeps from -3 to +3 across an enemy diameter [-1,1], expects slot 42 from the capsule, and expects no endpoint hit. It also runs 240 real Mixed ticks and requires positive swept-only hits. The live witness tests endpoint miss against the same enemy and displacement greater than its diameter. The script's swept-hit test passes. |
| M2.4 | pass | Index rows, cell table, sorted indices, oversize list and radix-order scratch retain their storage through clear/resize and reuse. The test-only System-forwarding counting allocator brackets warmed build, circle, box, capsule, wide-circle and nearest-k calls at 10,000 bodies. All six observations are recorded as zero. First-use growth is explicitly excluded; neither test nor validator imposes a zero-allocation threshold. The current cost manifest includes the unchanged allocator test. |
| M2.5 | pass | Crowd is byte-identical to accepted M1, SHA-256 `3878dbd394cb5b2e3e8538496032fd1cab1d5d0242e8debc7d8cfc595af573a0`. Mixed is retained independently from a straight corrected run. The command checks both traces at all three sizes, both small wire routes and both small native debug traces. Mixed records positive oversize counts 4,633/17,449/354,873, capsule hits 599/599/5,999 and swept-only hits 438/435/3,974. The script's current Mac run passes; stored checkpoints/folds also match. |
| M2.6 | pass | The harness compiles Wasm and executes the exported functions in real Chrome. Both traces at all sizes and both wire routes compare against the same retained references, including every checkpoint and the fold of the corrected timeline. Browser identity is recorded; Node controls Chrome rather than substituting for Wasm execution. The script's Chrome run passes. |
| M2.7 | pass | The runner builds `aarch64-apple-ios-sim` and executes through `simctl spawn`. Both traces at all sizes and both wire routes compare to the same reference values. The script's simulator run passes, and historical rows independently match. No simulator speed claim is made. |
| M2.8 | pass | Steering uses independent mutable 256-row chunks and a shared read-only index through `Executor::for_each_mut`; the engine uses Rayon parallel iteration in native forced mode. Results are applied in canonical row order. Retained trace rows report 28,800/72,000/1,440,000 chunks and unchanged reference digests. The script's current forced-parallel run passes. |
| M2.9 | pass | Two complete release runs contain 12 rows each: Crowd/Mixed, all three sizes, serial/forced-parallel. Each retains tick/index/per-neighbour costs, load brackets, build and timestamp, plus digest and eight-tick rewind/replay costs. Source identity matches this checkout. Four serial Crowd rows trip the M1 quarter-regression flag at identical neighbour counts; the investigation accounts for added index ordering, bounds/predicate and steering work without unsupported causal percentages. Every forced-parallel result is retained as measured. The script's cost validator passes. |
| M2.10 | pass | Two complete Chrome release runs contain six rows each, with both traces, all sizes, load/build provenance and per-neighbour costs. Their source identity matches this checkout. All six Crowd rows trip the M1 regression flag at identical neighbour counts; the investigation explains the changed work and separates historical profiling from fresh measurements. Chrome queue/load-sampling limitations are disclosed. The script's cost validator passes. |
| M2.11 | pass | Main-branch check exact-images, accepted from the script's shared-run receipt under ORCHESTRATE.md. M2 moves no retained image. |
| M2.12 | pass | `tests/domain.rs` uses the minimum accepted 1/16 cell size, edge bodies and wide circle/box/capsule queries, comparing with direct enumeration in canonical order. Out-of-domain bodies, query coordinates/radii and undersized cells return typed errors; rejected builds preserve prior state. The wide box spans over 536 million cells per axis. Walking that range cannot finish in the test's observed time; the implementation instead scans preordered bodies when covered cell area exceeds body count. The script's domain test passes. |

## Earlier findings

After the independent item assessment, I read both earlier reviews and the finding names in `RECORD.json`.

- **`mac_cost_identity_stale` — resolved.** The current Mac measurements were made at clean commit `768636def207df168be1bfb0feb90c25c12f860a`. Their source key is `0f6f30f9d3b2c2b2199641758aabbdead4065825cebce6a9818ff3b71217d8de`, exactly the current checkout's key. Every normalized source-manifest field matches, including the core clock additions absent from the previous records. The script's cost check exits zero.
- **`chrome_cost_identity_stale` — resolved.** The refreshed Chrome record has the same measured commit and matching source key, its own Wasm artifact identity, and two full target-specific runs. Its cost check also exits zero. The validation was not weakened and historical measurements were not relabeled.

Python tuples versus JSON arrays make a direct in-memory graph comparison appear unequal; JSON normalization proves the graphs equal, as do their canonical hashes. There is no remaining file, settings or dependency-graph mismatch.

## Quality and cost assessment

The implementation meets the query quality bar: deterministic visitation order, independent enumeration coverage, explicit geometry oracles, bounded wide queries, validation before index replacement and read-only parallel access. No canned answers, weakened correctness tests, game schema in the engine, or replacement simulation path was found.

Costs remain visible at dense scale: approximately 469,268 Crowd and 474,752 Mixed neighbour visits per tick at 10,000 bodies. Mac serial ticks span 9.754–10.054 ms, forced-parallel 1.981–2.254 ms, and Chrome 13.905–14.765 ms. The M1 comparisons openly show 33.6–39.8% Mac regressions at 2,000/5,000 and 62.1–81.6% Chrome regressions across the sizes. Added eight-pass canonical ordering raises measured index cost; historical native and Chrome samples locate shared predicate, bounds and steering work. The explanation correctly stops short of assigning each change a causal percentage.

The four Mac and three Chrome scaling flags are retained and explained at the 5,000-body table-capacity step. No row trips the clock or chronological quarter-regression flag. Eight-tick replay remains a material planning cost: 79.333–82.300 ms serial and 15.280–38.792 ms parallel on Mac, 114.260–120.025 ms in Chrome. These are shared-host observations, not quiet-machine qualifications or time budgets. The written criteria require complete, applicable records and explanations; those requirements are now met.

PASS M2
