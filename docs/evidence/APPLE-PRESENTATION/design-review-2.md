# APPLE-PRESENTATION design review

Reviewed integrated commit `9f83a70ea4ef2e38fd0837ab9c391184777cb262` on 10 November 2026, against the previous review of `4bbe44a433598650be47c1753c57da79f5d1df93` and both accepted briefs. No other agents were launched.

**Result: DESIGN OK for M1–M3 and seams 1–4 only.** The previous Must Fix is resolved in the design and acceptance proofs. There are no blocking findings and one minor Suggestion. This is not implementation, visual-quality, cadence or device-performance qualification. M4–M11 and seams 5–9 still require planning and their own design review; the named APPLE-TILE dependencies still apply.

## Previous findings

### 1. Must Fix — Chrome temporal semantics lacked a discriminating proof: resolved

The previous D2.5 relied on existing browser cases that did not observe reactive values or temporal predecessors. Revised [D2.5](../../plans/APPLE-PRESENTATION/DONE.json), lines 125–134, commissions a new `temporal` case and a reviewer question that binds its observations to the actual submitted browser frame path. The [design's witness](../../plans/APPLE-PRESENTATION/DESIGN.md), lines 176–214, defines seven rows and explicitly fails unavailable observations rather than skipping them.

The new proof distinguishes the ordinary wrong implementations identified last time:

- A target that is only cleared fails W1's reactive value of 1 on invalid-history geometry and W3/W4's positive volume/celestial values.
- A permanently reactive target fails W2's valid successor and W5's steady and uncovered regions.
- An encoded but unsubmitted view incorrectly becoming history fails W6's predecessor identity. That identity must come from the state used to pack previous matrices, not an independent diagnostic counter.
- Previous matrices advancing prematurely fail W7: after submitted A, an abandoned view at pose P and submitted C at the same pose P, C must retain the nonzero A-to-P camera motion. A report with correct labels but the wrong matrices cannot satisfy both observations.

The mechanism has a concrete host integration point. In [web.rs](../../../crates/thrum-app/src/web.rs), lines 254–306, the browser advances, encodes, attempts surface acquisition, then either submits or calls `encoded.skip()`. The [frame tokens](../../../crates/thrum-render/src/runtime/tokens.rs) own the unsubmitted encoder and its cancellation. Thus the required abandoned view can exercise prepared/encoded state, rather than merely omit an animation callback. The existing configure-failure injection in [check_v1_browser.mjs](../../../scripts/check_v1_browser.mjs), lines 246–262, is only an example of injection plumbing: configure happens before encoding and cannot by itself satisfy the revised latest-abandonment question. The new case must reach the actual post-encode abandonment path and record that fact, as the item already requires.

The [browser submitted-capture export](../../../examples/v1-game/src/browser/submitted_capture.rs), lines 17–44, supplies image bytes and submission identity without a redraw. The proposed diagnostic view or same-submission attachment readback can use that path; per-frame predecessor reporting is explicitly new work in the plan. The current checker does not yet implement `temporal`, which is expected for an unbuilt milestone. Approval judges the commissioned proof, not an execution of a nonexistent case.

### 2. Suggestion — Reproducible reconstruction reference and tolerance: resolved

[Design lines 247–268](../../plans/APPLE-PRESENTATION/DESIGN.md) now specify scene-linear accumulation before the common SDR transform, all held state, two identical 32-phase cycles, the separately accumulated reference, the 64-frame TAAU settling interval and the unjittered no-AA comparator. They accurately call the result a 32-sample reference rather than claiming convergence or 64 distinct samples.

The metric is whole-frame CIEDE2000 on decoded SDR sRGB with D65 white. The p99 bound remains at most 3 and strictly below the no-AA result. A miss is a failure; revising the bound requires evidence, a plan revision and design review. D3.2 retains its broken-history negative control. The revision removes ambiguity without relaxing the acceptance bound.

### 3. Suggestion — Positive reactive checks, including the separate star pass: resolved

D2.2 now explicitly covers each drawer's declarations and the separate star pass's graph accesses and attachments. The values formerly included in that item have moved into the shared witness and new D2.6, rather than disappearing. D2.6 requires Mac GPU readback through the real forward and star passes, checking invalid/valid geometry, volume alpha, phase edits, steady celestial pixels and uncovered pixels. Chrome runs the same defined rows through D2.5.

This addresses the concrete integration risk: [stars.rs](../../../crates/thrum-render/src/synthetic/stars.rs), lines 238–325, currently declares and attaches colour alone. The design now explicitly commissions the motion/reactive attachment changes as well as shader outputs. Merely adding a fragment output does not satisfy the revised proof.

### 4. Suggestion — Preserve the later planning boundary: resolved

[Design lines 25–33](../../plans/APPLE-PRESENTATION/DESIGN.md) and [plan lines 9–15](../../plans/APPLE-PRESENTATION/PLAN.md) explicitly limit approval to M1–M3 and seams 1–4. They retain the later review obligations for the EDR transform, normalized MetalFX output/exposure handoff, generation-build lifecycle and archive mechanism. This review preserves that limit.

## New findings

### Suggestion — Align the cost-table milestone label with D3.5

**Verified documentation inconsistency; nonblocking.** The [design cost table](../../plans/APPLE-PRESENTATION/DESIGN.md), line 470, still places “Both are measured (M3, M8)” in the TAAU-at-1080p row. Revised D3.5 and design lines 546–547 correctly separate M3's 640×360 result from the 1080p estimate, which awaits M4's surface extent.

Update that table's milestone label during the next planning edit so readers do not mistake the M3 record for a 1080p measurement. The authoritative item and explicit open-question text already prevent that interpretation from passing review, so this does not hold up M1–M3.

## Request and inspection scope

I read both `briefs` entries in [APPLE-PRESENTATION-ACCEPTED.json](../../plans/APPLE-PRESENTATION/ACCEPTED.json), oldest first and in full. The original task commissions the remaining road to the V1 exit plus water and clouds, continuous buildable work, module ownership/contracts, protection of concurrent PLATFORM-RUNTIME-MAC M2 and APPLE-TILE definitions, and explicit owner decisions. The second task commissions the revisions requested by the previous review; it does not narrow the original obligations.

For this module the request requires shared temporal inputs and reconstruction, followed by the named display, governor and preparation work. Sea/cloud consumers must use those shared contracts. M1–M3 supply the early foundation; the remaining work stays explicitly assigned and deferred for further planning. No revised criterion changes simulation authority, silently drops a later V1 requirement, or edits a concurrent producer's definition.

Inspection questions used:

- Does each prior finding have a concrete resolution in the design and authoritative items?
- Have moved or narrowed statements retained their behavioral obligations elsewhere?
- Can the browser witness observe both GPU values and the actual predecessor state through the real frame path?
- Would cleared-only reactive output, sticky invalidity, or premature history advancement satisfy the revised proof?
- Does the reference hold the relevant state and preserve a separately judged candidate and negative control?
- Is the approval boundary distinct from runtime, visual and performance qualification?

I read the design, plan, all 18 current items, and the changes to those three files since `4bbe44a43359`. Supporting reads covered README, the build-plan purpose/exit, the architecture brief and relevant module-list entries/seam rulings, ORCHESTRATE, both graded-example sources, Thrum's review guidance, C04/C06/C07/C12, RENDER-SPEC's temporal/ABI rules, APPLE-TILE's storage/pre-tone/drop handoffs, and the browser/frame-host/star/capture sections cited above. There is no modules-folder BRIEF.md or GLOSSARY.md. This was a scoped design/interface inspection, not a full renderer or unrelated integrated-change review.

## Acceptance-item audit

These are judgments of whether the planned proofs are usable, not milestone passes.

| Changed or added item | Judgment against the earlier review |
|---|---|
| D2.2 | Judgeable. Separates declarations/graph consistency from values; explicitly covers direct/indirect cooked paths, the contribution seam and star attachments. Value obligations remain in D2.5/D2.6 and the design. |
| D2.5 | Judgeable; previous blocker resolved. W1–W7 and the source-tracing question distinguish inert reactivity and wrong temporal predecessors on the browser path itself. |
| D2.6 | Judgeable. Positive and zero GPU readbacks cover W1–W5 on Mac using the same fixture definition as Chrome. |
| D2.7 | Judgeable as regression only. Preserves the old browser suite without treating its result as temporal correctness. |
| D3.2 | Judgeable. Reference state, colour domain, metric and binding tolerance are defined; separate accumulation and history-disabled failure remain mandatory. |
| D3.5 | Judgeable as a timing record. Retains pairing, build/load/order and the tripwire distinction while correcting the extent claim. |

The unchanged D1.1–D1.5 retain ABI/sequence, GPU motion, independently derived sky motion, invalidity lifecycle and route inspection proofs. D2.1/D2.3/D2.4 retain storage eligibility, exposure rescaling and pipelined submission/failure/drop assertions. D3.1/D3.3/D3.3c/D3.4 retain registry/pre-tone integration, separate Mac and Chrome motion reviews and the reset negative control. The revision neither weakens these proofs nor supplies a reason to reopen their prior design-review judgments.

`./dev check-docs` passed (620 Markdown files, 2,867 local links, 30 module records, crate policy and current design hashes), and `git diff --check` passed. These are repository checks, not qualification of the planned witnesses.

No GPU runs or captures were needed for this proof-definition review. No motion was watched, and no runtime or visual result is claimed. Only this review file was changed; the done-when file, plans and accepted record remain untouched.

DESIGN OK
