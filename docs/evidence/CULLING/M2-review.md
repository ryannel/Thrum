# CULLING M2 review

**PASS — all five items pass. No Must Fix or new Suggestion.**

Reviewed 13 November 2026 at `e6f7a54a3675d22179f0e276838778cdff129480`, builder boundary
`0146f8d2fbcf`. Applied review-milestone and Thrum engineering review guidance. No delegation,
production edits or commits. This replaces the previous review.

## Basis and independence

I judged the item statements, changed implementation, proof commands and raw measurement rows before reading
`M2-review.md`, the builder's evidence narrative and log, and `ACCEPTED.json`. Existing verdict/finding metadata
in DONE and repair descriptions in PLAN were visible while reading the required planning files; they were not
used as the acceptance criteria.

The supplied clean-checkout qualification at `081d74e86a66` establishes `./dev check` and the four named
checks, M2.1, M2.2, M2.3 and M2.5. I accept those results as the handoff and skill permit; I found no reason
to repeat their device runs. This review adds source inspection and independent recomputation of retained
simulator pixels, mutation identity and benchmark source identity. It does not claim a fresh reviewer GPU run.

## Item verdicts

| Item | Verdict | Judgment |
| --- | --- | --- |
| M2.1 | pass | Selection consumes the immutable capability record before pipeline creation. Missing indirect execution and unknown facts prepare actual Direct, with the reason reported. The automated check exercises supported, masked and unknown records for all requests, compares fallback images with Direct and asserts actual prepared modes. Existing equality witnesses reject silent fallback. |
| M2.2 | pass | The actual simulator host requests Occlusion and resolves Direct with `MissingIndirectExecution`. The named check now compares every pixel with an independently prepared explicit-Direct host using the same initial game input, and checks eight progressing presentations. Its retained omission control fails the comparison while preserving both the capability reason and presentation progress. |
| M2.3 | pass | The current-source browser build runs the mixed-mesh field, captures the 16 prescribed views and compares Frustum and Occlusion against bucket-ordered Direct without tolerance. It checks matching tick/view/extent, actual resolved modes, active/drawn counts and fewer Occlusion draws. The supplied clean-checkout pass establishes the target result. |
| M2.4 | pass | Both retained 1920×1080 runs support the mixed-field Occlusion default in cover and open groups. All three modes are paired, timed scene passes include tone and occlusion work, load is recorded, and the benchmark requires nonempty prepasses and actual hiding. Native and browser defaults use the same selector. |
| M2.5 | pass | The verifier rebuilds native and browser artifacts, runs both registered native matrices and all four Chrome digest sets, and binds success to unchanged source content. No baseline or pixel tolerance was changed by M2. The supplied clean-checkout verifier pass establishes the claimed integrated result. |

### M2.1: request, capability and prepared draw boundary

`selected_setup` and browser `create_field_game` carry the request through `GameSetup` into the retained
scene. `PreparedSynthetic::prepare_internal` calls `CullingSelection::resolve` in
[camera_culling.rs](../../../crates/thrum-render/src/synthetic/camera_culling.rs#L55).
Besides the requested mode itself, only `DeviceCapabilities::downlevel()` influences this selection.
Explicit Direct remains Direct; absent indirect or compute support and unknown facts resolve conservatively.
The capability producer captures adapter facts once; recording does not probe the device.

The resolved mode selects the view layout, vertex entry, visibility resources and depth-prepass pipelines.
`draw_camera` uses the complete active instance range when visibility resources are absent; the mesh route
likewise receives no indirect arguments for Direct. This is a real prepared fallback, not a report-only substitution.
The original scene request is retained: device replacement calls common preparation with that scene and the new
capability record. Selection reaches runtime frame samples, native app summaries, browser reports and simulator
state. I read both producer and consumer sides of these boundaries.

The cube oracle helpers and mesh oracle helper assert actual prepared mode. The mesh helper compares with its
own preparation request because Direct and culled renderers intentionally share frame inputs. Bucket and field
witnesses also assert resolved modes. No equality tolerance was relaxed by that correction.

### M2.2: simulator completeness and previous Must Fix

**Resolved: `simulator_complete_scene_unchecked`.** The former proof only checked presentation progress and
reported strings. [The current oracle](../../../examples/v1-game-ios/src/ios/culling.rs) captures the very host
which becomes the presenting app host, plus a separate explicit-Direct host. Both use the ordinary diagnostic
10,000-instance setup, the same device, extent, initial committed tick and capture alpha. It refuses incompatible
setup and attached cooked content. The capture path extracts the ordinary world publication and draws through
the prepared renderer; it does not synthesize reference pixels.

[The check](../../../scripts/culling_sim.py#L21) validates readback lengths, both selection records, tick zero,
expected population, a nonuniform Direct image and zero differing pixels. The ordinary draw source still uses
`0..self.instance_upload.active_count()`; the half-draw mutation is absent from production.

I independently decompressed and compared the retained positive and negative RGBA artifacts. The
[audit](M2-review/current-artifact-audit.json) records:

- Positive: 12,648,528 bytes per image at 1206×2622; **zero differing pixels**; presentations 120–960.
- Negative: the same extent and Direct reference hash; **1,158,221 differing pixels**; eight progressing
  presentation samples, 120–956. The retained check reports failure.
- The [mutation patch](M2/simulator-completeness/omission.patch) halves the actual fallback draw while leaving
  explicit Direct complete. Its hash matches the negative report after the same whitespace stripping used by
  the script's command helper. The positive and negative binary identities are retained in the audit.

These checks close the prior proof defect on its named simulator target. This is function and same-input image
completeness, not physical-device speed or a new motion-quality claim.

### M2.3: Chrome oracle boundary

`field-mixed-oracle` supplies `field-meshes` to all three pages. Browser setup passes that flag into the field
scene; all three modes therefore use the mesh bucket route. The checker captures eight headings at two pitches,
keeps the world paused, compares all RGBA bytes and rejects a mode/count fallback. Retained qualification has
20,789 Frustum draws versus 1,415 Occlusion draws out of 100,513 active slots and no unequal view.
The actual instantiated Wasm receipt is compared with the freshly built disk artifact, avoiding a stale-bundle
pass. I read the shared receipt implementation and its call sites, including the replacement for the oversized
DevTools response transfer. Its assertions remain fail-closed for missing or mismatched receipts.

### M2.4: measured default

[Run 1 excerpt](M2-review/current-bench-1-rows.txt) and
[run 2 excerpt](M2-review/current-bench-2-rows.txt) are copied from the retained raw logs, not new measurements.
Both have 1,800 measured ticks after 60 warm-up ticks, 920 cover and 880 open samples, 1,800 nonempty prepasses
and 1,065 hiding ticks. Order rotates among the modes each tick. Cover means Occlusion keeps at most half
Frustum's list; the camera pitch changes halfway through to expose open views. Both groups are required.

| Run | Cover paired O−F median | Open paired O−F median | Open paired p95 |
| --- | --- | --- | --- |
| 1 | −7.827 ms | −0.239 ms | +0.023 ms |
| 2 | −7.822 ms | −0.256 ms | −0.000 ms |

Occlusion's whole-GPU p50 and p95 also beat Direct in both groups of both runs. The small open median win
satisfies the design's rule; the positive run-1 open p95 prevents claiming that every sample wins. Segment
loads and `contended=true` are explicit. Whole GPU here is the sum of timed scene passes, not physical
presentation or untimed transfer/query overhead. No full-refresh claim follows from these rows.

The independent audit verifies that the current benchmark source matches the measurement identity. The only
difference in renderer/game source since measured `7dca0b33` is the mesh test-helper assertion, not production
rendering. Thus these rows remain applicable. The previous suggestion about stale maturity timing medians is
also resolved: the ledger now uses the retained values above.

### M2.5: exact baseline proof

The registry names the native APPLE-TILE baseline and day/night SDR/scene-linear Chrome sets. The verifier
requires a clean checkout, rebuilds artifacts, compares against retained references and publishes proof only
after every comparison succeeds and content identity remains unchanged. The retained builder run reports
38 + 4 native comparisons and four Chrome sets of 481 records. The later supplied clean-checkout pass is the
basis for accepting the integrated source; an old proof alone would not establish it.

## Scope and limits

Read the M2 implementation and follow-up changes: capability selection and reporting; native, browser and iOS
host consumers; default selection; benchmark; equality-helper assertions; browser and simulator checkers; Wasm
receipt repair; simulator oracle and omission patch; plan, evidence and maturity updates. Builder boundary
`git show 0146f8d2fbcf` itself only shortens LOG; the substantive changes are its ancestors. Unrelated concurrent
modules are not represented as CULLING implementation. No canned render result, weakened comparison, omitted
production draw or test-only substitute was found.

M2 has **no `inspect` proof**, so no visual inspection list or new screenshots are required for its items.
I did not inspect stills or motion in this round and make no new visual-quality claim. Accepted automated
coverage includes unknown/masked capabilities, real simulator fallback and presentation progress, Chrome's
16 changed views and device-loss recovery, and registered day/night native/browser baselines. I did not rerun
rotation, simulator background/foreground, storm, low-sun motion or active-panel full-refresh checks. Those
broader target conditions are not inferred from this bounded capability/equality review; valley, temporal
culling and explicit motion inspection remain M3 work.

Set all M2 item verdicts to `pass` and M2 state to `done`. Builder status, counts and other milestones are
unchanged; the script owns clearing the prior findings on success. Existing older files in the review evidence
folder remain historical; only the `current-*` audit files above belong to this round.

PASS M2
