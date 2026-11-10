# APPLE-PRESENTATION design review

Reviewed integrated commit `4bbe44a433598650be47c1753c57da79f5d1df93` on 10 November 2026. This is a design and acceptance-proof review, not implementation, visual-quality or device-performance qualification. No other agents were launched.

**Result: one Must Fix.** The early reconstruction design has a coherent path through the existing renderer and the proposed APPLE-TILE handoffs, but the newly added Chrome temporal-input item cannot establish the behavior it claims.

## Findings

### 1. Must Fix — D2.5's existing browser cases cannot prove reactive values or temporal-history behavior

**Verified proof gap; no faulty implementation is alleged or experimentally injected.** Location: [APPLE-PRESENTATION-DONE.json](../../design/modules/APPLE-PRESENTATION-DONE.json), D2.5, lines 117–123; [plan migration log](../../design/modules/APPLE-PRESENTATION-PLAN.md), lines 398–403.

The statement promises that “the reactive target and the history rules behave the same in Chrome,” but its sole proof is `node scripts/check_v1_browser.mjs --case all`, explicitly using the checker's existing cases. There is no `ask` or inspection of these temporal results. The migration log describes this addition as proving Chrome as its own target.

I inspected the case registry and dispatch in [check_v1_browser.mjs](../../../scripts/check_v1_browser.mjs), lines 12 and 1884–1895, and searched its assertions for reactive, motion and history observations. The suite exercises scene drawing, controls, recovery, culling and held route screenshots. Its sky “motion” records are paused camera/Sun samples (lines 1096–1145); its history reference at line 1799 concerns occlusion history. It does not read the reactive texture or check the temporal sample accepted by a successor. `all` also explicitly excludes the moving-route cases, although merely adding those existing cases would not supply the missing temporal assertions.

An ordinary wrong implementation can therefore declare a valid third attachment in Chrome, clear it to zero and leave it unwritten, while the valley still renders and the existing cases pass. At M2 reconstruction is not yet consuming that attachment, so the final-color screenshots cannot expose the fault. Likewise these cases do not distinguish advancing temporal metadata during an abandoned encode from advancing it only at submission. M2's Mac proofs and M3's later visual review do not turn this command into a Chrome proof of those rules.

**Required resolution:** give D2.5 a browser-executed semantic witness. Observe nonzero reactive output for a known invalid-history case, its cleared/valid counterpart, and the submitted-history identity across a submit and a skipped/cancelled view. Bind those observations to the actual browser frame path. An inert reactive target and premature history advancement must fail the witness. Reuse the shared M2 fixtures where possible; keep the existing suite as the regression check. A smaller claim of “third-target integration smoke test” would be accurate for the current command, but the cross-target temporal-correctness obligation would still need a named proof elsewhere.

### 2. Suggestion — Make the reconstruction reference and tolerance reproducible before collecting the first result

**Verified specification ambiguity.** Location: [plan](../../design/modules/APPLE-PRESENTATION-PLAN.md), lines 37–40 and 407–408; D3.2.

The separate accumulator, full-frame comparison, no-AA comparator and broken-history control are good protections. Clarify the color domain and precise ΔE definition, require the complete scene/time/exposure state to remain fixed during accumulation, and state how the provisional tolerance can be revised through review. “Restated after M3's first run” should not become permission to relax the bound merely because the candidate missed it.

Also, 64 frames using the continuous 32-phase sequence repeat the same 32 offsets twice. That is a legitimate deterministic reference, but it is not 64 distinct spatial samples. Describe it accurately and avoid treating the frame count alone as evidence of convergence. These are reproducibility improvements, not a demand for a larger supersampling project.

### 3. Suggestion — Include positive reactive-value checks in the shared M2 fixture

**Verified opportunity to sharpen proof.** Location: D2.2 and [design](../../design/modules/APPLE-PRESENTATION.md), lines 123–145 and 442–449.

D2.2's statement specifies useful values, while its reviewer question emphasizes target declarations, the steady-star zero case and the contribution seam. When adding the witness required by finding 1, reuse it on Mac to check invalid-history geometry, volume alpha and a celestial phase edit as positive cases. Include the separate star pass, not only the opaque forward pass. The current star implementation declares and attaches color alone in [synthetic/stars.rs](../../../crates/thrum-render/src/synthetic/stars.rs), lines 238–325, so its graph accesses and attachments need to follow its new shader outputs. The design already commissions this behavior; this suggestion makes the evidence more direct.

### 4. Suggestion — Keep the later planning boundary explicit in the next approval record

**Verified scope limitation, safely deferred.** Location: design lines 19–21; plan lines 9–15; [module list](../../design/modules/ARCHITECTURE.md), lines 159–169 and 215–221.

M4–M11 and seams 5–9 are expressly awaiting planning and reconciliation. Preserve that restriction when recording a future approval of M1–M3. In particular, review the final EDR transform, normalized MetalFX fallback's output/exposure handoff, generation-build lifecycle and archive mechanism when those designs are made concrete. The prose is useful direction, not qualification of those unsettled interfaces. No further planning of these later milestones is required to resolve this review's blocker.

## Request, scope and previous reviews

I read the sole `briefs` entry in [APPLE-PRESENTATION-ACCEPTED.json](../../design/modules/APPLE-PRESENTATION-ACCEPTED.json) in full before comparing the design. It commissions the remaining project road to V1 plus water and clouds, with module ownership, build order, contracts and briefs, and buildable plans for the next work. It protects the concurrent PLATFORM-RUNTIME-MAC M2 and APPLE-TILE definitions and leaves owner decisions explicit.

For this module, the resulting obligations are temporal inputs and reconstruction for the shared frame, followed by extent/transitions, MetalFX, EDR, governor and pipeline preparation. Sea/cloud consumers need the same temporal inputs and later quality knobs; this module must not introduce another simulation authority or a private effect reconstructor. The module map assigns those responsibilities and the design preserves them. The later work is named and explicitly deferred, rather than silently removed from V1. This review does not judge the separate sea/cloud finishing decisions or authorize their port timing.

There was no previous `docs/evidence/APPLE-PRESENTATION/design-review.md` in this checkout. The plan summarizes older reviews outside the loop; I do not treat those summaries as independent approval of this revision. There is no modules-folder BRIEF.md or GLOSSARY.md. I used the architecture brief, build-plan exit and module list, together with ORCHESTRATE, the review skill, Thrum's review guidance and both graded-example sources.

## Inspection and acceptance audit

Questions used for the review:

- Does the staged module cover its assigned part of the accepted request without changing the exit or concurrent producer definitions?
- Can a frame carry correct motion, invalidity and exposure through forward contributors, retained history, reconstruction and the pre-tone input?
- Do submission, cancellation, replacement and reset share consistent ownership?
- Can each proposed item be judged on its named target, and would an inert implementation pass its proof?
- Are later technical decisions and performance estimates kept separate from present qualification?

I read the complete design, plan and all 16 done-when items. Boundary reads included C04/C06/C07/C12, the current RENDER-SPEC temporal/ABI/exposure rules, APPLE-TILE's consumer handoffs, and the relevant current history, view, synthetic/cooked motion, sky, star and contribution code. I also inspected the showcase/browser proof entry points. This was a scoped interface inspection, not a full read of the renderer or every browser-checker case body.

| Item | Design-review judgment |
|---|---|
| D1.1 | Judgeable: sequence/ABI assertions plus an explicit last-submitted-matrix question. The accepted seam names the required specification update. |
| D1.2 | Judgeable: actual GPU motion readback and a moving-object positive control distinguish zero-motion correctness from an inert target. |
| D1.3 | Judgeable: an independently derived expectation, numerical bound and phase-only control cover more than camera motion. |
| D1.4 | Judgeable: the question traces setting and clearing invalidity across four lifecycle cases and both cooked paths. |
| D1.5 | Judgeable: a named motion visualization and whole-frame/crop sequence on the real route. |
| D2.1 | Judgeable: actual-reader eligibility and the unchanged disabled-storage path are explicit. Existing non-reconstruction readers must still be respected. |
| D2.2 | Judgeable through the value statement and contributor code/tests; suggestion 3 strengthens direct GPU evidence. |
| D2.3 | Judgeable: a changing-storage-scale test must demonstrate rescaling, not merely the presence of an exposure texture. C07 still requires retaining the matching sample. |
| D2.4 | Judgeable: the real-host pipelined test separately asserts submission, failure, cancellation, background reset and key semantics. A Mac simulation of the iOS drop is not an iOS device-performance claim. |
| D2.5 | **Must Fix:** the named existing command cannot distinguish the claimed Chrome temporal semantics from an inert attachment or wrong history advancement. |
| D3.1 | Judgeable: live route execution on both targets plus the registry/pre-tone code question prevents a second tone step or an unused reconstructor. |
| D3.2 | Judgeable with the separate reference and negative control; suggestion 2 clarifies reproducibility. |
| D3.3 | Judgeable: five specified sequences, on/off pairs, 1x playback and native-size crops cover distinct temporal failures on Mac. |
| D3.3c | Judgeable: the corresponding Chrome realtime sequences give a separate visual result. They do not replace M2's input/history-correctness proof. |
| D3.4 | Judgeable: independently reset output and a suppressed-reset control make the cut assertion discriminating. |
| D3.5 | Judgeable as a timing record, not a performance gate. Record M3's actual 640×360 extent separately from the design's 1080p estimate. |

The submission-order proposal accounts for portable cancellation, native-provider reset after discarded encoding, and late execution error through device recovery. The revised APPLE-TILE handoff retains the ordinary SDR stages and supplies a replaceable pre-tone input; this design does not assume the removed fusion or optimization milestones. The Chrome cooked valley and route exist in the current source, despite the stale README status sentence. Qualified target capture modes are available through the T2 handoffs. These are design/dependency findings, not fresh runtime passes.

No GPU runs, visual captures or repository test runs were needed to establish the proof-definition defect. No motion was watched and no visual or cadence result is claimed. Only this review file was written; the done-when file and all plans remain unchanged.

DESIGN CHANGES: 1 blocking findings
