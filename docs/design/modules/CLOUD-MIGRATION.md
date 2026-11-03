# Cloud and atmospherics migration build plan

2 November 2026. **Selected production-derived baseline; P0/P1a construction active.**
[The production baseline](CLOUD-PRODUCTION-BASELINE.md) locks authoring, data, renderer, light and consumer path.
[CLOUDS](CLOUDS.md) owns scope/contracts; [capabilities](CLOUD-CAPABILITIES.md) own actual outcomes;
[POC DoD](CLOUD-PROOF-ACCEPTANCE.md) and [CPU/GPU/memory discipline](CLOUD-PERFORMANCE.md) govern evidence.
This is the only active checkbox queue. Earlier research-led candidate comparisons are historical.

The default is **live E1 weather/wind -> native evolving sparse3D optical source -> Nubis³-style detail/occupancy/voxel
lighting -> Hillaire air + Beer receiver shadows + one cloud MS octave -> joint RGB media -> environment and independent
reflections -> per-view reconstruction**. [Provider selection](CLOUD-PROVIDERS.md) is replaceable; the
[complete atmosphere coverage map](ATMOSPHERE-SYSTEM.md) separates delivered optics/weather from neighbouring effects.
Houdini and complete baked lifecycle playback are removed. The renderer has production lineage; the dynamic source
and exact combined image/cost are unproved. Visual success leads selection; lower cost wins only at equivalent accepted
quality, behaviour and consumer scope. POCs verify this selected model, not an indefinite algorithm competition.

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
| D2 Runtime revision | Current user direction | Native runtime3D default and provider/scope contracts selected; [independent review closed](../../evidence/cloud-dynamic-revision-2026-11-02/REVIEW.md). Previous baked selection/review retained as superseded history |
| P0 Thin observable bench | D2 | [First runtime checkpoint](../../evidence/cloud-runtime-pilot-2026-11-02/README.md): visible bench, causal CPU clock, bounded GPU jobs and finite checks; whole P0 incomplete |
| P1a Live source/control pilot | P0 | Dense native3D Cu growth/renewal/dry-front pilot runs; soft morphology fails visual target; source/motion gate remains open |
| P2a Air reference | P0 | Small GPU/CPU air-transmittance check passes; complete P2a appearance unqualified |
| P3a Same-source consumers and ordinary LOD | P1a/P2a minimums | Main/receiver/independent mirror wired to one source; environment/occupancy/LOD and accuracy unbuilt/unqualified |
| P4a Small spatial coupled/cost sentinel | P1a/P2a/P3a minimums | Unbuilt; before library expansion |
| P1b Cb/Sc/St/Ci live source regimes | P4a | [Billowing/breakup source checkpoint](../../evidence/cloud-billowing-2026-11-02/README.md); finite rolling controls and real1x/16x clips, but regular/soft storm art and complete regimes remain unaccepted |
| P2b Complete four-LUT air | P4a | Unbuilt |
| P3b Baseline production sampling/light | P4a + P1b representative source conformance before format/library freeze | Unbuilt |
| P4b Baseline reconstruction/full overlap | P1b/P2b/P3b | Unbuilt |
| P5 Five isolated families/variation | P4b | Unbuilt |
| P6 Dynamic weather/combined layers | P5 | Unbuilt |
| P7 Complete standalone skies/device fixture | P6, costs screened throughout | Unbuilt |
| P8 Thrum integration/product admission | Accepted proof + engine I1/I2/I3 dependencies | Unbuilt; no engine runtime yet |

Normal parameter tuning within the chosen architecture is expected. It may adjust fixed-step source cadence/resolution/material
coefficients, page/mip/step counts, shadow resolution and temporal weights while retaining the witnessed cues. POCs
verify a concrete implementation; they do not defer these foundations to a later research competition. Architecture
changes need contradictory evidence and an explicit revision. No paid tools/services are purchased by this plan.

## Implementation boundaries and integration order

Build the standalone proof under `tmp/cloud-production-proof/`; do not restructure the old instrument into a new
engine. These are internal units, not speculative crates or public SDK promises. Promote selected source/evidence
with exact pins at checkpoints; engine packages and permitted dependencies are registered only at P8.

| Unit / first stage | Owns | Reads / prohibited coupling |
| --- | --- | --- |
| `recipes/`, P1a | Native deterministic basis generator, genus/control parameters, seed and reference/proof recipes | No paid authoring, lifecycle movie or per-consumer source. |
| `cook/`, P1a | Static basis noise/mips, optical calibration, manifests/hash/physical bytes | Optional imported shapes follow rights/admission. No weather authority or full animated cache. |
| `src/weather`, P0/P1a | Fixed-step weather/front controls, shared W, stable cause IDs and forcing/replay | One E1/W1 authority; no camera/GPU feedback or private clock. |
| `src/source`, P1a/P3a | Provider-neutral query/admission; native q update, support/index, bounded source/checkpoint leases | Weather snapshot/recipe/source state. No lighting, history or view ownership. Future F4/V3 boundary separates lease and sample roles. |
| `src/gpu`, P0 onward | Resources, pipelines, upload/submission/fences/epochs, one ordered queue and physical inventory | Demands/leases; no source facts or UI policy. Future V1. |
| `shaders/source`, P1a/P3a | One coefficient/occupancy/pose query used by all optical passes | No per-view cloud recipe, duplicate wind or independent source texture. |
| `shaders/air` P2a; `shaders/light` minimal P3a/P4a, complete P3b | Air LUTs/segments, BSM/point-voxel-cache/local/direct Sun queries, phase/MS/current ambient source, generation keys | Common coefficient query plus Sun/air/fog/geometry. No camera-final tint, environment feedback or hidden optical readback. Future V3. |
| `shaders/transport`, P3a/P4a | Sorted shared media, disjoint interval transport for main/reflected/environment rays | Coefficients/light/terminals and view demands. No separate completed fog overlay. Future V2/V3. |
| `src/views` requests P0/P3a; `shaders/resolve` P4b | Independent projection/prior samples, reactive/fresh masks, temporal/spatial resolve and display adapter | Optical S/T/guides. Cannot alter source or supply a main-view reflection. Future V2/V4. |
| `src/bench` and proof cards, P0 onward | Scenarios/controls, inspector/capture and nonblocking counters | Calls narrow units; does not own clock, source, GPU lifetime or light formulas. Future T1/T2. |

Implement one source query first and add readers without copying its math. Keep the orchestration file to dependency
ordering and publication; numerical algorithms, source preparation, controls and resource ownership stay in their units.
CPU references and GPU tests compare the same declared semantics independently. A changed source/lease/light/view
contract updates every affected producer/consumer and its proof card before a dependent slice continues. P8 maps these
roles onto the actual engine API; no prototype coordinator, browser loader or private final resolver is promoted wholesale.

## D2 — runtime revision and provider selection

- [x] Remove mandatory Houdini, baked world stories/wind and offline all-cloud ambient; preserve13a19ad as history.
- [x] Select Burning Shores/Nubis³ visual comparator and real3D field/rendering direction; document actual evidence/limits.
- [x] Select native runtime3D source operators/recipes, live weather controls and current voxel light; no physics solver prerequisite.
- [x] Specify replaceable provider roles, full versus shadow-only recipes and whole-atmosphere delivery/owner matrix.
- [x] [Independent source, rendering and whole-system review](../../evidence/cloud-dynamic-revision-2026-11-02/REVIEW.md); verify consequential corrections.
- [x] Align scope/capabilities/DoD/performance/research; documentation/hash/whitespace checks and commit/push form this planning checkpoint.

## P0 — thin comparison bench

[Checkpoint source, actual images, finite checks and scoped review](../../evidence/cloud-runtime-pilot-2026-11-02/README.md)
are retained. All-stage checkboxes remain open where their complete contract is not yet proved.

- [x] Preserve [first running bench checkpoint](../../evidence/cloud-runtime-pilot-2026-11-02/README.md), finite CPU/GPU checks, independent code review and actual growth/front images; complete P0 remains open.
- [x] Preserve [source-work and box-boundary correction checkpoint](../../evidence/cloud-runtime-work-2026-11-02/README.md): exact direct/mask/cached-velocity comparisons, causal reset correction, scoped lifecycle checks and actual Sc before/after images. Full P0/P1a and visual/device gates remain open.
- [ ] Implement only the selected source/light/view seams with an existing tiny comparison source; no general SDK/editor.
- [ ] Load recipe/weather controls and one world clock; play/pause/replay1x/16x/64x, camera/Sun/consumer toggles.
- [ ] Prove full versus independent analytic shadow-only provider configuration and reject incompatible full-sky selection.
- [ ] Show main, independent mirror, receiver, environment and optical diagnostics with matching generation/time/build.
- [ ] Retain stable Beer/phase/clock/oracle cases, late-result/epoch/idle/all-off/main-hidden and Mac trackpad checks.
- [ ] Bounded jobs/pages/fences, nonblocking counter/timestamp ring, allocation/GC and shader-demand/first-frame records.
- [ ] Pin the small family/motion/air/light reference board. Match source/camera/exposure in comparisons and keep a held-out reference.
- [ ] Done: runnable visible bench plus scoped correctness/observer/control receipts, not family or budget acceptance.

## P1a — live native3D source/control pilot

- [x] Retain first dense Cu reference with empty-field birth, transported growth, simultaneous young/old support and spatial dry-front loss; soft morphology and64x throughput fail acceptance.

- [ ] Generate pinned native3D base/erosion/strand ingredients; no paid tool, clip library or bulk asset prerequisite.
- [ ] Implement bounded source advection/local reaction with small independent CPU oracle, halos, source birth/support and immutable snapshot.
- [ ] One Cu undergoes actual local growth, continuing mature replacement and irregular breakup under live source/dry controls.
- [ ] Change wind direction/shear and move a new dry/lift front **after** development starts; existing matter/IDs respond.
- [ ] Compare resolved fixed/world-locked beauty and source regions at normal1x and actual16x/64x; inspect topology and material transport.
- [ ] Count all source/state/scratch/halo/mip/pass/lease bytes, CPU control work and update backlog from the start.
- [ ] Done: observed local birth/renewal/loss and changed-driver response, with recoverable native recipe and actual source-work receipts.
  A static volume globally faded/scaled/retimed or noise-only animation fails. This does not qualify a family library or5ms.

## P2a/P3a — minimum air and same-source consumers

- [ ] P2a: bounded RGB clear-air/Sun/reference segment with visible sky/neutral-depth response, independent optical oracle.
- [ ] P3a: selected current optical snapshot feeds main/mirror/receiver/environment from one source time and pose; main-off retains causes.
- [ ] Verify ordinary support/occupancy and footprint mips against resolved source, thin gaps, horizon, low Sun and offscreen blockers.
- [ ] Stress1/2/16/64/regional groups, shared static ingredients/independently evolving cells, sparse/full cover and1x/16x/64x; direct index traversal is oracle.
- [ ] Done: source/consumer/domain integrity and measured cost slopes. No special far summary or competing field architecture.

## P4a — early spatial coupled sentinel

- [ ] Two live cells/layers (one renewing Cu) + clear air + one fog + ground/interface mirror, shared Sun/BSM/current voxel light/environment and independent reflected ray.
- [ ] While main is hidden, perturb wind/drying and an offscreen blocker; receiver/mirror/ambient and rain/lightning cause stubs keep common IDs/time.
- [ ] Move shadow through air/fog/terrain/reflection. Compare direct/ambient/optical panes and same-point/time receipts.
- [ ] Observe native1x/accelerated lifecycle and consumer motion; reject stale shadow, double fog/air, static life or main-history reflection.
- [ ] Fill a combined source/IO/CPU/light/main/reflected/env/air/memory ledger using actual source voxels/passes and early stress counts.
- [ ] Done: passed pilot image/motion/coupling with quantified cost-risk outcome. Missing M2 is unqualified, structural overrun triggers correction.

## P1b — representative live source regimes before layout/library freeze

Active bounded risk proof: repair the Cb low-body seed, supply-to-cap transport and phase-coherent optical query.
Keep P1a/P4a gates open; this early Cb discrimination does not promote a family or skip coupled acceptance.
[Source, phase/filter/grid reviews, actual images and cost receipts](../../evidence/cloud-storm-source-2026-11-02/README.md)
are retained. [Thermal/asymmetric-cap/local-breakup proof](../../evidence/cloud-billowing-2026-11-02/README.md) now has real1x/16x clips and bounded120s shutdown fragmentation. [Expanding-interior checkpoint](../../evidence/cloud-core-growth-2026-11-02/README.md) corrects the analytical sheet-forming flow and proves native filled density/fed cap. [Bounded material renewal](../../evidence/cloud-material-renewal-2026-11-02/README.md) reduces indefinite texture streaking with actual reset/motion/cost evidence, but art remains soft/stacked and shutdown chiefly fades. [Causal-feed/path proof](../../evidence/cloud-connected-feed-2026-11-03/README.md) closes bounded pickup and cap transport with actual young/optical density, trajectories and90s1x motion, but confirms a smooth shaft. The [front-normal proof](../../evidence/cloud-front-growth-2026-11-03/README.md) passes its operators/work mask but rejects the resulting depleted shelf anatomy and dissolving shutdown. Stop extending profiles2/3/4 as default morphology. Next prove authored3D anatomy through unchanged optics, then complete its local live-development/material/ice/cost contract before lifecycle construction; the720s downwind chart exit requires regional exchange. Record actual1x/16x motion alongside held source images; do not tune
lighting or opacity to hide that failed cue. Regional exchange remains necessary after the long-run chart-edge failure.

- [x] Preserve bounded clear-start Cb cap-arrival/phase/filter pilot and [asymmetric thermal/local-breakup proof](../../evidence/cloud-billowing-2026-11-02/README.md): final GPU/CPU checks, real growth/breakup clips and available-host cost screens. CF02 active billowing art, same-ID Cu→Cb transfer and complete P1b remain open.
- [ ] Replace the failed default morphology producer and prove its complete local lifecycle; retain the [expanding convective-core correction](CLOUD-PRODUCTION-BASELINE.md#convective-interior-correction--under-proof): old moving-frame axial compression is analytically identified; native/raw-material diagnostics, populated thickness, actual merge/split, cap handoff and1x/16x/cost are required before acceptance.
- [x] Preserve bounded dual-rest correspondence proof, exact reset producer/phase checks and real1x/16x/cost evidence; this does not accept lifecycle art or budgets.
- [x] Preserve bounded causal-feed/optical-connectivity/root-to-cap witness proof and long1x failed-appearance evidence; not source-art acceptance.
- [x] Preserve [front-normal stencil/mask proof and rejected1x/16x source anatomy](../../evidence/cloud-front-growth-2026-11-03/README.md); this does not accept the source or budgets.
- [ ] Prove a broad frozen authored3D formation from front/side/oblique, then lock its full weather-driven growth/erosion, survivor/correspondence and tower-fed-ice producer before dependent lifecycle work.
- [ ] Recover optics after a long host/background gap as an explicit new optical epoch, with current-time chart/seed alignment; preserve CPU weather and declare lost q continuity.
- [ ] Same-ID Cu strengthens into Cb towers, cap outflow/anvil and unequal core/ice decay; no inserted/faded disk.
- [x] Correct Ci repeated sine-comb source with [finite filament/head and anisotropic transport proof](../../evidence/cloud-cirrus-2026-11-02/README.md): actual1x/16x images, ice-only GPU regressions and bounded supply-off response; full Ci acceptance remains open.
- [ ] Sc/St connected3D cells/sheet closes/opens actual holes; Ci completes fine high strands/tails and full wind/lighting/region qualification.
- [ ] Dense deck at admitted extent/pitch, thin ice and tall mixed-phase Cb qualify source/halo/material/occupancy bytes and resolution.
- [ ] Two source regions exchange transported support across a boundary without clipping, duplicate material or camera-dependent placement.
- [ ] Same-point/time source, shadow, point-light-cache, ambient and reflection respond to independent runtime forcing.
- [ ] Done: all representative3D regimes and live control/lineage/cost before P3b format freeze or large variation library.

## P2b/P3b — implement the specified production path

- [ ] Four air LUTs plus separate RGB aerial T, spherical terminals and stable segment fallback; surface/ambient/reflected air share inputs.
- [ ] Selected native dynamic sparse field/pages/ordinary mips/occupancy, world-cell and screen/light tile lists; current-time fallback quality floor and conservative growth halo.
- [ ] Selected receiver BSM, current3D voxel Sun tail +2local samples, complete-ray oracle/fallback, one MS octave and runtime ambient accessibility; material/front-back/thin/thick cases.
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

## P6 — runtime weather and combined layers

- [ ] FAIR/BROKEN/OVERCAST/STORM forcing recipes and CW01–CW04 routes: clear-to-scattered, increasing cover, overcast, developing/approaching storm.
- [ ] Fronts/staggered local births, mature renewal and survivors use live source state and one shared wind provider.
- [ ] Inject new controls/fronts after the original trace, alter storm intensity and wind midlife; run continuously without clip reset.
- [ ] Typed rain/lightning flux or potential/event causes, headless consumers, checkpoint/restore and provider stop/load keep honest source epochs.
- [ ] Same-condensate parts/adjacent tiles partition once; distinct layers add. Rain/lightning cause stubs keep IDs/time/support with main draw off.
- [ ] Done: observed normal/accelerated local development, consumer correspondence, weather continuity and cost. No global sky crossfade.

## P7 — full standalone skies and fixture admission

- [ ] Freeze representative counts/coverage/routes from CLOUDS; named512-event/etc cases are stress fixtures, not hard product populations.
- [ ] Wide/close/horizon/clear-side storm/deck-edge, repetition/renewal, low Sun, cold/seek/swap/resize/recovery and all-off/retained modes.
- [ ] Same accepted settings in visual and exclusive benchmark runs: actual M2 cloud5ms, total standalone16ms and fixture-browser peak512MiB comparisons.
- [ ] Retain direct velocity evaluation and shared per-tick float32 caching until a matched full-sky A/B decision: five-family/layer mixes, calm and storm/transition workloads,1x/16x/64x, CPU/GPU/physical memory/bandwidth/cache pressure, whole graph and source floor. Single-Cu results do not eliminate the option; preserve equivalent source outputs and visual quality.
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
private-source investigation, scientific solver, arbitrary cross-provider live migration, special far codec or new light fit is a
prerequisite for a required P stage. Proven source/transport references and rejection witnesses remain useful.
