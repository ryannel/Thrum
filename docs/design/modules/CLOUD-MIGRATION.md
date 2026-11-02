# Cloud and atmospherics migration build plan

2 November 2026. **Selected production-derived baseline; construction not started.**
[The production baseline](CLOUD-PRODUCTION-BASELINE.md) locks authoring, data, renderer, light and consumer path.
[CLOUDS](CLOUDS.md) owns scope/contracts; [capabilities](CLOUD-CAPABILITIES.md) own actual outcomes;
[POC DoD](CLOUD-PROOF-ACCEPTANCE.md) and [CPU/GPU/memory discipline](CLOUD-PERFORMANCE.md) govern evidence.
This is the only active checkbox queue. Earlier research-led candidate comparisons are historical.

The default is **Houdini22 -> authored VDB optical animation -> ordinary sparse volume playback/LOD -> Hillaire air +
Beer shadows + one cloud MS octave -> joint RGB media -> current environment and independent reflections -> per-view
reconstruction**. No custom lifecycle solver, far-transfer codec, fitted light bank, runtime wind warp or optical-flow
reconstruction must succeed to build this baseline. This is an assembly of established methods, not a claim one
studio ships our exact implementation. Local art and device measurements remain unrun.

## Progress and completion semantics

A checked item links actual evidence for its stated scope. Each P stage records implementation, contract/numerical,
visible quality, personally observed motion, coupled consumers, cost-risk and actual-device admission separately.
Partial helper/test passes never green the whole stage. Movie existence/contact sheets do not prove watched motion.
Unavailable hardware stays unqualified. Freeze each stage's proof card, cue/reference/negative control and quantitative
thresholds before inspecting its candidate. Every POC has a launch command/URL, pinned scenario, build/source/time
readouts, usable controls, native beauty and its diagnostic views.

| Stage | Dependencies | Actual status |
| --- | --- | --- |
| D0/D1 | Original commission | Historical research/first review retained; no implementation pass |
| D1R2 | D1 | [Second review](../../evidence/atmospherics-migration-round2-2026-11-02/REVIEW.md) retained; previous experimental gate choices superseded |
| D2 Production baseline | Current user direction | [Independently reviewed](../../evidence/cloud-production-baseline-2026-11-02/REVIEW.md); written specification selected, construction unrun |
| P0 Thin observable bench | D2 | Old proof preserved in a124167; new harness unbuilt |
| P1a Houdini source pilot | P0 + licensed Houdini prerequisite | Unbuilt; tool chosen, installation/license not yet verified |
| P2a Air reference | P0 | Unbuilt |
| P3a Same-source consumers and ordinary LOD | P1a/P2a minimums | Unbuilt |
| P4a Small spatial coupled/cost sentinel | P1a/P2a/P3a minimums | Unbuilt; before library expansion |
| P1b Complete representative source recipes | P4a | Unbuilt |
| P2b Complete four-LUT air | P4a | Unbuilt |
| P3b Baseline production sampling/light | P4a + P1b representative source conformance before format/library freeze | Unbuilt |
| P4b Baseline reconstruction/full overlap | P1b/P2b/P3b | Unbuilt |
| P5 Five isolated families/variation | P4b | Unbuilt |
| P6 Authored weather/combined layers | P5 | Unbuilt |
| P7 Complete standalone skies/device fixture | P6, costs screened throughout | Unbuilt |
| P8 Thrum integration/product admission | Accepted proof + engine I1/I2/I3 dependencies | Unbuilt; no engine runtime yet |

Normal parameter tuning within the chosen architecture is expected. It may adjust source cadence/resolution/material
coefficients, page/mip/step counts, shadow resolution and temporal weights while retaining the witnessed cues. POCs
verify a concrete implementation; they do not defer these foundations to a later research competition. Architecture
changes need contradictory evidence and an explicit revision. No paid tools/services are purchased by this plan.

## Implementation boundaries and integration order

Build the standalone proof under `tmp/cloud-production-proof/`; do not restructure the old instrument into a new
engine. These are internal units, not speculative crates or public SDK promises. Promote selected source/evidence
with exact pins at checkpoints; engine packages and permitted dependencies are registered only at P8.

| Unit / first stage | Owns | Reads / prohibited coupling |
| --- | --- | --- |
| `authoring/`, P1a | Houdini scene/recipes, shared episode drivers, VDB/pose/ID/AO export and Karma reference recipe | World story/metre/second/wind inputs. No runtime camera or per-consumer sky assets. |
| `cook/`, P1a | Offline optical calibration, guarded bricks/mips/occupancy, manifest/physical-byte/provenance validation | Exported fields/tracks. Does not become weather authority or renderer. |
| `src/episode`, P0 | Immutable episode facts and sampled world clock/poses/IDs/cause stubs | Admitted manifest. No GPU, private clock, view culling or numerical wind integration. Future E1/W1. |
| `src/source`, P1a/P3a | Pure query contract, frame-pair keys, support/index, bounded IO/decode/admission/lease requests | Episode snapshot and cooked source. No lighting, history or view ownership. Future F4/V3 boundary separates lease and sample roles. |
| `src/gpu`, P0 onward | Resources, pipelines, upload/submission/fences/epochs, one ordered queue and physical inventory | Demands/leases; no source facts or UI policy. Future V1. |
| `shaders/source`, P1a/P3a | One coefficient/occupancy/pose query used by all optical passes | No per-view cloud recipe, duplicate wind or independent source texture. |
| `shaders/air` P2a; `shaders/light` minimal P3a/P4a, complete P3b | Air LUTs/segments, BSM/direct Sun queries, phase/MS/ambient source, generation keys | Common coefficient query plus Sun/air/fog/geometry. No camera-final tint, environment feedback or hidden optical readback. Future V3. |
| `shaders/transport`, P3a/P4a | Sorted shared media, disjoint interval transport for main/reflected/environment rays | Coefficients/light/terminals and view demands. No separate completed fog overlay. Future V2/V3. |
| `src/views` requests P0/P3a; `shaders/resolve` P4b | Independent projection/prior samples, reactive/fresh masks, temporal/spatial resolve and display adapter | Optical S/T/guides. Cannot alter source or supply a main-view reflection. Future V2/V4. |
| `src/bench` and proof cards, P0 onward | Scenarios/controls, inspector/capture and nonblocking counters | Calls narrow units; does not own clock, source, GPU lifetime or light formulas. Future T1/T2. |

Implement one source query first and add readers without copying its math. Keep the orchestration file to dependency
ordering and publication; numerical algorithms, source decode, controls and resource ownership stay in their units.
CPU references and GPU tests compare the same declared semantics independently. A changed source/lease/light/view
contract updates every affected producer/consumer and its proof card before a dependent slice continues. P8 maps these
roles onto the actual engine API; no prototype coordinator, browser loader or private final resolver is promoted wholesale.

## D2 — lock and review the production baseline

- [x] Choose Houdini FX22.0.459 + sparse Pyro/SOP/VDB/Karma/cooker; declare actual licensed-install prerequisite.
- [x] Choose baked episode wind/poses, source frame playback/streaming, all-five3D optical output and material convention.
- [x] Choose the complete air/cloud/shadow/ambient/environment/reflection/reconstruction path and starting defaults.
- [x] Move algorithm challengers into the optional register; retain tests/references without making them prerequisites.
- [x] [Independent authoring, rendering and whole-system review](../../evidence/cloud-production-baseline-2026-11-02/REVIEW.md); resolve findings and verify consequential fixes.
- [x] Align canonical scope/DoD/performance/source records; docs/hash/whitespace checks and commit/push form this planning checkpoint.

## P0 — thin comparison bench

- [ ] Implement only the selected source/light/view seams with an existing tiny comparison source; no general SDK/editor.
- [ ] Load an authored episode/ID/pose manifest and one world clock; play/pause/seek1x/16x/64x, camera/Sun/consumer toggles.
- [ ] Show main, independent mirror, receiver, environment and optical diagnostics with matching generation/time/build.
- [ ] Retain stable Beer/phase/clock/oracle cases, late-result/epoch/idle/all-off/main-hidden and Mac trackpad checks.
- [ ] Bounded jobs/pages/fences, nonblocking counter/timestamp ring, allocation/GC and shader-demand/first-frame records.
- [ ] Pin the small family/motion/air/light reference board. Match source/camera/exposure in comparisons and keep a held-out reference.
- [ ] Done: runnable visible bench plus scoped correctness/observer/control receipts, not family or budget acceptance.

## P1a — selected Houdini source pilot

- [ ] Verify chosen Houdini production installation/license and tool pin; no tool-selection experiment or substitute solver.
- [ ] Create one recoverable Cu clip with actual local growth, continuing mature renewal and breakup using the locked recipe.
- [ ] Export baked shared-wind density, event-local poses/IDs/bounds and optical channels; runtime applies wind zero additional times.
- [ ] Run the selected RG16F brick/mip/frame-window cook with accessibility and optional motion field; compare24Hz source
  to4Hz initial runtime frames and intermediate times. Increase cadence/detail only where required cues fail.
- [ ] Review fixed/world-locked views, tagged local birth/renewal/loss, native beauty and actual1x/16x/64x playback.
- [ ] Record scratch/durable/master bytes before larger caching; first pilot peak<=2GiB. Stage regenerable recipe and package.
- [ ] Done: one conventional authored source works and is visually accepted for the pilot scope, with real IO/decode/peak receipts.

## P2a/P3a — minimum air and same-source consumers

- [ ] P2a: bounded RGB clear-air/Sun/reference segment with visible sky/neutral-depth response, independent optical oracle.
- [ ] P3a: selected optical frames feed main/mirror/receiver/environment from one source time and pose; main-off retains causes.
- [ ] Verify ordinary support/occupancy and footprint mips against resolved source, thin gaps, horizon, low Sun and offscreen blockers.
- [ ] Stress1/2/16/64/regional groups, shared density products/authored independent birth times, sparse/full cover and1x/16x/64x; direct index traversal is oracle.
- [ ] Done: source/consumer/domain integrity and measured cost slopes. No special far summary or competing field architecture.

## P4a — early spatial coupled sentinel

- [ ] One accepted Cu + clear air + one fog + ground/interface mirror, shared Sun/BSM/ambient/environment and independent reflected ray.
- [ ] Move shadow through air/fog/terrain/reflection. Compare direct/ambient/optical panes and same-point/time receipts.
- [ ] Observe native1x/accelerated lifecycle and consumer motion; reject stale shadow, double fog/air, static life or main-history reflection.
- [ ] Fill a combined source/IO/CPU/light/main/reflected/env/air/memory ledger using actual pages and early stress counts.
- [ ] Done: passed pilot image/motion/coupling with quantified cost-risk outcome. Missing M2 is unqualified, structural overrun triggers correction.

## P1b — representative authoring recipes, same chosen format

- [ ] A continuous same-ID Cu-to-Cb episode develops towers, cap/outflow/anvil and unequal decay from its authored inception.
- [ ] A connected Sc/St pilot closes and opens real holes/edges; a Ci pilot emits/shears and loses local strands/tails.
- [ ] All export3D optical frames through the same selected source query; no required runtime column/filament/cohort solver.
- [ ] Before P3b format freeze, include dense Sc/St at the admitted extent/pitch, thin Ci and tall mixed-phase Cb adverse
  frames in physical page/IO/peak estimates. Change a second cloud's position above unchanged Cu to verify episode AO
  changes while identical density remains shareable; no unrelated accessibility reuse.
- [ ] One short authored weather trace proves survivor IDs and baked height-wind coherence in source, shadow and reflection.
- [ ] Done: representative morphology/cook conformance and coupled cost before making the full variant library. No live branch/shear editor gate.

## P2b/P3b — implement the specified production path

- [ ] Four air LUTs plus separate RGB aerial T, spherical terminals and stable segment fallback; surface/ambient/reflected air share inputs.
- [ ] Selected sparse frame-window IO/pages/ordinary mips/occupancy, world-cell and screen/light tile lists; current-time fallback quality floor.
- [ ] Selected cascaded BSM full-column receiver path and complete interior secondary Sun rays, one MS octave, baked accessibility/clear-sky ambient; material/front-back/thin/thick cases.
- [ ] Current six-face32² environment -> SH9/roughness, direct Sun excluded from lighting cube; no recursive environment or double solar energy.
- [ ] Bound production-like rough-reflection/depth/transparency extra work before freezing final quality; real water remains P8.
- [ ] Fill scenario steady/burst and old/new/IO/compile/allocation ledger; compare numerical/spatial references and inspect actual images.
- [ ] Done: chosen algorithms implemented with admitted settings and same-source correctness. Optional experiments do not block.

## P4b — reconstruction and full overlap

- [ ] Add two fogs/low deck/horizon/interface to accepted sentinel; one RGB joint integral, correct space/Sun/surface terminals.
- [ ] Implement the selected every-frame disjoint far/near pass (near demand includes shadowed clear air) and independent history/spatial fallback, motion/reactive masks and fresh silhouettes.
- [ ] Observe births/holes/low Sun/cuts/seeks16x/64x against spatial reference, including main-hidden and independent reflected camera.
- [ ] Done: no doubled intervals, halos, smeared births or stale history; net combined work/cadence/peak measured at accepted quality.

## P5 — isolated family acceptance and meaningful variation

Each family needs its mechanism pilot then4 distinct review variants (inspection sample, not user-imposed universal library size).

- [ ] Cu: persistent base/body, new billows, mature replacement and ragged breakup.
- [ ] Cb: same cell's towers/anvil and unequal core/outflow decay.
- [ ] Sc: shallow connected cells/rolls, changing gaps, local merge/renewal/loss.
- [ ] St: finite-thickness sheet, revealing edge, local hole closure/opening; no puff ceiling/grey overlay.
- [ ] Ci: thin high fibres, sheared heads/tails and local deposition/loss; no flattened Cu or painted bars.
- [ ] Every variant: native appearance and personally observed normal/accelerated life plus basic light/shadow/reflection/LOD routes.
  One per family takes the full adverse Sun/view matrix. Done: CF packet with cue verdicts and all-retained-consumer cost; unresolved cues stay failed.

## P6 — four authored weather stories and combined layers

- [ ] FAIR/BROKEN/OVERCAST/STORM episode data and CW01–CW04 routes: clear-to-scattered, increasing cover, overcast, developing/approaching storm.
- [ ] Fronts/staggered local births, continuing mature renewal and persistent survivors use baked density/pose and one wind table.
- [ ] Same-condensate parts/adjacent tiles partition once; distinct layers add. Rain/lightning cause stubs keep IDs/time/support with main draw off.
- [ ] Done: observed normal/accelerated local development, consumer correspondence, weather continuity and cost. No global sky crossfade.

## P7 — full standalone skies and fixture admission

- [ ] Freeze representative counts/coverage/routes from CLOUDS; named512-event/etc cases are stress fixtures, not hard product populations.
- [ ] Wide/close/horizon/clear-side storm/deck-edge, repetition/renewal, low Sun, cold/seek/swap/resize/recovery and all-off/retained modes.
- [ ] Same accepted settings in visual and exclusive benchmark runs: actual M2 cloud5ms, total standalone16ms and fixture-browser peak512MiB comparisons.
- [ ] Predeclared p95/60s warm steady plus bursts; CPU/IO/queue/observer/driver scope and deadline misses separate; missing devices/counters unqualified.
- [ ] Done: FS quality/fixture-cost scope only. P8 owns actual Thrum frame, rough-water consumers and full product memory.

## P8 — actual Thrum integration

- [ ] Map the accepted provider to real F2/F4/E1/V1–V4 APIs after I1/I2/I3 dependencies; no competing engine framework.
- [ ] Replay sentinel/mixed weather/adverse sky with real rough water/depth/transparency, independent reflection and shared display.
- [ ] Provider omission/replacement, headless facts, save/replay/network and device recovery preserve source IDs/time; caches rebuild.
- [ ] Qualify actual engine/browser full frame/memory and native3/10ms on actual hosts. Retain recoverable packages; retire duplicate prototype after parity.

## Optional work, explicitly outside this queue

See the baseline's [optional register](CLOUD-PRODUCTION-BASELINE.md#optional-experiments--not-prerequisites).
A proposal must name a measured baseline deficit and projected gain before activation. No challenge implementation,
private-source investigation, scientific solver, live wind/branch response, special far codec or new light fit is a
prerequisite for a required P stage. Proven source/transport references and rejection witnesses remain useful.
