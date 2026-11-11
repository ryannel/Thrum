# ATMOSPHERE M5 independent review

Reviewed commit `783502e58ee6ed24161f812c249cfd7e19e872da` on the M3 Max, using native Metal and hardware Chrome WebGPU. All ten items pass. No Must Fix finding, including outside the items. One verified Suggestion is recorded below. This closes M5 only; it does not qualify M6's landscape/shadow polish or physical display cadence.

## Findings

**Suggestion — verified: check shader substitution cardinality.** In `crates/thrum-render/src/synthetic/scene_source.rs:6` (`compose_with_cloud`) and `crates/thrum-asset-gpu/src/draw.rs:790` (`cooked_source`), cloud composition uses unchecked string replacement. Read the cloud-enabled composition branches: a future edit to the matched shader text could silently leave the original direct-light expression in place. Prefer an exactly-once assertion at composition. At this commit each needle occurs once and the production GPU checks exercise both paths successfully; this breaks no M5 item and is not a Must Fix.

## Method and scope

Read the brief, module design/plan, C08 contract, review calibration, M5 implementation changes and affected producer/consumer boundaries before consulting the builder's claim and log. The [source inspection record](M5-review/code-inspection.md) answers the reviewer questions. The M5 implementation comparison includes commits `5b319029`, `53c783d3`, `3cfcb4ae` and browser bootstrap `8b431781`; no implementation, referee reference, bound or case was edited in this review. No prior M5 review or configured image anchors were present.

Built and ran this checkout. [The full check log](M5-review/dev-check-metal.log) records the successful `./dev check`. The initial sandboxed attempt could not obtain a Metal adapter; rerunning with GPU access passed. The full check supplies correctness evidence, not an exclusive-load timing claim. All subsequent target checks, paired cost runs and visual captures used the shared GPU lock. [Command results](M5-review/commands.json) and [capture index](M5-review/visual-index.md) provide commands, actual submitted times, selected original frames and sampled sequences.

The first native capture attempts correctly rejected my 1280×720 browser route against the native harness's prepared 1920×1080 extent. Native `r2` captures use the correct extent. Three initial Chrome fixed selections incorrectly treated simulation tick as elapsed capture time despite the initial one-second hold; their corrected `r2` captures replace those selections. Neither harness mistake is product evidence. Accepted source frames were hash-verified before retaining selected PNGs and removing owned raw spools. Contact strips and preview movies are derived at 10 Hz; this review inspected the strips and original frames/crops, not full-rate video playback.

## Inspection list, written before viewing

The [original inspection plan](M5-review/inspection-plan.md) specified noon and low-Sun off/on slow pans, native Metal realtime and Chrome fixed/realtime, checking:

1. Terrain-to-cooked-mesh patch-edge alignment, including synthetic instances.
2. Shaded ground beside sunlit ground: retained sky light, no black patch.
3. Cloud shade crossing existing ridge shadow: no second darkening of geometric umbra.
4. A world-space landmark over two seconds: drift consistent with the declared `[160,40]` m/s and receiver heights.
5. Cross-target shade placement and depth at matched route times.

The run includes noon (12 h), low Sun (17.3 h), distant ridges, the west-facing ridge-shadow view, moving camera approaches and the route's faster turn. Strong diagnostic attenuation and the zero/above-plane/outside-support cases are also covered by the GPU tests. Actual weather is outside this diagnostic milestone. Full-refresh physical presentation is not inferred from engine capture or browser RAF intervals.

## Item verdicts

| Item | Verdict | Evidence and assessment |
|---|---|---|
| M5.1 | pass | `./dev check`; the single environment identity changes with profile and actual device/resource replacement, not time scrub. Raw Sun remains continuous through sunset while surface illumination fades. `EnvironmentState`/`AtmosphereFrame` produce the identity; contributions read it. See source inspection. |
| M5.2 | pass | `./dev check`; independent contribution pipelines use the lent layout/library. H1–H5 assert their stated references and bounds, including non-unit exposure. H2/H4 use separate f64 filtered decodes on live and ≥32-step known fields, with `C/32 + 1e-5 M + Q`; named wrong lookups fail. H5 varies direction, roughness, raw lux, night and rewritten sky. P1–P11 references/cases/bounds are unchanged. The [additional Metal precision probe](M5-review/target-check-5.log) also passes. |
| M5.3 | pass | [Chrome environment command](M5-review/target-check-0.log), [row results](M5-review/chrome-environment.json.gz), [hardware record](M5-review/chrome-browser-gpu.json.gz). H1–H5 resolve and pass locally enforced double-precision references; H2/H4 negative lookups are rejected. No Metal output supplies expected values. |
| M5.4 | pass | [Native parity command](M5-review/target-check-1.log) and [results](M5-review/metal-parity.json.gz): recorded base/current, one cooked store, full byte comparison, same-build control and failing negative control. Source inspection confirms unchanged no-provider shader text and no extra off-path binding, texture, pass or per-frame write. |
| M5.5 | pass | [Chrome parity command](M5-review/target-check-2.log) and [results](M5-review/chrome-parity.json.gz): base/current and same-build stills match exactly in one browser process; changed-camera negative control is rejected. |
| M5.6 | pass | `./dev check`; shared `cloud_factor` applies `exp(-tau)` to direct Sun only. Terrain/instances/meshes use the shared shade function; each cooked-volume march sample uses its transformed world point. Tests isolate sky/emission and cover ln 2, spatial variation, zero, above-plane and outside-region. Provider presence/extent changes require preparation and history reconstruction; ordinary moving fields do not reset history every frame. |
| M5.7 | pass | Fresh 1920×1080 native realtime captures, whole frames, original-size mesh/ground crops and 10 Hz sequence inspection. All five inspection entries pass; observations below and linked capture index. |
| M5.8 | pass | Fresh 1280×720 Chrome fixed and realtime captures of the same views. Placement and depth agree with Metal after accounting for extent and nearest realtime sample time; no receiver-specific disagreement observed. All five entries pass. |
| M5.9 | pass | [Native cost command](M5-review/target-check-3.log), [full record](M5-review/metal-cost.json.gz), [summary](M5-review/cost-summary.json). Fresh OFF/ON/ON/OFF order, build/load metadata and GPU/whole-frame observations recorded. All forward texture counts fit the capability limit. |
| M5.10 | pass | [Chrome cost command](M5-review/target-check-4.log), [full record](M5-review/chrome-cost.json.gz), [summary](M5-review/cost-summary.json). Fresh paired order; every synthetic/mesh/volume pipeline created. Counts fit the browser capability limit; RAF observations are labelled without a physical cadence claim. |

## Visual assessment

At noon the moving diagnostic bands are plainly visible across the distant hills. At the foreground rock, the direct-lit top and adjacent ground dim together between the 1.25 s and 3.25 s views; the dark blue side and cast shadow retain ambient illumination. The sphere's reflected sky remains legible. The transition follows world-space receivers rather than a screen-space overlay, with no discontinuity at the mesh/ground boundary. The 10 Hz sequences show coherent displacement through the camera approach; the route's later camera turn is an authored view change, not a cloud pop.

At low Sun the bands stretch across the lit slopes. Direct-lit rock faces change less strongly than the noon view but visibly, while the blue faces and long geometric shadow stay lit by the sky. In the west view, comparing off/on at 2 s and 4 s shows cloud bands on the sunlit slope below the broad ridge umbra; the umbra itself gains no second dark band. The same ground features show advancing bands over those two seconds, consistent with the declared field displacement and projection. This is a visual consistency check; the exact world-point factors and support rules are established by the independent numerical checks.

Chrome fixed and realtime views reproduce these relationships and the native shade placement/depth. The indexed source frames preserve target resolution; comparisons do not assert pixel equality between different extents. Existing faceted mesh shading, low-Sun terrain steps and the authored route transition are visible with the provider off as well. M5's written bar excludes diagnostic shape/softness, and M6 separately owns the broader sky/shadow polish. These observations do not turn those existing limitations into a new M5 regression.

## Cost observations

Fresh native medians in OFF/ON/ON/OFF order were forward GPU **1.1485 / 1.1922 / 1.1942 / 1.1488 ms**, whole GPU including present **3.8134 / 3.9561 / 3.9557 / 3.8241 ms**, and CPU wall **0.6331 / 0.6443 / 0.6535 / 0.6548 ms** (480/480/479/479 samples). These are capture-on observations, not a budget gate. Chrome RAF interval medians were **16.7 ms** for each arm (241 samples each), not isolated GPU duration or display timing.

Provider-on sampled-texture counts were **12 synthetic, 12 cooked mesh, 9 cooked volume**, against **16** on both targets; the corresponding off counts were 11/11/8. Full records retain build, order, load and capability context.

Final [documentation checks](M5-review/check-docs.log) passed. Review/index links resolve. A structural comparison against HEAD confirms that DONE.json changes only M5 state and item verdict/review fields; counts, builder status, planner fields and M6 remain unchanged.

PASS M5
