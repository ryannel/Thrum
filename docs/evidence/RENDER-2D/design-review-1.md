**RENDER-2D design review — changes requested**

Reviewed on 13 November 2026 at `fe5c89f9a19238e83e829db8775eb79b0eedf063`. This is a design review, not implementation, visual or device qualification. There was no previous design review in this checkout. There are **two Must Fix findings**, both verified below.

The design addresses the human's requested 2D engine support for the swarm-survival game, but its ordering mechanism cannot deliver its stated stability guarantee, and its reduced frame contract needs reconciliation with C07 before approval.

**1. Must Fix — `sort-band-does-not-prevent-reversals` — verified by calculation.**

The guarantee in M2.2 and M5.3 is not supplied by the held-value rule in [DESIGN.md, mechanism 4](../../plans/RENDER-2D/DESIGN.md#4-order), lines 199–205. Filtering each row's absolute sort value independently does not give the promised hysteresis between overlapping rows. Ordinary small movements after a shared approach can reverse their order and reverse it back without either row moving by the band.

I executed the stated rule in Python, using `b = 1`, two world rows on the same layer, persistent identities A and B, and overlapping sprites. Both held values start at their true values. On each tick, a value outside the band moves its held value to `true - b` or `true + b`, as the design specifies. The resulting ascending order is:

| Tick | True A | True B | Held A | Held B | Back to front |
|---|---:|---:|---:|---:|---|
| 0 | 0.0 | 0.1 | 0.0 | 0.1 | A, B |
| 1 | 2.0 | 2.1 | 1.0 | 1.1 | A, B |
| 2 | 2.2 | 2.1 | 1.2 | 1.1 | B, A |
| 3 | 2.1 | 2.3 | 1.2 | 1.3 | A, B |

After the approach at tick 1, each row's range is only 0.2. Even total travel is below the band: 0.3 for A and 0.2 for B. Nevertheless, the overlap reverses twice in two ticks. This directly contradicts [DONE.json](../../plans/RENDER-2D/DONE.json), M2.2 and M5.3 (the latter at lines 563–566), and the C3 cue. The example scales with any positive band, so choosing a larger band from the crowd's measured jitter does not establish the guarantee.

The update used for the reproduction was:

```python
if true > held + b:
    held = true - b
elif true < held - b:
    held = true + b
```

Resolve the relationship between the desired stable overlap order and its mechanism, with an independently checkable argument or reproduction that includes this case. Preserve a total order across multiple overlapping rows. If the intended guarantee is weaker, state that change explicitly and reconcile the cue, mechanism and items; selecting a favourable Crowd trace is not proof of the current general promise. No GPU or crowd run was needed to establish this mathematical counterexample, and none was performed.

**2. Must Fix — `c07-no-reconstruction-exception-unresolved` — verified against the shared interface.**

[DESIGN.md](../../plans/RENDER-2D/DESIGN.md), lines 361–366, says no contract text changes, then proposes that a 2D view publishes projection, exposure and extent without motion, depth, reactive mask or jitter. [C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4), lines 289–293, currently requires each view to publish those fields; only its guide buffers are optional. The [hardware-native decision's frame contract](../../design/decisions/HARDWARE-NATIVE-ENGINE.md#frame-contract) repeats that requirement.

The module list's [seam ruling 1](../../plans/ARCHITECTURE.md#seam-rulings-for-render-2d) correctly assigns the work and describes the desired no-reconstruction mode. It does not remove the conflicting requirement from the shared authority. As written, a builder cannot meet both M1.2's prohibition on allocating those resources and C07's requirement to publish them. The design explicitly invites this reviewer to decide whether its exception changes C07: **it does**.

Reconcile the exception in C07 and its associated hardware-native guidance with the producing frame runtime and consuming presentation/provider rules, using the existing seam ruling. The resolved contract should distinguish a view that declares no reconstruction from one supplying the full provider inputs, and retain refusal of incompatible providers. This does not require adding 3D buffers to the sprite path or changing the human's scope. It requires one consistent shared interface before dependent work begins. I left those files unchanged, as this review is authorised to write only its report and captures.

**Suggestion — distinguish the two coverage error measurements. Verified wording mismatch; no GPU failure claimed.**

M3.1 records the largest per-pixel scene-linear colour error on random texels; M3.2 uses the same number to bound the change in an individual texel's total weight over the screen. Those are different measurements. The design's exact box-filter argument is sound for axis-aligned magnification, but a measured per-pixel error is not automatically a bound on accumulated weight error. State the normalization or derivation connecting them, or retain the weight measurement separately. This sharpens the proof without changing the selected sampler. Locations: [DONE.json](../../plans/RENDER-2D/DONE.json), lines 289–304, and [the planning probe](planning-probe.md#1-does-coverage-sampling-keep-texels-the-same-size-while-the-view-scrolls).

**Suggestion — qualify the unextended graph's pass list. Verified wording mismatch.**

M1.2 says the sprite and output passes are the entire pass list, while M1.3 explicitly accepts and runs upload and simulate contributions. Mechanism 1 also lists optional upload but omits simulate from its enumeration. Name the M1.2 fixture as the scene without contributions and include the admitted stages when describing an extended scene. The intended mechanism is already clear from M1.3 and [EFFECT-GRAPH's stages](../../plans/EFFECT-GRAPH/DESIGN.md#effects-and-stages-m2-m3); this needs clarification, not another extension mechanism.

**Request coverage and proof review.**

I read the sole planner task in [ACCEPTED.json](../../plans/RENDER-2D/RECORD.json) first: “Plan module RENDER-2D. It needs: SHOWCASE, APPLE-TILE, CONTENT-LOOP”. It supplies dependency context rather than detailed human requirements. I used [BRIEF.md](../../plans/BRIEF.md), especially decision 7, and [the owner's next-goal page](../../design/NEXT-GOAL-SWARM-GAME.md) as the request authority before comparing the design. They require reusable 2D support alongside 3D, demonstrated by an original swarm survivor with networked play. The top-down sprite style, thousands of bodies and three targets are recorded defaults. Networking and the finished game's rules/art belong to other modules.

Within that division, the design covers the required sprite scene, layered crowds, tick-driven animation, interpolation, colour/effects, source art and hot reload, lifecycle, and reuse by another game. The omitted lights, tile-map tools, skeletal animation, rotating camera and mixed 2D/3D scenes are explicit assumptions in PLAN.md rather than hidden reinterpretations of “2D support”. The eight-sheet limit and linear-light blending are also disclosed. I found no unrelated feature expansion that should hold up the build.

I read all **58 done-when items**, including their checks, questions, inspections and milestone bars. These are assessments of whether future proof is possible, not pass verdicts for unbuilt work:

| Items reviewed | Assessment at their milestone boundary |
|---|---|
| M1.1–M1.11 | Embedded frame rows let the first milestone reach Mac and Chrome without a cooked sheet or SIM-2D. Source-derived pixel oracles, resource/pipeline accounting with a 3D control, lifecycle comparisons and baseline verification reject a flag that merely hides the 3D picture. Resolve finding 2 and clarify the contribution fixture. Cost records correctly avoid numeric speed gates. |
| M2.1–M2.12 | V1's existing simulation supplies the world rows. Pixel positions at intermediate alphas, identity/generation/timeline discontinuities, overlap readback, mixed blending and upload/draw counters are judgeable and distinguish genuine simulation extraction from a static sprite demo. Finding 1 prevents the ordering promise from being met as designed. Real-time inspections are separate from those correctness checks. |
| M3.1–M3.11 | Embedded sheets suffice for filtering, mips, gutters and tiles. Independent area references and the nearest-sampling negative control reject aligned stills masquerading as scrolling proof. The coverage-metric suggestion makes the retained bound easier to interpret. Density, resize, fractional motion and smooth-art inspections are possible without M4. |
| M4.1–M4.11 | CONTENT-LOOP M3 supplies the lease/publication boundary. The planned array-sheet extension is explicitly assigned to this builder. Pixel-and-packing edits, withheld submission, discarded switches, name handles, refusal paths and device replacement can establish atomic replacement and last-good retention. The claim of “one frame” is read as one atomic visible switch after cooking/upload readiness, consistent with the specified budgeted lease. |
| M5.1–M5.13 | SIM-2D M1 supplies the Crowd trace, churn and retained digests; its M2 Mixed trace is not required. Public-interface use, digest preservation, full crowd counts and separate costs/visuals can reject game-specific renderer logic and a thinned crowd. The simulator proves function only. M5.3 remains contradicted by finding 1. The guide and maturity questions are judgeable at close. |

I checked the declared dependency chains: SHOWCASE M8 follows its M7 dependency on APPLE-PRESENTATION M3, and the other explicit frame/content/effect dependencies supply the boundaries the milestones need. I found no required later milestone that makes the planned proof order circular. GAME-UI's interface stage and font layout remain its work; this module supplies the reusable sprite-pass inputs.

The shared graph, submission lifetime, content publication and tick ownership agree in direction with C01/C02/C04/C05/C06, EFFECT-GRAPH and CONTENT-LOOP. I followed the normal frame, sheet replacement and device-replacement paths through their design boundaries. The selected premultiplied blend and coverage-sampling families have support in the primary [blend explanation](https://shawnhargreaves.com/blog/premultiplied-alpha.html) and [sampling implementation](https://gist.github.com/d7samurai/9f17966ba6130a75d1bfb0f1894ed377). Eight sampled sheet textures fit the published [WebGPU texture-binding limit](https://www.w3.org/TR/webgpu/#limits); that alone does not qualify a complete future pipeline layout or its device cost.

This review also read the project review guidance and graded examples, the planning probe, and the relevant frame/publication/capture contracts. It did not build the future renderer, inspect a running sprite scene, watch motion, reproduce the planner's timings, or qualify hardware. The numerical ordering reproduction above is the independent experiment. Panel cadence and EDR appearance are explicitly owed rather than inferred from engine capture, which is appropriate here. No milestone verdict or planner field was changed.

Validation: `CARGO_TARGET_DIR=/private/tmp/thrum-render2d-design-review-target ./dev check-docs` passed: 773 Markdown files, 4,924 local links, 30 module records, crate policy and current design hashes. `git diff --check` was clean; repository status showed only this new review file.

DESIGN CHANGES: 2 must_fix findings
