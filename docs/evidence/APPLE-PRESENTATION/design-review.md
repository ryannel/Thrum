# APPLE-PRESENTATION design review

Reviewed integrated commit `a471fe5d83beb926bdd1cf49e123696090d4e262` on 11 November 2026 against both accepted briefs, oldest first, and the previous design review of `9f83a70ea4ef2e38fd0837ab9c391184777cb262`.

**Result: DESIGN OK for M1–M3 and seams 1–4 only.** No blocking findings. The migration preserves the acceptance proofs and the earlier approval boundary. M4–M11 and seams 5–9 still require planning and another design review. This is a design review, not implementation, visual, cadence or device-performance qualification.

## Previous findings

### Must Fix — Chrome temporal semantics lacked a discriminating proof: remains resolved

The immediately previous review had no outstanding Must Fix. It resolved the earlier finding, “D2.5's existing browser cases cannot prove reactive values or temporal-history behavior,” from [round 1](design-review-1.md). That resolution survives this revision.

[D2.5](../../plans/APPLE-PRESENTATION/DONE.json) still requires W1–W7 on the game's submitted Chrome frame path. The [witness definition](../../plans/APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence), lines 176–214, retains positive and zero reactive values, predecessor identity sourced from the previous-matrix state, and a nonzero camera-motion witness after an abandoned encode. A merely cleared target fails W1/W3/W4; permanently reactive output fails W2/W5; premature history advancement fails W6/W7. An unavailable observation fails rather than being skipped.

The current [browser frame path](../../../crates/thrum-app/src/web.rs), lines 254–306, still permits encoding followed by failed acquisition and `encoded.skip()`. Thus the abandonment witness has a real integration point. A failure before encoding alone would not satisfy D2.5's latest-abandonment requirement. The [submitted capture export](../../../examples/v1-game/src/browser/submitted_capture.rs), lines 17–44, still exports pixels and submission identity without a redraw. The proposed temporal diagnostics remain work for M2; their absence today is not a design failure.

The prior resolutions for the reconstruction reference, positive reactive checks including the separate star pass, and the later planning boundary also remain intact. The reference retains held state, separate scene-linear accumulation, a binding CIEDE2000 bound and a broken-history control. D2.2/D2.6 retain graph/attachment inspection and actual reactive readback. The design and plan expressly prohibit starting M4 or later on this approval.

### Suggestion — Align the cost-table milestone label with D3.5: still open

**Verified documentation inconsistency; nonblocking.** [Design line 470](../../plans/APPLE-PRESENTATION/DESIGN.md#the-system-as-it-runs) labels the 1080p TAAU estimate with “Both are measured (M3, M8).” D3.5 and design lines 546–547 correctly say M3 measures 640×360 and the 1080p measurement awaits M4's surface extent.

On the next planning edit, make the table name the appropriate extent and milestone. The authoritative item already prevents a 640×360 measurement from qualifying the 1080p estimate, so this suggestion does not hold up M1–M3. No new Must Fix is raised.

## Request and migration inspection

I read both tasks in [ACCEPTED.json](../../plans/APPLE-PRESENTATION/ACCEPTED.json) in full before comparing the design. The original request commissions the remaining road to V1 plus water and clouds, with continuous buildable milestones, module ownership and contracts, protection of concurrent PLATFORM-RUNTIME-MAC M2 and APPLE-TILE definitions, and explicit owner decisions. The retry task commissions the earlier review's corrections; it does not replace the original purpose.

For this module, the [project brief](../../plans/BRIEF.md) and [module list](../../plans/ARCHITECTURE.md) require shared temporal reconstruction, followed by extent transitions, MetalFX, EDR, the quality governor and pipeline preparation. Sea and cloud contributors use the shared temporal contract and later governor. The early milestones supply that foundation without introducing simulation authority or a private effect reconstructor. Later obligations remain assigned and explicitly deferred, as the previous review allowed.

Inspection questions:

- Did migration remove or weaken a criterion, prerequisite or prior resolution?
- Do normal frames, skipped views and exposure changes retain one history owner and matching sample state?
- Can each early milestone prove its outcome with the preceding milestones and named producer dependencies?
- Would inert reactive output, premature history advancement, missing temporal accumulation or disabled cut reset pass the specified proofs?
- Does approval remain distinct from the later presentation design and from runtime qualification?

I compared the current files with their old `docs/design/modules/` locations at `9f83a70e`. `DONE.json` is **byte-for-byte identical**, including all 18 items, quality bars and dependencies. The original design body changes links and its log reference; the appended game outcomes, evidence conventions, ownership and dependency sections were moved from the former plan. The shorter plan retains M1–M3's implementation work and links the later drafts. Moving those drafts into evidence does not promote them into approved acceptance criteria.

The module list's explicit seam ruling remains authoritative: the APPLE-PRESENTATION builder owns M1's motion-only atmosphere edit and M2's third-target changes. The older “Atmosphere owner” labels in the carried work breakdown do not revoke that ruling or introduce a new dependency.

Boundary inspection covered C04/C06/C07, RENDER-SPEC's view/history rules, and APPLE-TILE's stored-resource, pre-tone and acquisition-refusal contracts. Submission-order acceptance is an explicitly accepted seam change, with bounded delayed-error contamination and reset specified; it is not an accidental assumption that CPU completion equals acceptance. The pre-tone handoff retains matching exposure and identity and one SDR transform. APPLE-TILE remains a producer prerequisite, with no reverse dependency on G4's later readers or scale proofs.

## Acceptance-item audit

These are judgments of the planned proofs, not milestone verdicts. All items were read; none changed since the approved revision.

| Item | Design-review judgment |
|---|---|
| D1.1 | Judgeable: sequence and byte-layout checks, specification update and last-submitted-matrix inspection. |
| D1.2 | Judgeable: actual GPU motion readback across jitter phases and a moving-object positive control distinguish inert output. |
| D1.3 | Judgeable: independently derived motion expectation, numerical bound and phase-only case. |
| D1.4 | Judgeable: traces marking and clearing through all four invalidity cases and forward drawers. |
| D1.5 | Judgeable before reconstruction: the existing display pass exposes motion, with a fixed sequence and explicit regions to inspect. |
| D2.1 | Judgeable after APPLE-TILE M2: graph readers and eligibility reports expose stored versus disabled behavior; M3's finished reconstructor is unnecessary. |
| D2.2 | Judgeable: drawer declarations, cooked paths, contribution seam and separate star attachments are inspected together. |
| D2.3 | Judgeable: retained exposure/storage scale and a changed-scale resampling test can be exercised before TAAU exists. |
| D2.4 | Judgeable: actual host tests separately assert submission, failure, cancelled acquisition, background refusal and extent-key behavior. |
| D2.5 | Judgeable: W1–W7 observe Chrome's temporal values and actual predecessor state; the prior blocker remains resolved. |
| D2.6 | Judgeable: Mac GPU readbacks exercise W1–W5 through the real forward and star passes against the shared fixture. |
| D2.7 | Judgeable as regression only; it explicitly makes no temporal-correctness claim. |
| D3.1 | Judgeable after APPLE-TILE M3: registry selection and the pre-tone insertion are traced; the moving command is smoke, with motion qualification in D3.3/D3.3c. |
| D3.2 | Judgeable: independent accumulation, held-state checks, whole-frame metric, binding tolerance and disabled-history negative control. |
| D3.3 | Judgeable: Mac realtime engine captures, on/off pairs, native-size crops and an explicit requirement to watch motion. |
| D3.3c | Judgeable: the same sequences are independently inspected in Chrome using realtime capture. |
| D3.4 | Judgeable: independently reset comparison and suppressed-reset negative control discriminate a broken cut. |
| D3.5 | Judgeable as a paired timing record at 640×360, with load/build/order labels and no invented performance gate. |

## Evidence and limits

Read the complete current design, plan and done-when file; both accepted briefs; the previous reviews; README, ORCHESTRATE, project brief and relevant module-list entries; Thrum's review guidance and both graded-example sources. There is no plans-folder glossary. This is a scoped design/interface review, not a complete renderer code audit. No agents were launched.

`./dev check-docs` passed after writing this review: 661 Markdown files, 3,180 local links, 30 module records, crate policy and current design hashes. `git diff --check` also passed. No GPU runs or captures were needed for this migration and proof-definition review. No motion was watched and no running behavior is claimed. Only this review file is changed; plans, DONE.json and ACCEPTED.json remain untouched.

DESIGN OK
