# EFFECT-GRAPH design review

Reviewed 11 November 2026 at integrated commit `c9dd6cd0d58617e5b269054e9b9b5a564095ced9`.
Result: **changes requested — four Must Fix findings**.

This is the first design-review round; no previous `design-review.md` was present. I read the sole task in
`ACCEPTED.json.briefs` in full, the project brief and module-list interfaces, then the complete EFFECT-GRAPH
design, plan and all 21 done-when items. The request calls for a small resource milestone immediately after
APPLE-TILE M2, followed by the graph foundation sea and clouds need, and a way to catch changes to another
module's exact images before landing. It does not commission the sea or cloud implementations themselves.

## Must Fix findings

### MF1 — Repeated ticks reset persistent simulation

**Must Fix; verified contradiction.** Location: [DESIGN.md](../../plans/EFFECT-GRAPH/DESIGN.md), lines 144–148;
affected proofs: M2.2, M2.5 and M2.6.

The time rule first says that several display frames within one tick step once, then says that a tick which is
“not ahead” resets the effect. Equality is not ahead. After submitting a nonempty trail at tick 10, presenting
tick 10 again resets it. At 120 Hz presentation with the ordinary 60 Hz world clock, this is a normal frame,
not a rare discontinuity. A paused world repeats the same problem. This contradicts N2 and the M2 quality bar.

Distinguish equal tick, forward tick, backward tick and changed timeline explicitly. An equal tick must retain
the committed simulation state; define reset for the actual discontinuities. Make the oracle assert retention
of an already nonempty field across repeated ticks and pause, rather than merely reproduce the same erroneous
reset rule on CPU and GPU. Include backward-tick and timeline-change expectations in that same state table.

### MF2 — Cadence does not define which history result readers receive

**Must Fix; verified mechanism gap.** Location: [DESIGN.md](../../plans/EFFECT-GRAPH/DESIGN.md), lines 149–175
and the frame walkthrough at lines 225–230; affected proofs: M2.2–M2.5.

The design allows a skippable writer to target a history pair, says the executor binds the last submitted side
for reads and the other side for writes, and commits the effect's side on submission. It concludes that
persistence alone makes skipped readers see the last result. That conclusion does not follow for ping-pong.

For example, let A contain step 4 and let a submitted update write step 5 into B. In the next frame the writer
skips. A downstream reader still bound to that frame's nominal output side A sees step 4. Publishing A because
the frame submitted also makes the next update start from step 4. Keeping the side unchanged at submission is
necessary but does not itself redirect the skipped frame's downstream reader. The existing `ResourceScope::History`
declaration in `crates/thrum-render/src/graph.rs` tracks the physical resources; it does not supply this new
conditional binding/publication mechanism.

Define the tentative output and committed output for each independently updated history, including downstream
reads in run and skip frames, publication only when its producer actually writes, and discard/capture behavior.
Also settle initialization for time-sliced writes: running a pass once over a subset does not initialize every
texel a later pass may read. This matters on the first epoch and after reset. The resolution can constrain legal
declarations or define the bindings and initialization, but cannot rely solely on the persistent scope.

Require the cadence witness to read the consumer's result on each run/skip boundary, including two histories
with different cadences and a discarded update. One always-running history plus a skippable in-place texture
would not exercise the missing composition.

### MF3 — No refraction request incorrectly requires no stored depth

**Must Fix; verified interface conflict.** Location: [DONE.json](../../plans/EFFECT-GRAPH/DONE.json), M3.1,
lines 183–186; [DESIGN.md](../../plans/EFFECT-GRAPH/DESIGN.md), lines 183–194.

M3.1 requires that without a refraction request neither the copy nor stored depth exists, and asks the reviewer
to confirm that toggling refraction changes both. But depth and refraction are independent inputs in the design.
A non-refracting volume still needs this frame's depth to stop at terrain. Removing the colour-copy request
must not remove that depth reader's storage.

There is also another owner: APPLE-PRESENTATION M2's D2.1 requires stored, sampleable depth when reconstruction
is enabled, whether any effect requests refraction or not. C06 derives storage from all readers. EFFECT-GRAPH
M3 explicitly depends on that milestone and tests temporal sampling.

Separate colour-copy demand from depth demand. State that an absent effect introduces no additional storage;
do not require removal of storage another consumer needs. Make M3.1 cover a depth-only effect, a refracting
effect, and reconstruction's depth reader with no effect request. The expected graph must follow the union
of the actual accesses.

### MF4 — M3 cannot prove its reconstruction quality bar

**Must Fix; verified dependency/proof gap.** Location: [DONE.json](../../plans/EFFECT-GRAPH/DONE.json), M3
`needs` and `bar` at lines 175–179, M3.2, M3.5 and M3.6.

M3 promises stability under reconstruction, with no edge shimmer or ghost on a cut. Its dependencies provide
the replaceable pre-tone input and temporal inputs, not a running reconstructor. APPLE-PRESENTATION M3 builds
TAAU; M2 explicitly proves inputs before reconstruction exists. That module's M3 is not a dependency here.

The motion/reactive readback is a useful input proof, but can pass while a reconstructed effect shimmers or
ghosts. The only M3 visual proofs are fixed stills, which cannot reveal those temporal failures even if a
reconstructor happened to have landed first.

Align the bar with what this foundation is commissioned to supply: either qualify the temporal input seam
here and explicitly leave reconstructed visual acceptance to the named reconstruction/SEA/CLOUDS milestones,
or add the required reconstruction dependency and elapsed-time motion/cut inspections. Do not let incidental
build order decide whether a reviewer can judge the written bar.

## Suggestions

- **S1 — Make the portable resource proof explicit (verified coverage ambiguity).** N1 promises exact
  readback of every texel of every kind in Chrome as well as Metal. M1.1 is Metal-only; M2.5 names the same
  scripted state run but does not explicitly enumerate the cube, all volume texels and all array mips. Name
  those comparisons in the Chrome witness so its meaning is unambiguous. This sharpens the intended proof;
  the diagnostic already carries every kind.
- **S2 — Settle when named timing becomes available (verified sequencing ambiguity).** M2.7 asks for per-pass
  times on the valley route, while the design assigns pass-name timing to APPLE-TILE M3, required only by
  EFFECT-GRAPH M3. The current low-level `GpuTimings` does have named `PassTiming` values, so a scoped M2 proof
  is possible; the host currently reduces them to fixed fields (`runtime.rs:1578–1586`). Say which existing
  observation M2 uses, or move the shared host-schema proof behind its provider, to avoid competing migrations.
- **S3 — Do not imply that list order establishes medium validity (verified wording issue).** At design
  lines 185–186 the contribution order is described as C08's medium-segment order. It is an execution order;
  C08 additionally requires ordered disjoint segments or an explicit joint/approximate integration policy.
  Clarify that SEA/CLOUDS and V3 retain that admission/composition obligation. This foundation need not build
  their integrator.

## Item-by-item proof assessment

“Judgeable” below means that the planned proof can be assessed when built, not that it has passed now.

| Item | Design-review assessment |
|---|---|
| M1.1 | Judgeable: GPU writes/readback, every mip, resize and replacement; S1 concerns the later Chrome counterpart. |
| M1.2 | Judgeable: named refusal cases and non-aliasing distinguish descriptor support from acceptance-only stubs. |
| M1.3 | Judgeable: capability-based refusal plus an actual Chrome capability record. |
| M1.4 | Judgeable: pin existing variant pass order, storage and transient allocations against the pre-change graph. |
| M1.5 | Judgeable: release build and existing retained-byte matrix, not newly generated expected images. |
| M2.1 | Judgeable: external public-seam consumer beside the cooked contribution. |
| M2.2 | Resolve MF1 and MF2 before defining the CPU oracle; GPU/CPU agreement alone can preserve a shared mistake. |
| M2.3 | Resolve MF2 for history readers and first-epoch initialization. |
| M2.4 | Judgeable replacement test once the reset/publication rules in MF2 are settled. |
| M2.5 | Judgeable browser state/loss proof once MF1/MF2 are settled; clarify S1. |
| M2.6 | Appropriate realtime inspection of trail persistence and wrapping; MF1 currently contradicts it. |
| M2.7 | Judgeable with a stated timing observation; S2. |
| M2.8 | Judgeable retained-byte regression check. |
| M3.1 | Rewrite the conflicting storage expectation; MF3. |
| M3.2 | Judgeable motion/reactive/reset input witness, not reconstructed output proof; MF4. |
| M3.3 | Judgeable eligibility/refusal and unchanged-order check; it claims no overlap speedup. |
| M3.4 | Judgeable knob, invalidation and skipped-timing check after its named providers. |
| M3.5 | Browser check plus still inspection can prove composition and input behavior; cannot establish the temporal bar. |
| M3.6 | Noon/night stills on both recipes can prove crossings and background composition; cannot establish the temporal bar. |
| M3.7 | Judgeable documentation, paired cost and separated maturity evidence. |
| M3.8 | Judgeable retained-byte regression check. |

## Scope, evidence and calibration

The small first milestone is appropriately confined to resource descriptors, validation and allocation after
APPLE-TILE M2. Extending the existing contribution seam, keeping async eligibility declarative, and leaving
atmosphere migration and real sea/cloud implementations out are consistent with the request. No new package
or speculative rendering framework is required. Costs are recorded as tripwires rather than invented gates.

The baseline request is addressed by the module list's ownership rules, the planned BASELINE-GUARD land gate,
and an explicit retained-image check in every milestone. The interim gap before BASELINE-GUARD lands is disclosed;
builders are instructed to run these checks after rebasing and before landing. I read the matrix scripts:
`apple_tile_m1.py` compares against retained M0 bytes and records revision, diff and binary identity; the second
script adds the split-frame checks. No baseline refresh is authorized here.

Boundary inspection included the existing contribution callbacks, graph history declarations/tests, host
submission and capture callbacks, timing collection, C02/C04/C06/C07/C08/C12, the port decision, SEA/CLOUDS
briefs and the relevant APPLE-TILE/APPLE-PRESENTATION plans and items. This is a design review informed by the
existing implementation, not a blind technique study. No other agents were launched.

I also checked the cited primary references. External-resource lifetime is supported by
[Unreal RDG's external-resource documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/render-dependency-graph-in-unreal-engine#externalresources)
and [Unity's render-graph documentation](https://docs.unity3d.com/Packages/com.unity.render-pipelines.core@14.0/manual/render-graph-fundamentals.html).
The [WebGPU specification](https://www.w3.org/TR/webgpu/#limits) defines device limits, including storage
texture bindings; using the enabled generation's capabilities is appropriate. These precedents do not establish
the design's particular skipped-history publication rule.

No implementation or visual milestone is claimed, so I did not run the proposed GPU checks or create captures.
No running effect, motion quality or performance result is inferred from this review. The done-when file and
all other project files are unchanged.

Validation: `./dev check-docs` passed (697 Markdown files, 3,625 local links, 30 module records, crate policy
and current design hashes). `git diff --check` passed; the only added file is this review.

DESIGN CHANGES: 4 blocking findings
