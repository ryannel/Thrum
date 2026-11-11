# BASELINE-GUARD design review

Design approved with one Suggestion and no Must Fix findings. Reviewed integrated revision
`4a1e0aeaf5a5d8ae028b4f803b4f9e4d5c3faf94` on 11 November 2026 against the previous review at
`9619c1370075`. No other agents were launched. This approves the design, not the implementation or current
baseline equality.

The request is cross-module exact-baseline protection before landing, including the jitter regression and
stale evidence after a rebase. I read both tasks in [ACCEPTED.json](../../plans/BASELINE-GUARD/ACCEPTED.json),
oldest first and in full, and the project brief before judging the revision. The second task asks the planner
to address every prior finding or explain why not; it does not relax the first task's protection requirement.

## Previous findings

**MF1 — resolved in the design: the full comparison can still happen after the failing revision lands.**

[DESIGN.md](../../plans/BASELINE-GUARD/DESIGN.md), lines 81–100 and 126–146, replaces the selected Mac checks
with a full verifier and a proof keyed to covered content. The verifier includes the native matrices and all
four Chrome sets. The gate in `./dev check` refuses a covered-path change without that proof. M1.3 exercises
rebasing and proof invalidation; M1.4 adds the actual loop's `land`, night-only and Chrome-only regressions,
and an unchanged throwaway remote after refusal. BASELINE-GUARD owns all these parts in M1.

I reread the installed orchestrate 1.7.1 `build/scripts/agent_loop.py`, `land()` at lines 2602–2632 and its
command handler: the ordinary `land` command enables the profile check after fetch/rebase and before push,
and repeats that sequence after a rejected push. An untracked worktree proof survives the rebase, but its key
ceases to match when covered content changes. The gate can therefore stop the push without needing to run
Chrome inside `land` or return control midway through integration. A documentation-only rebase can retain a
proof because the tested inputs are unchanged.

The architecture now makes this gate apply to every ordinary checked landing once installed, without waiting
for each existing plan to gain a verifier item. The initial CULLING/EFFECT-GRAPH work before installation remains
explicitly described as a transitional exposure, not as protected work; the module list schedules BASELINE-GUARD
first and later milestones depend on it. PLAN.md explicitly limits enforcement to checked integration; bypassed
checks and forged proofs remain outside the mechanism. That is a stated workflow assumption, not the former
hidden gap between a successful checked integration and a full comparison.

**MF2 — resolved in the design: the full clean-checkout verifier lacks its Chrome reference inputs and capture path.**

[DESIGN.md](../../plans/BASELINE-GUARD/DESIGN.md), lines 89–124, specifies the fresh Wasm build, fixed-capture
route and four exact reference sets. The [architecture ruling](../../plans/ARCHITECTURE.md#exact-baselines)
authorizes a reference-by-digest mode while preserving the existing byte mode and retained files. The new mode
recomputes each candidate digest from decoded frame bytes; it does not trust the candidate manifest's hash.

I independently decompressed all four committed manifests. Each is complete and fixed, contains 481 records,
and supplies a valid SHA-256 and every identity field the existing comparator requires for each record. Both
linear sets supply eight exposure words per record. The [digest inventory](design-review/digest-inventory.json)
records the inputs and results. Those retained digests supply the reference information needed by the revised
comparison without the missing bulk frames. This accepts the explicitly stated SHA-256 equality assumption;
it does not regenerate the reference or relax image tolerance.

The existing browser build and capture entry points support the proposed sources and scenes. M1.1 requires
the complete clean-checkout route, and M1.2 checks every record, fresh-byte hashing and reference preservation.
Whether the four sets still match current main is honestly left to the first build experiment, with a stop
and owner-controlled refresh if they do not. That empirical uncertainty does not leave the mechanism undesigned.

**S1 — resolved by removal.** The debug-build image guard and its alternate reference set are no longer in
scope. There is no conflicting fallback for M1.1 to accept.

**S2 — resolved.** M1.4 and PLAN.md now retain mutant patches, base revision, exact commands and logs, and
require failures to distinguish changed output from unreadable references.

## Remaining suggestion

**S3 — Suggestion, verified omission; future effect not demonstrated: include repository build configuration in the content key.**

[DESIGN.md](../../plans/BASELINE-GUARD/DESIGN.md), lines 77–84, lists the covered inputs but omits
[.cargo/config.toml](../../../.cargo/config.toml). It currently contains only the `xtask` alias, so I found no
present renderer input missing there. However, this is also where repository compiler flags or build environment
settings can be added. Including `.cargo/` now, or explicitly requiring coverage to expand before such settings
are introduced, would prevent a future configuration change from retaining an old proof. This is a preventive
refinement, not a demonstrated current baseline failure and not a reason to hold the build.

## Changed acceptance items

I compared DONE.json with `9619c1370075` and read every revised item, not only the new integration proof.
M1 still has no prerequisite milestone. Its new bar requires proof for the integrated content and exact named
comparisons. Existing native scripts, retained references and browser capture infrastructure make that
independence credible; no unbuilt consumer is needed to judge these items.

| Item | Assessment of the planned proof |
| --- | --- |
| M1.1, rewritten | Judgeable. The automated verifier plus source inspection covers fresh native/Wasm builds, cooking, the complete registered matrix, dirty-tree refusal, input provenance and proof creation. It replaces the former small native guard rather than preserving its narrower claim. |
| M1.2, rewritten | Judgeable with M1.1. Inspection of the comparator and verifier record can establish all four sets, all 481 records, hashing actual decoded candidate bytes, retained-file preservation and unchanged byte mode. Manifest-only hash copying would fail its explicit question. |
| M1.3, rewritten | Judgeable. Its automated throwaway-remote test and gate inspection cover proof-free unchanged content, refusal without a matching proof, invalidation after covered changes and preservation after unrelated changes. Checking the remote after refusal establishes the pre-push effect. |
| M1.4, rewritten | Judgeable in the same milestone. Actual `land` runs and reproducible mutants cover jitter, night and Chrome, plus the stale-proof case. A passing local verifier with no enforcement cannot satisfy this item. The scratch remote contains the demonstration safely. |
| M1.5, added | Judgeable. Three verifier runs and three gate checks with load establish cost and proximity to the loop timeout. This is a cost record, not a numeric performance gate. |

A selected-frame verifier cannot satisfy M1.1/M1.2; a stale manifest cannot satisfy fresh-byte inspection;
a gate disconnected from `./dev check` cannot satisfy M1.3/M1.4. Together the items reject the convincing
incomplete implementation identified in the previous review. No additional blocking finding is warranted.

## Scope and validation

I read the complete current DESIGN, PLAN, DONE and LOG, the previous review, relevant module-list rules,
C12, contributor guidance, engineering review guidance and both graded-example sources. There is no plans
GLOSSARY.md. I inspected the native matrix scripts and their reference/capture helpers, Chrome comparator,
browser build/capture entry points, contributor check dispatch and installed integration control flow.
The historical reference inventory remains unchanged beside the new digest inventory.

This was a design and interface review. No GPU output, browser captures or motion were reviewed, and the
not-yet-implemented verifier and gate were not run. Only this review and its evidence folder were changed;
DONE.json was not edited. `./dev check-docs` passed: 697 Markdown files, 3,633 local links, 30 module
records, crate policy and current design hashes, with existing file-size notes only. `git diff --check`
also passed.

DESIGN OK
