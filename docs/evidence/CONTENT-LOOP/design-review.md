# CONTENT-LOOP design review

13 November 2026. Reviewed commit `a06204535dde8dc3e87278c399a1ebe4215fc6c7`, against the previous review
of `8d09e6b4d2a67b72fe88d449be1f94baf297e008`.

**Design changes requested: one Must Fix finding.** Both previous Must Fix findings are resolved. The design
substantially answers the human-approved brief, but the revision explicitly defers C05's requirement to publish
the affected presentation closure together. The recorded exception for incomplete external saves does not
settle this separate interface change.

## Must Fix finding

### MF3 — Cross-reader publication is deferred against C05

**Must Fix · verified design/interface conflict · DESIGN.md lines 237–248; PLAN.md line 22; consequences for
M3.2, M3.4 and M3.8.** This is distinct from MF1's external-save boundary and MF2's impossible frame ordering.

The revised [design](../../plans/CONTENT-LOOP/DESIGN.md#what-a-publication-promises-m3) expressly permits a
single cooked product to appear new through one reader and old through another. Placements switch in the
last-copy frame, leases in the following frame, and mesh rows after their separate bounded write. It acknowledges
that C05 asks for one switch across readers, but defers that to source admission or a later visible mismatch.
The new [plan assumption](../../plans/CONTENT-LOOP/PLAN.md#open-for-the-owner) repeats this deferral.

[C05](../../design/CONTRACTS.md#c05-assets-streaming-and-retirement--f4-with-w2-v1-and-p7), lines 191–196,
requires a qualified visual preview to compute the entire affected presentation closure, prepare replacements
and publish at extraction. It calls these correctness invariants. The
[G5 exception](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md#contract-questions-recorded-reversible-option-taken),
lines 182–192, permits an intermediate *source* state to become a presentation-only preview because the watcher
cannot identify a completed external save. It does not establish that the representations of one already-cooked
publication may switch independently. The module list's row-replacement ruling also does not change C05.

**Ordinary counterexample:** draw two instances of the same cooked mesh, one as a placement and one through an
entity mesh row. Finish a valid source edit before cooking it, with no partial save. Make the replacement large
enough that the row's bounded write takes additional frames. The revised design deliberately shows the placement
with the new silhouette while the entity still has the old silhouette. A texture shared by a placement and a lease
has the same issue for one frame. This outcome follows directly from the specified schedule; no runtime defect
is claimed to have been reproduced.

**Consequence:** implementing the design exactly would violate the existing presentation-publication invariant.
The revision expressly restricts M3's four edit demonstrations to products read through one path, so all those
demonstrations can pass without exposing the conflict. Acknowledging the limit as an assumption makes it visible,
but does not resolve it with the shared contract's producers and consumers.

**Resolution:** reconcile the cross-reader publication rule before building dependent work. Under the existing
contract, design a publication point at which every affected reader is ready and switches together, and require a
coupled-reader proof at the milestone that provides those paths. Alternatively, settle and record a scoped
interface exception with the affected producers and consumers. Merely postponing it to source admission leaves
the conflict open: a completed cooked publication already has an identity even when its sources are previews.
This finding does not require implementing T1 or rejecting incomplete external saves.

The previous S3 asked for the publication scope and its relationship to C05 to be explained. That explanation is
now present; it establishes a deliberate contract deferral, rather than just ambiguous wording. This finding is
about that newly explicit decision, not a request for an additional optional edge case.

## Previous findings and suggestions

- **MF1 — Partial-source-save guarantee has no mechanism: resolved.** C4 and the new publication section separate
  detectable invalid inputs from incomplete saves that remain valid. M3.7 no longer promises to reject arbitrary
  partial saves; it requires presentation-only readers and convergence to a fresh cook after the save finishes.
  M3.8 and M3.9 carry the limitation into the ledger and public documentation. This follows the existing G5
  exception. The watcher's quiet interval is not treated as a transaction boundary. The cooker includes the glTF
  and buffer hashes in its action key, making the proposed final-product comparison a meaningful proof
  ([watch.rs](../../../crates/thrum-cook/src/watch.rs), lines 28–31 and 75–95;
  [cooker.rs](../../../crates/thrum-cook/src/cooker.rs), lines 326–355 and 517–540).
- **MF2 — The required old-frame-after-switch proof contradicts the host protocol: resolved.** M2.3 now requires
  the production host protocol, submission-time commitment, discarded-switch recovery and supersession before a
  switch commits. The design routes writes, switching and retirement through the exclusive encoded-frame token.
  That matches [tokens.rs](../../../crates/thrum-render/src/runtime/tokens.rs), lines 108–119, and
  [EFFECT-GRAPH's tentative-recording rule](../../plans/EFFECT-GRAPH/DESIGN.md). Already-submitted frames remain
  protected by queue order; the impossible overtaking test is removed without removing the old geometry's
  lifetime obligation.
- **S1 — State how lease invalidation survives an abandoned consumer frame: resolved.** The per-product serial
  remains different from the consumer's committed serial until a frame using it calls `submitted(true)`. M1.3
  distinguishes discarding the first *consumer* frame from discarding the upload, and covers capture frames.
  M1.8 documents the obligation. This matches the existing contribution callback's history flag
  ([contribution.rs](../../../crates/thrum-render/src/contribution.rs), lines 298–300).
- **S2 — Make the Chrome stall question agree plainly with the quality bar: resolved.** M3.3 explicitly fails
  content-induced stalls at the valley and twelve-volume fixture sizes. Larger-product measurements remain
  measurements, not new numerical acceptance gates.
- **S3 — Explain publication granularity across the three consumption paths: explanation supplied; the contract
  consequence is MF3 above.** The coupled examples and the expected interval of disagreement are now explicit.

## Changed items and proof assessment

All 27 items and the milestone bars were read. Each of the eight changed items was compared with its earlier
wording and the previous findings. These are judgments of planned proofs, not implementation passes.

| Changed item | Assessment at its milestone boundary |
|---|---|
| M1.3 | Judgeable. Retained identity and byte/creation counts remain; the new question catches a change notification consumed by a discarded or capture frame. A one-shot notification would fail. |
| M1.7 | Judgeable. “One change” now describes one product revision rather than an event lost on first offer. Fetched texels, changed-product traffic and device recovery remain required. |
| M1.8 | Judgeable. Public documentation must now cover the consumer's pending invalidation across discard and capture, without depending on SEA or CLOUDS being built. |
| M2.3 | Judgeable under the retained host protocol. The revised proof includes the frame after a discarded switch and supersession of that replacement, and checks that tests do not bypass the token. MF2 is closed. |
| M3.3 | Judgeable. Browser edit-window timing and causal attribution enforce the existing no-stall bar at the claimed sizes; an explained content-induced hitch cannot pass. |
| M3.7 | Judgeable. The independent empty-store cook rejects stale final products, while the reader audit independently proves presentation-only use. MF1 is closed. |
| M3.8 | The clause-by-clause G5 and maturity audit remains judgeable and honestly names source admission as parked. It cannot authorize MF3's interface deferral. |
| M3.9 | Judgeable. A creator must be told both what a detected bad save does and what a valid intermediate save can show. |

The unchanged items retain their earlier proof assessment. In particular, invalid-input retention in M3.5 is
separate from M3.7's valid intermediate save. M3.2 and M3.4 remain usable single-path visual proofs, but are not
proof of C05's affected closure under the newly explicit restriction. No further Must Fix was found in the
unchanged items.

## Suggestion

### S4 — Distinguish cooker publication from runtime last-good retention

**Suggestion · verified wording overreach.** [DESIGN.md](../../plans/CONTENT-LOOP/DESIGN.md), lines 225–226,
says every case in C4 leaves the previous product “published, drawn and lent.” C4 also includes device refusal,
upload failure and a name missing from a new manifest. Those happen after cooker publication; the runtime can
retain its last-good resident set without reverting the store's manifest. State the guarantee at the layer that
owns it. M1.5 and M3.5 already require the useful runtime behavior, so this wording does not add a build blocker.

## Request, prerequisites and review limits

Both complete tasks under `briefs` in [ACCEPTED.json](../../plans/CONTENT-LOOP/ACCEPTED.json) and the
[project brief](../../plans/BRIEF.md) were read before assessing the revision. The planning tasks are lead/script
instructions, not new human quotations. The controlling brief still asks for content-addressed cooking and
loading, visible edits without GPU stalls, authoring-free shipping, and Mac/Chrome delivery. The lease, live mesh
rows and G5 integration remain appropriate to that request. Future SEA/CLOUDS consumption and the later 2D/font
product asks remain accounted for without starting those modules or changing the V1 exit.

M1 can still begin when APPLE-PRESENTATION M2 passes: its additional EFFECT-GRAPH M2 prerequisite is already
`done`. M2 waits for CULLING M3, and M3 waits for APPLE-PRESENTATION M3's reconstruction. The diagnostic consumer
allows M1 to prove leases independently of unfinished sea/cloud renderers. No new dependency cycle was found.

This was a document and scoped boundary-code review. I read the full design, plan and done-when file, their
changes from the previous reviewed commit, the shared contract and recorded exception, the relevant architecture
rulings, and the watcher/cooker and frame/contribution boundaries cited above. The planner's log was read after
forming the findings. The repository has no glossary or owner-graded verdicts; Thrum's review calibration and the
skill's starting examples were read. No agents were started.

No runtime, visual-quality or device-performance result is claimed. No GPU run or capture was needed for these
design findings. No planner-owned file or milestone verdict was changed. `./dev check-docs` passed (740 Markdown
files, 4,511 local links, 30 module records, crate policy and current design hashes). `git diff --check` and the
review's relative-file links passed. Only this review file was changed.

DESIGN CHANGES: 1 must_fix findings
