# CONTENT-LOOP design review

13 November 2026. Reviewed commit `8d09e6b4d2a67b72fe88d449be1f94baf297e008`.

**Design changes requested: two Must Fix findings.** The proposed work substantially answers the request,
but its partial-save guarantee exceeds its mechanism, and one mandatory proof requires an ordering excluded
by the frame host. Both should be settled before builders are held to these items.

This is the first design review; there is no previous review to resolve. Findings below are verified against
the documents and code cited, not reproduced runtime defects in the proposed implementation.

## Must Fix findings

### MF1 — Partial-source-save guarantee has no mechanism

**Must Fix · verified · M3.7, with consequences for M3.5 and M3.8.**

[DONE.json](../../plans/CONTENT-LOOP/DONE.json), lines 300–302, promises that an “invalid or partial change
set keeps the previous set.” Its only reviewer question checks whether readers are confined to presentation.
That question cannot establish the second half of the statement.

The design retains the existing watcher and cooker, and [PLAN.md](../../plans/CONTENT-LOOP/PLAN.md), line 21,
explicitly retains presentation-only previews and defers source admission. The retained
[G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md), lines 182–192, expressly says that a glTF
and its buffer saved separately can publish an intermediate state. The implementation agrees:
[watch.rs](../../../crates/thrum-cook/src/watch.rs), lines 28–31 and 75–95, closes batches after a quiet
interval; [cooker.rs](../../../crates/thrum-cook/src/cooker.rs), lines 267–284, preserves failed assets while
publishing successful products. Neither mechanism knows the intended boundary of an external save.

An ordinary counterexample is a valid glTF and an existing valid buffer: save a changed glTF first, leave
the old buffer stable long enough for a cook, then save the intended new buffer. Choose changes whose
intermediate combination remains structurally valid. Stable hashes and product validation accept that
intermediate combination. This is different from M3.5's missing-buffer or concurrent-write cases. Keeping
all readers in presentation does not prevent it.

**Consequence:** building the stated mechanisms cannot satisfy the promised source-change atomicity. A
reviewer answering the specified question could accept the existing preview behaviour without proving the
partial-set guarantee. Atomic manifest publication and whole resident lease sets do not supply a source
transaction boundary.

**Resolution:** reconcile the promise with C05 and the recorded preview exception. If partial-source-set
rejection is required here, identify the source completion/admission mechanism and include its proof. If
the existing preview exception remains the intended scope, distinguish incomplete external saves from
invalid/unresolvable cooked products explicitly in the design and item; retain the required last-good
behaviour for the latter. Do not claim arbitrary partial-save rejection merely from presentation-only use.

### MF2 — The required old-frame-after-switch proof contradicts the host protocol

**Must Fix · verified · M2.3.**

[DONE.json](../../plans/CONTENT-LOOP/DONE.json), line 160, requires a test of “a frame encoded before the
switch and submitted after it.” [DESIGN.md](../../plans/CONTENT-LOOP/DESIGN.md), lines 168–175, places the
row switch at a frame boundary; lines 206–208 rely on the host allowing only one unsubmitted frame.

The prerequisite [EFFECT-GRAPH design](../../plans/EFFECT-GRAPH/DESIGN.md), lines 169–174, states that every
earlier encoded frame has committed or been dropped before another is recorded. This is enforced by
[runtime/tokens.rs](../../../crates/thrum-render/src/runtime/tokens.rs), lines 108–119: the encoded token
holds an exclusive mutable borrow of the host until submission or discard. The host cannot perform the
next frame-boundary row switch while an earlier encoded frame remains outstanding. A loader may make a
replacement ready during that interval, but readiness is not the committed row switch.

**Consequence:** the mandatory reviewer question cannot be answered affirmatively through the retained
production protocol. A test that privately mutates the arena around the token would manufacture a different
lifecycle and would not prove the supported path. Already-submitted frames still executing on the GPU are
a real case, but are not the encode-before/submit-after ordering the item names.

**Resolution:** make the proof match the chosen protocol: retain the discarded-switch and submitted-frame
lifetime requirements, and allow the exclusive-token invariant to establish that an unsubmitted old frame
cannot be overtaken by a row switch. If overtaking is actually intended, it needs an explicit change to the
host contract and a designed lifetime mechanism, rather than an incidental test requirement.

## Suggestions

### S1 — State how lease invalidation survives an abandoned consumer frame

**Suggestion · verified omission, not a demonstrated runtime fault · M1.2–M1.3, M1.8.**

[DESIGN.md](../../plans/CONTENT-LOOP/DESIGN.md), lines 137–140, reports a change once in the first frame
offering the new view. The upload may already be committed while that first consumer frame is discarded,
or is a capture that does not advance history. The consumer must retain its invalidation until an appropriate
submitted frame uses the new product. The contribution protocol already supplies the general rule
([contribution.rs](../../../crates/thrum-render/src/contribution.rs), lines 19–23 and 298–300), so this need
not introduce another residency state machine. State the consumer obligation in the public lease contract
and cover it in the diagnostic. M1.2 currently exercises discard of the final upload frame, a different point.

### S2 — Make the Chrome stall question agree plainly with the quality bar

**Suggestion · verified wording mismatch · M3.3.**

M3.3 asks whether edit intervals remain within the steady spread, then asks for a size and cause when they
do not. In contrast, M3.1 expressly requires a slower frame's cause to be outside this module. The M3 bar,
C2 and the design's worker fallback already require fixing content-induced stalls in the claimed range;
merely explaining one does not pass that bar. Say this directly in the Chrome criterion, separating the
required valley/twelve-volume range from exploratory larger-product measurements. No new numeric gate is
needed, and I have not treated the current wording as permission to waive the existing bar.

### S3 — Explain publication granularity across the three consumption paths

**Suggestion · verified ambiguity · design's edit journey and M3.2/M3.4.**

[DESIGN.md](../../plans/CONTENT-LOOP/DESIGN.md), lines 232–238, lets placements change in the last-copy
frame, leases in the next frame, and mesh rows after their separate bounded write. Explicitly describe what
a creator sees when one cooked mesh is both a placement and an entity row, or a texture serves both a
placement and a lease. The design clearly promises whole individual sets, but its “one frame shows the old
content and the next the new” cue can also read as one switch for all affected readers. Identify the intended
scope and its relationship to C05's affected presentation closure; add a coupled example if needed.

## Request and scope assessment

I read the complete `briefs` entry in [ACCEPTED.json](../../plans/CONTENT-LOOP/ACCEPTED.json) and the
[project brief](../../plans/BRIEF.md) before the design. The accepted task is the lead's wording, not a direct
owner quotation. The owner-approved brief supplies the controlling G5 and V1 requirements: content-addressed
cooking and runtime loading, visible live edits without GPU stalls, shipping isolation from authoring, and
Mac/Chrome delivery. It also keeps performance figures as baselines and leaves sea/cloud completion outside
the V1 exit.

The resulting scope is appropriate:

- M1 provides shared resident textures and volumes with product identity, change notification, bounded
  uploads, failure handling and device lifetime. That addresses the SEA and CLOUDS consumers without starting
  either port or importing their assets.
- M2 supplies cooked geometry to entity mesh rows and replaces it live. The architecture's explicit seam
  ruling assigns the necessary CULLING changes and protects its comparisons. This is supported by the
  showcase requirement, rather than an unrelated extension.
- M3 assembles visible edits under reconstruction and audits the G5 clauses, shipping boundary and maturity.
  T2 realtime captures and separate capture-off records distinguish visible behaviour from timing claims.
- Sprite-sheet/font metadata, array textures and pixel-art profile selection are acknowledged as later
  RENDER-2D/GAME-UI asks. The task asks to account for these consumers; it does not require their products
  before track 3. CAS collection, source admission and physical-iPhone encoding are explicit deferrals, not
  hidden claims of completion. MF1 concerns the inconsistent acceptance promise, not a demand to build all
  those deferred systems now.

M1 remains buildable when APPLE-PRESENTATION M2 passes: its additional EFFECT-GRAPH M2 dependency is already
`done` at this commit. M2 waits for CULLING M3; M3 waits for APPLE-PRESENTATION M3, which provides the required
TAAU. The generated diagnostic lets M1 prove leases without an unbuilt SEA or CLOUDS renderer. The source
already contains the fetched valley/browser cases needed as a starting point.

## Done-when assessment

All 27 items were read. “Judgeable” below assesses the planned obligation and proof, not implementation
completion. New test commands are expected to be implemented in their milestone.

| Item | Assessment at its milestone boundary |
|---|---|
| M1.1 | Judgeable: public-lease consumer, all portable mips against mapped bytes, and compressed GPU samples reject dummy handles or a private loader. |
| M1.2 | Judgeable: submission, last-copy discard and supersession cover upload publication; see S1 for the later consumer-history boundary. |
| M1.3 | Judgeable: identity/resource reuse, changed bytes and creation counts distinguish incremental reload from full reconstruction. |
| M1.4 | Judgeable: concurrent placement/lease loads test the common budget and ring; the bench supplies cost evidence. |
| M1.5 | Judgeable: the reviewer checks fault placement in the production failure path, recovery and last-good sets. |
| M1.6 | Judgeable: new-device emptiness and texel equivalence cover stale-generation views. |
| M1.7 | Judgeable on Chrome: fetched data, edit traffic and actual device recovery are observable. |
| M1.8 | Judgeable against the public lease surface and consumer briefs; S1 improves the handoff. |
| M1.9 | Judgeable with the registered exact-baseline verifier. |
| M2.1 | Judgeable: product-to-row equality and named admission failures have independent expected data. |
| M2.2 | Judgeable after replacement commits: independent fresh host, matched tick/view, growth and shrinkage exercise ranges, bounds and shadows. |
| M2.3 | Must change: its required overtaken encoded frame conflicts with the host, MF2. |
| M2.4 | Judgeable: write bounds, changed-row traffic, repeated-edit storage reuse and creation counters expose full rebuild substitutes. |
| M2.5 | Judgeable: last-good output, recovery, resize and new-device output compare with a fresh scene. |
| M2.6 | Judgeable on Chrome: fetched rows, unchanged page and fresh-page image equality provide an observable endpoint. |
| M2.7 | Judgeable with T2 realtime frames and native-size inspection; it adds visible switch/shadow evidence to the oracle. |
| M2.8 | Judgeable with the exact-baseline verifier. |
| M3.1 | Judgeable using ordinary-host capture-off records plus tripwires; the existing tripwire command alone is insufficient, as the question recognises. |
| M3.2 | Judgeable with built TAAU and realtime captures of all four edit kinds. |
| M3.3 | Judgeable against the M3 no-stall bar; see S2 to remove ambiguity about explained failures. |
| M3.4 | Judgeable with Chrome T2 realtime output under TAAU. |
| M3.5 | Judgeable for the enumerated invalid/unstable-source cases; it does not prove arbitrary partial-save rejection, MF1. |
| M3.6 | Judgeable: dependency graphs, release-symbol scan and the cooker positive control provide distinct checks. |
| M3.7 | Must change: the partial-set promise has no source-boundary mechanism and its question checks only readers, MF1. |
| M3.8 | Judgeable clause by clause once MF1's scope is reconciled; ledger labels cannot substitute for running evidence. |
| M3.9 | Judgeable from the public creator and engine-consumer documentation. |
| M3.10 | Judgeable with the exact-baseline verifier. |

## Review evidence and limits

This was a document and boundary-code review, including the shared contracts, architecture seam ruling,
prerequisite designs, SEA/CLOUDS briefs, contribution callbacks, exclusive frame token, mesh arena,
resident-set submission, cooker/watch publication, browser valley cases and shipping scanner. The repository
has no glossary and no owner-graded verdicts yet; the skill's starting examples and Thrum's standing review
calibration were read. No agents were started.

No runtime, visual-quality or device-performance result is claimed. No GPU run or capture was necessary to
establish these design contradictions, and no milestone verdict or planner-owned file was changed.
`./dev check-docs` passed (740 Markdown files, 4,504 local links, 30 module records, crate policy and current
design hashes). The review's relative links also resolve; the only repository addition is this review.

DESIGN CHANGES: 2 must_fix findings
