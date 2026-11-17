# SIM-2D M2 review

Reviewed 17 November 2026 at `8176afb5083a87eaa032d98bf9c5d24ab4588373`.
**FAIL: ten items pass; M2.9 and M2.10 fail.** No Could Not Judge or outside-item Must Fix.

## Must Fix findings

1. **M2.9 — `mac_cost_identity_stale` — verified, repeated.** The Mac cost proof does not pass for this integrated build. `python3 scripts/sim2d_conformance.py costs --target mac-metal` exits 1 at `scripts/sim2d_conformance.py:390`, reporting different simulation sources. The refreshed measurement is from `119bd20ab17438e0bab5a5c5bfb52b695e835b74`, source key `a8df10ea68a72e0b109e9f934951d7cd1dcf2809395fbba59c195c0b44d8b8c3`; this checkout computes `0f6f30f9d3b2c2b2199641758aabbdead4065825cebce6a9818ff3b71217d8de`. Five core files differ: `src/clock.rs`, `src/sim/clocked_world.rs`, `src/sim/world_lane.rs`, `tests/clock_spans.rs`, and `tests/clock_spans/lane.rs`. The design requires a complete record applicable to the claimed build. Make the retained evidence and its applicability check agree for the integrated build, preserving actual measurement provenance. I inspected both repeats and their explanation, compared input manifests and reran the validator. I did not repeat the measurement campaign or establish a speed change from these later clock additions. This is a failed required proof, not a timing-budget failure.
2. **M2.10 — `chrome_cost_identity_stale` — verified, repeated.** The Chrome cost proof fails for the same integrated-source mismatch: `python3 scripts/sim2d_conformance.py costs --target chrome-webgpu` exits 1 at line 390. Its refreshed record has the same measured commit and source key and differs in the same five files. Establish the Chrome evidence's applicability to the reviewed build as well; the Mac record cannot substitute for it. I read both Chrome repeats, their comparison with M1 and the regression explanation, and reran the validator. I did not launch a new browser measurement campaign.

[Validation evidence](M2-review-2/validation.json) retains commands, exit codes, full errors, source identities and changed paths. The error lists `graph` as well as `files`, but normalizing JSON lists versus Python tuples makes the graphs equal; settings also agree. The source-file mismatch is real. No dependency-feature change is claimed.

## Independent scope and previous review

I formed the item judgements from the design, plan, criteria and implementation before reading the previous review. I read the index and shape implementations, oracle, predicate, domain and allocation tests, the swept-hit test, crowd rules and trace changes, runner/cost code, browser harness and conformance-tool changes and tests. I checked the engine executor on the other side of the steering interface. I then read the builder evidence and log, and finally the prior review and recorded finding names.

`git show 1f1e22310066` is an APPLE-TILE documentation checkpoint, not the SIM-2D implementation patch. Path history locates M2's implementation at `7d2c025fc` and subsequent query, harness and evidence commits. I reviewed those changes in the supplied checkout rather than attributing other modules' work to this builder.

The prior Must Fix findings `mac_cost_identity_stale` and `chrome_cost_identity_stale` are both back. The builder did refresh both complete records, but later core clock additions again invalidate their required source check. The names remain unchanged; these are not different defects. The five changed files also appear in the diff from the script's checked `de2bc14ac5b1` to this review tip.

The script's successful `./dev check` and correctness item checks on `de2bc14ac5b1` are accepted. Query, crowd, runtime stepping and conformance code are unchanged since that check. The later production changes add clock due-time accessors, not a replacement crowd tick or executor. Only the two lightweight source-bound cost validators had a concrete reason to be rerun, and both failed. I independently compared every retained target row's fold and checkpoints with its trace reference; all matched. The Crowd file is byte-identical to accepted M1 commit `2aad52d4e`.

## Inspection list and target scope

There are no `inspect` proofs or human-feedback items in M2. The visual inspection list is empty: this module has no drawing, and its target programs are CPU command-line conformance runners. Low sun, storm, camera distance/speed, display refresh, rotation and app backgrounding do not apply to these query APIs. The applicable numerical edges are empty/single/dense scenes, touching and degenerate shapes, equal distances, oversize circles, domain boundaries, invalid inputs and very wide queries, covered below. No new screenshots or device campaign was needed. No physical iPhone, x86, Android, other-browser, visual or display-cadence claim is made.

## Item verdicts

| Item | Verdict | Judgement |
| --- | --- | --- |
| M2.1 | pass | The oracle enumerates every input body without using index tables, shares the intended geometry predicate, and independently sorts by oversize/row/column/slot or squared-distance/slot. Generated cases include empty, one, 10,000 three-deep bodies, coincident centres, radii zero through several cells, oversize bodies, 16,384-unit positions, fractional/tiny coordinates and wide queries. Padding changes table capacity. Script's oracle check passed. |
| M2.2 | pass | Predicate tests give literal true/false geometry expectations for touching, just-apart, zero-radius, degenerate box and zero-length capsule cases; nearest tests write out ties and cutoff answers. Expectations are not computed by the tested predicates. Tiny-scale cases also guard against underflow producing false intersections. Script's check passed. |
| M2.3 | pass | The fixture sweeps -3 to +3 through an enemy of radius 1, expects the capsule to visit slot 42 and the endpoint to visit none. A real 240-tick Mixed run must also report swept-only hits. Its witness checks endpoint miss against the actual hit enemy and displacement exceeding its diameter. Script's check passed. |
| M2.4 | pass | Index buffers retain capacity through clear/resize, including ordered rows and radix scratch. The System-forwarding allocator measures warmed calls at 10,000 bodies; build, circle, box, capsule, wide-circle and nearest counts are all recorded as zero. The code checks observation completeness, not an allocation budget. First-use growth is explicitly excluded. The allocation/index code is unchanged; broader cost identity is judged under M2.9. |
| M2.5 | pass | Crowd retained bytes are unchanged from accepted M1. Both traces compare checkpoint lists and corrected-timeline folds to straight-run references. Mixed records positive oversize counts (4,633/17,449/354,873) and capsule hits (599/599/5,999), plus swept-only witnesses. Mac covers all sizes, wire routes and both small debug traces. Script's conformance check passed. |
| M2.6 | pass | The browser harness builds Wasm and invokes exported functions in Chrome, not a Node simulation. Both traces at all sizes and both wire routes match the Mac references; HeadlessChrome 157 is recorded. Script's Chrome check passed. |
| M2.7 | pass | The runner builds the iOS simulator target and executes it with `simctl spawn`; both traces and wire routes match the retained references. This is simulator function only. Script's simulator check passed. |
| M2.8 | pass | Steering dispatches private 256-row chunks through `Executor::for_each_mut`; native forced mode uses Rayon, with a shared read-only index and canonical result application. Records show positive 28,800/72,000/1,440,000 chunk totals and equal digests. Script's forced-parallel check passed. |
| M2.9 | fail | Two complete Mac release runs contain 12 rows each: both traces, three sizes and both execution modes, with load and per-neighbour costs. Explanations agree with their rows, but the required cost validator fails for the integrated build. See `mac_cost_identity_stale`. |
| M2.10 | fail | Two complete Chrome release runs contain six rows each with load and per-neighbour costs. Explanations agree with their rows, but the required source validator fails. See `chrome_cost_identity_stale`. |
| M2.11 | pass | Main-branch check exact-images, accepted from the supplied script result under ORCHESTRATE.md. No retained image is moved by M2; no duplicate GPU verification was run. |
| M2.12 | pass | At cell size 1/16 the domain test builds edge bodies, compares wide-query sequences with direct filtering, and checks typed refusal of out-of-domain bodies/queries and undersized cells. Invalid builds preserve prior state. The wide box spans over 536 million cells per axis; the implementation scans preordered bodies when area exceeds body count, so it cannot walk that empty-cell range. Script's check passed. |

## Quality and cost assessment

Query order is independent of hash-table capacity and scheduling. The implementation keeps one derived index, validates before replacing it and offers read-only queries to parallel rules. The tests exercise the design's relevant range and the game uses the real query path. I found no additional Must Fix or Suggestion.

The refreshed records openly retain regressions: Mac serial Crowd at 2,000/5,000 is 32.1–42.8% slower than M1, and Chrome Crowd is 59.0–74.6% slower at identical neighbour counts. The investigation relates measured index increases to eight stable radix passes and identifies the changed predicate/bounds and steering paths in historical CPU samples. It explicitly does not assign causal percentages or pretend those old samples profile the refreshed executable. That is an adequately bounded explanation, not a claim of isolated attribution.

Dense 10,000-body rows retain roughly 469,268 Crowd and 474,752 Mixed neighbour visits per tick. Current Mac serial medians span 9.640–9.878 ms, forced-parallel 1.952–2.107 ms; Chrome spans 13.855–14.190 ms. Both repeats and all modes are retained. Host-load brackets and Chrome's queue-sampling limitation are disclosed. These figures are shared-host baselines, not speed budgets or quiet-machine qualifications. Their source-applicability failures, rather than their values, prevent completion.

FAIL M2: 2 fail, 0 could_not_judge, 0 outside the items
