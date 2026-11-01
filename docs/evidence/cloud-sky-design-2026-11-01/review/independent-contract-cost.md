# Independent whole-sky contract and workload challenge

1 November 2026. Bounded review of the complete-cloud commission. **Analytical accounting only: no new benchmark, implementation, visual pass or device certification.** Sole owned output is this scratch report. Canonical files, source code and existing evidence were not changed.

## Basis and conclusion

Read the exact goal attachment, Thrum AGENTS, module-design/performance and repository skills, ARCHITECTURE-BRIEF, C01/C02/C04/C05/C07/C08/C12, RENDER-SPEC, ENVIRONMENT, baseline budget and construction records. Derived the family/domain/workload obligations below before opening CLOUDS or CLOUD-CAPABILITIES. BASELINE and ENVIRONMENT expose summaries of the existing RC proposal: independence means requirements-first derivation, not blindness to every prior proposal. Subsequently read the old cloud design and coupled performance disposition for reconciliation. No old implementation code was inspected.

The five requested families, isolated lifecycle proofs, combinations, weather progression and four complete skies are **required by the new commission**. Earlier optional labels cannot reduce that scope. Rain/lightning implementations remain separate, but concrete source/integration seams are required. Exact all-orders scattering, a moist-fluid solver, orbital viewing, all ten meteorological genera, unrestricted aircraft flight and seamless gameplay streaming are not necessary implications.

The common-source, independent-consumer and C08 boundaries remain useful. RC-one/RC-four capacity and a 4 km optical domain do not specify a whole sky. A bounded one-source instrument already misses its cloud allocation. Full required behavior must remain the target while budget failures stay explicit.

## Family inventory and consequential controls

| Required family | Structure/lifecycle that needs a producer | Controls and consumer consequences |
| --- | --- | --- |
| Fair-weather cumulus | Condensation/base level; bright convex lobes, shaded flatter underside; local emergence, rising new lobes, persistent mature body, detached/eroded remnants; varied formations/ages | Base/top, size distribution, coverage, local birth/death and development, wind at height; all projections track actual support/phase |
| Towering cumulus/cumulonimbus | Substantial vertical development, heterogeneous towers, spreading mature anvil, sheared upper outflow, mature/dissipating material; more than uniformly enlarged cumulus | Growth envelope/rate, top cap, anvil spread/orientation, height-dependent wind; stable cell, precipitation and lightning seams |
| Broken low cloud/stratocumulus | Connected banks/cells and meaningful irregular openings; varying bases/thickness; openings close/develop and cells join/separate without synchronous global fade | Coverage/connectivity/gap-size distribution, base/thickness, directional organization/shear; patchy receiver light and reflected gaps |
| Layered stratus/overcast | Extensive deck with varying optical thickness and coherent underside; distinct vertical levels; formation by filling/merging, breakup through openings | Deck support/base/top, extinction/thickness, level separation; diffuse/attenuated lighting, low-Sun rim/base and cloud-on-cloud light paths |
| High wispy cirrus | Thin fibrous/streaked structure, taper/fraying and broad organization; emerging/stretching/breaking with persistent streaks | High altitude, anisotropy/orientation, thin optical depth, wind/shear and independent ages; overhead/horizon lighting and matching attenuation/reflection |

Every family needs meaningful variants and unsynchronized phases, normal/accelerated motion, changing Sun/sky, reflected rays and appropriate distance/angle witnesses. A finite thin/lower-dimensional cirrus representation may be valid; its view/lighting/thickness regime must be stated. A horizontal image cannot prove a tower or low-cloud entry. Authored deterministic evolution is valid when labelled; shared weather does not imply meteorological conservation.

Causal authoring exposes source/revision/support/seed, family/variant, physical base/top/scale, phase/birth/death, coverage/size/connectivity distributions, wind versus height, optical units, front trajectory, Sun/sky and distant/detail policy. Density, optical depth, ground footprint and image alpha are distinct quantities. Camera visibility cannot seed sources.

## Required domains and integration closure

1. **Altitude/range:** Ground cameras see 8–14 km cirrus/anvils without flying. Baseline air (camera 0–500 m; path ≤2 km) and RC (0–1536 m; ≤4 km) cannot contain them. Keep those existing recipes valid and admit a distinct extended-sky closure. Raising a renderer FAR value alone supplies neither atmosphere nor source lighting.
2. **Near/inside:** Retain ground-accessible low-cloud entry and hillside intersections. Other families need beneath/oblique/side views; high-altitude flight remains separately selectable. Interior Sun queries require partial optical depth, not a ground-only mask.
3. **Horizon:** Declare flat-ground/curvature approximation, support boundary, terminal sky and far-air owner. Finite decks cannot end at a camera-centred square or expose a horizon band. Planetary simulation is optional; coherent supported horizon treatment is required.
4. **Low Sun:** Shadow displacement is h/tan(elevation). At 12 km this is 44.8 km at 15 degrees, 137.2 km at 5 degrees, 343.6 km at 2 degrees. Offscreen cloud sources matter. Specify source/light-path guard and admitted Sun range; do not silently clamp sunlight or delete low-angle cues.
5. **Distant levels:** Full geometry, reduced middle-distance density and distant shell/statistical products retain identity/phase and matched optical-depth/coverage moments. Detail handoff has partitioned support, no duplicate extinction and matching shadow/reflection effects. Camera movement never reseeds or deletes clouds.
6. **Overlap:** Low bank beneath tower/anvil, high wisps above deck, intersecting anvils, fog in low bank and multiple sequential levels are adverse cases. Sum coefficients once under one C08 owner. Even disjoint camera intervals need illumination attenuated by all relevant layers along Sun/ambient paths.
7. **Geometry/interfaces:** Terrain/building/water depth truncates intervals. Reflected rays start at the interface; camera-to-interface media applies once. A perfect mirror fixture is not ocean/roughness/environment-light qualification.

### A concrete extended-air producer, not a larger FAR value

A bounded flat-world extended profile can retain baseline exponential air coefficients and analytic extinction, but must explicitly admit camera-to-terminal distances and altitudes through the complete source support. E1 publishes the shared atmospheric parameters/revision and clear terminal-background asset; V3 evaluates analytic segment extinction and joint air/fog/cloud source quadrature across that domain. Split at actual supports/geometry; use separate near/far step policies selected against radiance/source error, not the old 32-step cap assumed sufficient over 160 km. Air outside cloud still contributes source radiance.

For direct sunlight, integrate the same atmosphere from each shading point to its declared atmospheric top/escape boundary; cloud opacity from every intersected level and bounded fog/geometry applies once. Sun at positive elevation admits analytic exponential-air depth in the flat approximation; near-horizontal/negative elevation requires explicit ground/curvature/source visibility policy. A baseline cloud-free HDRI may remain an authored boundary with declared colour/illumination, no baked cloud/Sun duplication. It is not automatically a physically dynamic sky. Its ambient sampling and cloud occlusion need a named producer/cadence, including overcast. Terminal background, aerial source and environment-light products must state which air segment each already contains to avoid double atmospheric integration.

This is an implementable ownership/closure candidate, not a qualification of flat-world horizons or a selected replacement for root's eventual sky model. The complete profile must test long nearly horizontal rays, high cloud ground views, low Sun, and reflected legs; orbital/planetary physics need not be invented.

## Representative repeatable workloads

The values below are **review proposals for falsification**, not universal SDK limits, measurements or climatology. Different explicit values may be selected if equivalent witnesses remain; one/four-cloud scenes cannot replace whole-sky work.

Shared setup: fixed 128 ×128 km inner square; central 16 km radius eligible for close detail. Fixed far annulus 64–160 km with 96 stable 3.75-degree azimuth sectors carrying family/height/coverage/phase. Partition square/annulus overlap instead of duplicating density. Source support beyond the square is included where Sun guards need it: a 160 km appearance annulus does not solve a 343 km shadow path. Finite scene, no infinite-world/planet claim.

Low-family placement uses deterministic 32 ×32 sites at 4 km spacing with hashed offsets and stable per-family namespaces. Pin the variant/phase distribution in the manifest. Measure union plan footprint separately from projected sky occupancy; they are not interchangeable.

| Scene | Inventory, extent and physical occupancy assumptions | Image/distribution witness |
| --- | --- | --- |
| F fair | 512 cumulus IDs; diameters 1–5 km, RMS 2.8 km; base 0.8–1.8 km, tops 1.5–3.5 km. Nominal circular footprints sum 3153 km²/16384 =19.2% before overlap. 24 cirrus ribbons, lengths 8–24 km/widths 0.5–2 km at 8–12 km. Far sectors continue sparse organization. | Target 20–40% cloud sky pixels in fixed wide captures; many silhouettes/scales, clear gaps/Sun paths, high wisps overhead/horizon and dying remnants |
| B broken | 512 connected low-bank IDs; diameters 2–8 km, RMS 4.8 km; base 0.5–1.2 km, tops 1–2.5 km. Nominal footprint 9265 km² =56.5% before overlap/connectivity. 64 cumulus fragments in openings plus 24 high ribbons. | Target 50–80% cloud sky pixels; meaningful gap/connectivity distribution, near/far occlusion and rays crossing low/high levels; openings persist during pans |
| O overcast | One 128 ×128 km lower deck at 0.7–1.5 km, 0.8 km thick: support 13107 km³; 64 stable 16 ×16 km authoring regions. Second deck at 3–4 km covers 70% of plan area; 24 high ribbons for light/off-axis tests. Lower footprint 100%. | ≥95% cloud sky pixels centrally; coherent varied underside and depth, clearing/edge route revealing layer order; empty-space savings cannot be assumed |
| S storm | 12 system IDs: four growing/four mature/four decaying; cores 6–14 km across, bases 0.6–1.5 km, mature tops 10–14 km. Eight non-newborn systems have 20–40 km anvils. 384 low banks supply surrounding 50–70% plan cover; 24 ribbons outside/above storm. Anvils/cores intentionally overlap. | 70–100% cloud sky captures plus clear-side approach showing ≥3 systems, towers/anvil silhouettes; front changes illumination/shadow/reflection; a near black sheet cannot hide every defining structure |

Add cirrus-only and tower-only full-width witnesses. Five isolated proofs each contain at least four variants at different ages and meaningful family scale, not one 512 m puff with uniform scaling.

Fixed view/time assumptions:

- Browser main 1280 ×720, DPR1, 2 m camera, 75-degree horizontal FOV; pitches 15/60/90 degrees plus yaw sweep; media 640 ×360. Independent reflected colour 640 ×360/media 320 ×180. Desktop 1920 ×1080 with corresponding half-linear media. These are workload settings, not a universal DPR restriction.
- R1 second camera half main width/height every other frame. Include orthographic 2 km receiver view, oblique 20 km view, low-cloud entry route and 500 m hillside route. Pause/skip the second view while source continues.
- Sun elevations 75/30/15/5 degrees, two azimuths relative to wind; 2 degrees is stress/rejection until its domain is designed. Source-off and all-light-off retain extinction. Use two bounded fog volumes. Lightning/local finite light paths are separate stress cases, not assumed supported by Sun caches.
- One environment product with declared cadence/filtering. Accounting example: six 128² faces once per 0.5 source seconds plus diffuse/specular filtering. This is not an imposed replacement for the existing 128 ×64 probe; either must reflect common source age with bounded error.
- Repeat a 600-source-second sequence at 1×/16×/64×: clear→F (0–120), F→B (120–240), B→O (240–360), clearing/approaching S (360–600). Retain named survivor IDs through each transition, local births/deaths, asynchronous lifecycles and an existing distant storm entering view. Accelerated playback supplements an actual normal-speed weather witness.

## Contracts and weather seams

| Contract | Required source/consumer obligation |
| --- | --- |
| C01 | Stable source/formation/revision, world support/metres/seconds/origin epoch; cameras/rebases/slot reuse cannot move or reseed clouds |
| C02 | One committed age, explicit cadence/interpolation, immutable bounded leases; no per-view stepping, half-new cloud/old light publication, hidden accelerated clock or unbounded history |
| C04 | Main, attenuation, reflection/environment and optional CPU semantics bind independently; actual off savings; required retained consumers/families survive validated quality substitutions |
| C05 | Hashed variant/level source library, recoverable recipe/master, finite pipelines, device recovery and CPU/GPU/upload/in-flight replacement peak; no first-draw preparation |
| C07 | Conservative visibility for all views/light paths; actual previous accepted sample or explicit invalidity for growth/erosion/level change; offscreen blockers and skipped views remain correct |
| C08 | One shared coefficient/phase/support and source closure; joint incident light, ordered disjoint segments, all-level Sun paths, no duplicate cloud/Sun/background attenuation |
| C12 | Report N/U, reconstructed/dirty cells, sample/overlap/light work, all views/probes/fallback, bytes/allocations/cadence; whole critical path/tails/loaded identity, not only a kernel or still |

Precipitation seam: a world-space flux/type field, units kg/(m²·s), support/time/revision/validity and stable cell ID. Rain amount cannot come from opacity/particles. Deposits, streaks, sound and physical response have separate owners; shaft/footprint motion must agree with the source. Snow/virga/evaporation can remain explicitly unsupported model choices.

Lightning seam: stable event ID, committed start/duration, world path/region, light/emission descriptor, source/cell revision and deterministic replay at normal/accelerated rates. V3/V5 render lighting/effects; audio owns thunder timing. Culling/skipped views cannot lose an event. Pulses need history invalidation and environment-light cadence policy. Ordinary directional Sun depth does not answer finite point/line lightning paths. No new external API is required if existing typed queries express the path/domain/error and consumers bind explicitly.

## Analytical cost and light-cache alternatives

Let N be formation IDs, U distinct evaluated source/phase/shape combinations, Q cells per evaluation, P_v media pixels/view, s_v occupied steps/ray, A overlapping constituents and D source-path work. N ≠U: sharing immutable clips saves encoded bytes, but phases/shape and relative Sun geometry may still demand distinct current/light products.

Source reconstruction is O(UQ) plus mips/occupancy/invalidation; weather descriptors O(N) on their declared cadence. Joint rays cost ΣP_v s_v times coefficient/lighting work(A,D), support traversal and edge fallback. Looping over all N per sample is structurally unsuitable for hundreds of sources. A spatial candidate index belongs within existing owners. Reflection/probes have independent rays/filtering/cadence but share source facts/light products.

### Concrete multipliers, not timing predictions

- Main+mirror media: 288000 browser pixels, 648000 desktop. Illustrative 96 occupied steps gives **27.648M/62.208M coefficient locations per fresh frame**, before overlap/light work. Eight further locations per Sun query adds 221.184M/497.664M. This identifies risk; 96 steps is not a quality recommendation.
- Existing 256 steps/support gives 73.728M/165.888M locations for one fully occupied support; sequential supports multiply work. Overcast approaches full occupancy. Six 128² faces ×32 steps add 3.146M positions/update. R1 secondary camera adds 12.5% of main rays on average, 25% on active frames.
- 128³ is 2.097M cells. Thirty-two distinct current R16F phases alone use **128 MiB**, before encoded clips/light/history/scene. At 60 rebuilds/s they evaluate/write 4.03 billion cells/s. Shared phase products or other family-specific producers must be concrete; identical phases everywhere are not acceptable variety.
- Dense 128 ×128 ×14 km at old 4 m spacing is 3.584 trillion cells: **3.26 TiB/R8 frame;107.6 TiB/33 frames**. This rejects globally uniform dense clip residency, not the complete sky or all efficient alternatives. Extensive overcast weakens sparse-empty assumptions.
- Five uncompressed 33 ×128³ R8 clips are **330 MiB** before metadata, only one modest clip/family. Actual sparse/compressed ratios require qualifying family content. An 11 MiB cold source cap cannot be copied five times into the same cold closure.
- Hypothetical current+history at 20 bytes/pixel each (S8,T2,depth4,motion4,masks2) costs 10.99 MiB over 288000 pixels, excluding guides/fallback/colour. This does not replace the actual 56-byte history layout without precision/correspondence proof.
- Retracing 5% of 720p output at 256 steps adds 11.796M locations. Existing evidence has fallback peak 142675/921600 =15.48%; assuming negligible edge repair contradicts a retained witness. Layered/distant silhouettes may increase it.

### Shared cumulative field versus deep optical-depth curves

The external contract can remain typed τ(path,time,domain/error). Neither alternative needs a speculative engine API/crate. For a directional Sun, define one cumulative depth P(u,v,z) along the light axis including **all participating sources**, with a named entry boundary. Sun depth from any interior point comes from that cumulative field; finite collinear partial depth is a difference of cumulative values. Both a 3D grid and piecewise deep curves can therefore answer cross-layer and self-partial queries. A ground-only map cannot. Per-instance local prefixes omit other formations unless their contributions are explicitly accumulated/query-combined.

| Alternative | Accounting and strengths | Main rejection risks/required producer details |
| --- | --- | --- |
| Sparse/cascaded light-aligned 3D cumulative field | A 256² ×128 R16F field is 16 MiB/cascade; four =64 MiB, before guards/overlap/history/replacement. Good direct partial queries and fixed address cost; occupied updates can be local. | Must propagate accumulated depth through upstream empty space, define outer-boundary carry and cross-cascade paths, avoid double counting overlap, preserve depth at cloud/layer boundaries. Tall towers/low Sun enlarge support. Dense overcast fills columns. Prefix rebuilding after upstream motion can dirty downstream cumulative values far beyond changed density. |
| Deep per-column monotonic depth/τ curves | 1024² ×16 knots ×(depth16+τ16) =64 MiB/map; four such cascades =256 MiB, before counts/indexing/guards. Eight knots halve this. Adaptive knots can encode empty altitude gaps efficiently. | Per-column knot production/reduction needs a maximum τ/interpolation error and explicit overflow/refinement. Separated wisps, sharp tower sides and many levels can require many knots; fixed 8/16 capacity cannot silently drop a layer. Lateral interpolation must preserve partial depth/visibility at disocclusion. Dynamic reallocation/sorting has a measured cost. |

These examples compare layouts, not equal-quality selected capacities: a sparse grid can compress and deep curves may use smaller maps/quantization. A 1024-wide map over 256 km has 250 m lateral cells; it cannot establish near-cloud fine shadow correctness without cascades/detail. Both need finite precision control: τ_far−τ_near suffers cancellation under large cumulative depth, and interpolating quantized τ needs an absolute-depth/error bound. Filtered mean τ does not equal filtered mean transmittance exp(-τ); prefiltering must state its receiver/footprint policy. Sun direction changes invalidate/reproject products only under documented accuracy conditions. Lightning requires finite nonparallel paths or a separate local-light provider.

Recommended selection test: analytically derive required lateral/depth errors for near wisps, two separated layers, opaque storm core and low Sun, then construct both candidates at matched error/memory or reject one by those bounds. Keep exact direct coefficient integration as a finite oracle. Preserve source-specific Q1 as a golden, not a universal current-field architecture. A world/hierarchical shared light product appears structurally appropriate, but neither candidate is performance-qualified here.

## Actual budgets and adverse evidence

Baseline whole-frame: desktop CPU/GPU **5/10 ms at60 Hz**, browser **8/16 ms**, mobile **7/16 ms at30 Hz**; memory planning **3 GiB/512 MiB/1 GiB**. Due simulation ticks share 30% CPU, extraction/submission40%, input/UI/audio10%, prepare10%, reserve10%; two mobile ticks do not get two allocations. All views/shadows/media share GPU time.

Optional RC targets: cloud CPU0.5 ms, browser GPU5 ms, native GPU3 ms. Gross service reservations 88 MiB at720p,128 MiB at1080p; RC-four proposes112 MiB browser for four phases sharing one clip. **These are not whole-sky budgets or support measurements.** Browser category shares remain76.8 MiB world,102.4 decoded,179.2 persistent GPU,51.2 transient,51.2 staging and51.2 reserve; different owners cannot spend the same reserve.

RC cold worksheet16.5 MiB/4.69 s is an estimate; high-cap22.25 MiB/5.65 s already fails5 s. Larger game/family libraries need a combined closure. Do not invent compression/overlap to make startup pass.

[Coupled performance disposition](../../cloud-coupled-2026-11-01/PERFORMANCE.md): one128³ source,720p main/mirror,two fogs,fresh source/cache/rays/fallback/display; M3 Max browser WebGPU bracket median12.845 ms,p95 18.809,p99 21.758,max23.855. CPU encoding median0.10 ms is not total CPU. Invalid render-begin timestamps are excluded; compute-sentinel overhead/interference limits remain documented. Logical GPU57.512 MiB+CPU source6.260 =63.772 MiB before overhead; complete residency/512 MiB scene unmeasured.

This misses5 ms cloud and has tails beyond16 ms full browser before other game work. Native3/10 ms and named M2/mobile targets are unmeasured. Do not multiply milliseconds by formation count: costs can share/overlap. Do not treat one-source optical correctness as hundreds-of-formations admission either. Full-sky cost/quality gates remain open.

## Reduction order and required dispositions

1. Share authority/immutable clips and identical evaluated products with complete keys/invalidation; report N and U separately. Relative lighting and other-cloud shadows can prevent cache reuse even for equal phase.
2. Bound candidate traversal and dirty-region work; retain births/tails, shadow-only/reflection-only sources. Prefix dirty support may exceed local density edits.
3. Select family-appropriate anisotropic/low-dimensional/resolved/distant producers under common coefficients. Preserve optical depth, broad shape, identity and all-consumer handoff. Near entry/tower cues cannot become distant cards silently.
4. Share all-layer light products/ambient updates; account cache construction/bandwidth against removed queries. Use actual Sun/support/error domains and lightning policy.
5. Remove graph transfers/duplicate preparation, qualify compact/scalar layout and allocation reuse while retaining optical/image witnesses. Logical bytes are not physical residency.
6. Only then reduce sample/resolution/cadence or add temporal reuse with valid retained-time correspondence/reactivity. Count full-resolution fallback. Never slow weather time, delete family behavior or hide new density to meet budget.

Blocking dispositions for canonical owner:

| Finding | Required closure |
| --- | --- |
| Old optional labels contradict new scope | Five families/four skies/transitions become acceptance gates; separate precipitation/lightning implementations from mandatory seams |
| High/distant/low-Sun domain incomplete | Select air/terminal/horizon producer and full source-light support; qualify long ground-to-high-cloud and near-horizontal/reflected paths |
| Sky source sharing unspecified | Specify family producers,N/U,phase/variant diversity,levels,occupancy and complete memory; global dense/per-formation copying is not an answer |
| All-layer/ambient coupling unqualified | Trace bank→tower→anvil→cirrus and overcast diffuse/reflected/environment paths under one C08 closure |
| Transitions may reset/synchronize IDs | Concrete front/coverage driver,survivor IDs,local births/deaths,layer winds and normal/accelerated replay; preset crossfade insufficient |
| Existing cost failure open | Preserve adverse receipts, reduce cost against retained witnesses and qualify the complete workload; do not shrink commission |
| Storm seams need finite paths/events | Existing E1/V3/V5/audio owners bind precipitation flux and stable lightning events; pixels/ordinary Sun caches are not authority |

No runtime rewrite or speculative package is proposed. The registry currently contains only thrum-core and xtask. D1/D2 and I1/I2/I3a/I3b construction dependencies remain authoritative; authorized standalone instruments can prove bounded models without claiming public bindings. This review stops at analytical challenge; proposed values/reductions remain unqualified.
