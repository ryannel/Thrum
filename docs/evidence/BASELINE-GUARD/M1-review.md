# BASELINE-GUARD M1 review

PASS. All five items pass; no Must Fix findings or Suggestions.

Reviewed commit `6945ae1758575899cdd0c1e573dc44e4e1b059d9` on this Mac, 12 November 2026. No other agents were launched. The implementation and acceptance criteria were inspected before the builder's evidence and log. This is the module's only milestone; the review covered the complete delivered module, including the registry, verifier, native retention adapter, gate, tests, comparator addition, xtask integration, documentation and retained reproduction scripts. Unrelated engine changes brought in by other modules are not attributed to this milestone.

## Independent execution

Both device commands ran from the requested checkout, under the repository's `agent_loop.py lock gpu --` wrapper. The tree remained clean throughout verification. Substantial waiting for other GPU jobs preceded execution; that queue time is not included in the verifier's duration.

- `python3 scripts/exact_baselines.py verify`: exit 0, **932.85 seconds**, load averages **3.70 / 3.94 / 3.84** at start. Release native and Wasm builds, fresh cook, both native matrices, and all four Chrome captures ran. All **38 + 4 native comparisons** and **4 × 481 Chrome records** passed. [Independent record](M1-review/verifier/record.json), [proof](M1-review/proof.json), [native M1 manifest](M1-review/verifier/apple_tile_m1-manifest.json.gz), [native M2 manifest](M1-review/verifier/apple_tile_m2-manifest.json.gz).
- `python3 scripts/test_exact_baselines.py`: exit 0, all nine tests passed. [Test log](M1-review/focused-tests.log).
- `./dev check`: exit 0. [Full check log](M1-review/profile-check.log).

The independent proof names content key `3d0e464e78f6c6260a94671d971c1ff1cf3fc73eee2272385719ba4aaad94046`, the reviewed revision, native binary SHA-256, generated Wasm/page hashes and cooked root. Its native binary SHA-256 is `be33c9c3ef2a111e41b4fad7adc64adbb9859690e64485cb9685760b79daf9e9`; its cooked root is `70d2baf823d09d85574f7cdfce7102648754544dd7d68659e9bd92dd5f30e21d`.

## Item verdicts

| Item | Verdict | Judgment |
| --- | --- | --- |
| M1.1 | pass | The required command completed on the reviewed commit and published a proof only after every comparison passed. Native inputs come from this checkout and its release binary/fresh store; Chrome uses this checkout's freshly built page and cooker. No dependency on another task's scratch was found. The registry and architecture table agree on the native baseline, four Chrome baselines, APPLE-TILE ownership and covered paths. Dirty trees are rejected before work begins; the focused tests exercise this refusal. |
| M1.2 | pass | Each Chrome candidate frame is opened, decompressed and hashed again. The digest must match both the candidate's declared hash and the retained hash; identities are checked record by record. Each of the four independent comparison reports contains all 481 records. All 55 registered retained files are byte-identical to the pre-implementation revision. The existing byte comparator's AST and all three owner matrix scripts are unchanged. |
| M1.3 | pass | The tests drive actual Git fetch, rebase and push against disposable local bare remotes. They verify unchanged remote main on refusal, covered rebases invalidating proof, uncovered rebases preserving proof, no-proof documentation changes, and dirty/missing-base rejection. The gate is unconditional at the beginning of xtask's `check` branch. Reading the loop's `land` confirms it fetches/rebases, checks, then pushes, and repeats the check after a rejected push and another rebase. No gate skip flag or environment variable was introduced. |
| M1.4 | pass | Retained patches, named failures, actual loop-land logs, and before/after remote revisions support all four refusals. The jitter mutant changes 472,377 pixels; the night mutant changes 2,073,600 pixels. The Chrome-only mutant passes both native matrices before failing Chrome day SDR at ordinal 0. The stale-proof case changes the covered key after rebase and is refused. All destinations are local scratch bare repositories. I independently reconstructed the retained source patch from its published anchor and obtained exactly the recorded experiment key. |
| M1.5 | pass | Three complete verifier samples record 900.27, 896.99 and 884.58 seconds with load and revision; three gate samples record 0.1583, 0.1225 and 0.1623 seconds including Python startup. A covered landing pays roughly 15 minutes of verification, plus GPU queueing and the repository check, and pays again after a covered-content rebase. Those historical verifier samples use 24.6–25.0% of the one-hour check allowance; the repaired run uses 28.4%, and this independent run uses 25.9%. These are measured samples, not a worst-case guarantee. |

Chrome proof: [day SDR](M1-review/verifier/chrome-day-sdr-comparison.json.gz), [day scene-linear](M1-review/verifier/chrome-day-scene-linear-comparison.json.gz), [night SDR](M1-review/verifier/chrome-night-sdr-comparison.json.gz), [night scene-linear](M1-review/verifier/chrome-night-scene-linear-comparison.json.gz). [Independent source/reference integrity and refusal-base reconstruction](M1-review/integrity.json).

M1.4's retained evidence was audited rather than rerunning its expensive mutants: [actual refusal report](M1/refusals-final/report.json), [reproducer](M1/reproduce_refusals.py), [base recovery](M1/refusals-final/base-source.json), [jitter land log](M1/refusals-final/jitter-land.log), [night land log](M1/refusals-final/night-only-land.log), [Chrome land log](M1/refusals-final/chrome-only-land.log), [stale-proof land log](M1/refusals-final/stale-proof-land.log). The question proof asks whether these permit reproduction; the source reconstruction, patches, code paths and logs do. Missing/corrupt retained-reference errors are distinct from decoded-image differences in the comparator and focused tests.

Timing sources: [three verifier and gate samples](M1/timing.json), [independent audit of retained run records](M1-review/retained-records-audit.json), [repaired builder run](M1/disk-repair/verified/record.json). The historical timings span explicitly identified source revisions; the fresh run above qualifies the actual review commit.

## Edges, scope and quality bar

There are no `inspect` proofs in this milestone, and the design expressly excludes visual-quality judgment. Accordingly, there is no visual inspection list or claim to have watched motion or qualified physical display cadence. The product exercised here is the verification/gating tool; it built and ran the native game and Chrome renderer through the owner's matrices. Its complete native manifests and Chrome records are retained as linked evidence rather than embedded images.

The fresh matrices cover both native recipes, moving-route checkpoints, low-sun and night stills, SDR and scene-linear output, two letterbox surface formats, realtime variants/routes and render-stall recovery; Chrome covers day/night and both sources in fixed mode. Focused negative checks cover last-record pixel changes, altered identities, corrupt/missing references, dirty inputs, missing remote base, stale proofs and synchronous cleanup before the next capture. Physical full-refresh cadence, simulator lifecycle and artistic storm quality are not acceptance claims of this gate.

The cleanup adapter calls the owner's original publisher and only then archives completed successful runs. Reading both owner matrices confirms they finish each run's comparisons before publishing and do not subsequently consume its pixels. Failed captures are preserved. No tolerance, retry, reference refresh or test weakening was found.

The result meets the stated merge-gate bar: the tested covered content is what the integration gate accepts, exact comparisons include native and Chrome output, and real regressions are refused before the local integration remote moves. The design's explicit limits—local proof forgery and deliberately bypassing checked integration—remain unchanged.

PASS M1
