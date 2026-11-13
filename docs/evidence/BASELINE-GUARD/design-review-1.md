# BASELINE-GUARD design review

Design changes requested: two Must Fix findings. Reviewed integrated revision
`9619c1370075b6b03f9d436f12af454f6020197c` on 11 November 2026. This is the first design review;
there was no earlier review to resolve. No agents were launched.

**MF1 — Must Fix, verified: the full comparison can still happen after the failing revision lands.**

Locations: [DESIGN.md](../../plans/BASELINE-GUARD/DESIGN.md), lines 67–69 and 107–109;
[ARCHITECTURE.md](../../plans/ARCHITECTURE.md), lines 243–255 and 266–271;
[DONE.json](../../plans/BASELINE-GUARD/DONE.json), M1.1–M1.3.

The request is to catch a change to another module's exact baselines **before landing**. The design puts only
a subset of Mac comparisons in the land gate, and relies on rule 3 for the remaining comparisons. Rule 3 asks
the builder to rerun the full matrix if `land` rebases covered paths, but the installed orchestrate 1.7.1
implementation does not return control between that rebase and the push. I read `land()` in the build skill's
`scripts/agent_loop.py`, lines 2602–2628: under the integrate lock it fetches, rebases, runs only the profile
Check, then pushes. Here that Check is `./dev check`. A rejected push repeats that sequence.

An ordinary failing sequence is: a builder verifies its branch; another builder lands; `land` rebases the
first branch; the combined revision changes an unguarded night, surface-format or Chrome result; the selected
Mac checkpoints pass; `land` pushes. The full rerun requested by rule 3 can now detect the regression, but
main already contains it. A fix without a milestone also has no claim-time check. This is a control-flow
finding, not a claim that I ran a GPU mutation or pushed a test commit.

Chrome's exclusion from `./dev check` is explicitly listed as an assumption in PLAN.md; it is not hidden.
However, the proposed alternative does not establish the requested pre-landing protection. Its rollout is
also incomplete: CULLING M1.5 and EFFECT-GRAPH M1.5 run only the native scripts, and the architecture defers
adding the rule-3 item to several existing plans until their next revision.

Resolve this by specifying a pre-push mechanism that verifies the affected registered matrix on the final
rebased revision, including Chrome, or refuses integration until that revision has valid proof. Name the
implementation owner and its place in the plan. Add an acceptance item that exercises a regression outside
the small guard, including invalidation of proof when the integrated revision changes, and demonstrates
refusal before main advances. A small fast guard can remain useful; passing it cannot stand in for the full
pre-landing claim. If a narrower guarantee is intended, that changes the request and needs to be settled
explicitly rather than presented as its completion.

**MF2 — Must Fix, verified: the full clean-checkout verifier lacks its Chrome reference inputs and capture path.**

Locations: [DESIGN.md](../../plans/BASELINE-GUARD/DESIGN.md), lines 45–49, 72–81 and 115–117;
[DONE.json](../../plans/BASELINE-GUARD/DONE.json), M1.3;
[ARCHITECTURE.md](../../plans/ARCHITECTURE.md), line 260.

M1.3 promises every registered baseline in full from a clean checkout, and the registry table includes
Chrome. The proposed mechanism builds the native release game, cooks the valley and wraps the owner scripts.
The registered Chrome command, [apple_tile_m1_compare_browser.py](../../../scripts/apple_tile_m1_compare_browser.py),
does not build or capture anything. Lines 14–15 require two unpacked manifests; lines 29–30 read every frame
from both captures. The registered reference directory contains compressed manifests and five selected PNGs,
not the complete reference capture it requires.

I decompressed the committed reference manifests and checked their record paths. All 481 frame files in the
registered M0 reference are absent beside the manifest, starting with `frames/000000.rgba.gz`. The three
pre-change linear/night extensions documented by APPLE-TILE likewise lack all 481 referenced frames each.
The [inventory](design-review-1/reference-inventory.json) records the exact paths and counts.
The owner's [M0 report](../render/apple-tile-m0/README.md), lines 66–71, places the complete sources in
`tmp/g3t-m0`; its [M1 report](../render/apple-tile-m1/README.md), lines 51–55, says bulk sources and pre-change
Wasm stay in owned scratch without remote backup. Those inputs are not supplied by a clean checkout.

Wrapping the comparator therefore cannot deliver M1.3 as designed. The verifier also needs an explicit fresh
Wasm build and fixed-capture route, with artifact identity; hashing the native binary alone does not identify
Chrome's candidate. Existing `examples/v1-game/web/build.sh` and `scripts/capture_v1_browser.mjs` provide
implementation starting points, but the design does not connect them to the verifier.

Resolve the Chrome reference acquisition and preservation contract with APPLE-TILE, without silently
regenerating its baseline. Specify which retained day/night and SDR/linear sets the registry covers, how a
clean checkout obtains their required inputs, and how the current revision produces fresh candidates.
Then make M1.3 prove that whole route without depending on another builder's scratch. If the references
must first be made available by their owner, record that dependency instead of calling M1 independent.

**S1 — Suggestion, verified: align the debug-build fallback with the acceptance item.**

[DESIGN.md](../../plans/BASELINE-GUARD/DESIGN.md), lines 90–94, permits a separately retained debug reference
set after diagnosis and a passing release verifier. M1.1 instead requires equality with APPLE-TILE's retained
M0 bytes, and PLAN.md's stop rules say to stop if those exact bytes cannot be reproduced in the gate. The
first experiment can legitimately show that no fallback is needed. Before using the fallback, settle which
rule governs and how the owner approves and maintains the derived references; the builder cannot satisfy
the current M1.1 merely by following that alternative design paragraph.

**S2 — Suggestion, verified: retain reproducible mutants alongside their logs.**

M1.2 asks the reviewer to read two mutant logs. Keeping the small mutation patches, base revision and exact
commands with those logs would let the reviewer reproduce both failures and distinguish a changed decoded
pixel from a malformed compressed file. M1.1's code inspection already helps reject an independent fake
rendering path; this would make the negative proof easier to audit.

The review started with the complete `briefs` array in
[ACCEPTED.json](../../plans/BASELINE-GUARD/RECORD.json), which contains one planning task, and the project
brief. For this module the required outcome is cross-module exact-baseline protection before landing,
including the failure exposed by APPLE-PRESENTATION's jitter and stale post-rebase evidence. CULLING and
EFFECT-GRAPH's engine features remain outside this review. A separate one-milestone tooling module, shared
game rendering path, owner-controlled reference bytes and recorded gate cost are reasonable choices within
that request. No visual-quality or performance promotion follows from this tooling work.

I read the complete DESIGN, PLAN and all four done-when items, then the module log, relevant architecture
rules, C12, contributor guidance, engineering review guidance and graded examples. I inspected the native
matrix scripts, Chrome comparator and capture entry points, the contributor check dispatch and the installed
land control flow. No prior design findings existed. The item assessment is:

| Item | Can its planned proof establish its statement? |
| --- | --- |
| M1.1 | Yes for its selected Mac checkpoints: execution plus inspection of the shared default path is meaningful. It does not establish the wider pre-landing outcome; see MF1 and the conditional S1 mismatch. |
| M1.2 | The two mutations can establish sensitivity to the original jitter failure and a one-value reference change. S2 would improve reproducibility. They do not cover MF1's integration gap. |
| M1.3 | Not with the specified inputs and mechanism: MF2 blocks the full clean-checkout matrix. Its standalone success would also not prove pre-push enforcement. |
| M1.4 | Yes: three recorded runs with load can establish added gate time. This is a cost record, not a numeric performance gate. |

The principal convincing incomplete implementation is a genuine passing subset guard plus a genuine full
verifier that is never enforced on the final revision before push. All four local proofs could be satisfied
once the reference issue is repaired, while that integration defect remains. MF1 therefore requires an
integration acceptance item, not merely more image checkpoints.

This was a design and interface review. I did not run the proposed, not-yet-implemented commands, capture GPU
output or judge motion. The reference inventory is an independent filesystem inspection, not visual evidence.
Only this review and its evidence folder were changed; DONE.json was not edited.
`./dev check-docs` passed (694 Markdown files, 3,587 local links, 30 module records, crate policy and design
hashes); it reported existing file-size notes only.

DESIGN CHANGES: 2 blocking findings
