# Independent cloud capability discovery

1 November 2026. Bounded requirements discovery for the cloud-system-design worktree; no implementation,
visual, motion or performance validation was performed. This first inventory precedes comparison with the cloud
inventory/design/proof code. It selects observable requirements and rejection witnesses, not a preferred algorithm.

## Inputs and independence boundary

Read the exact original commission at
`/Users/ryannel/.codex/attachments/ddebda96-45ae-46ab-9ede-f75a48bd1b7e/goal-objective.md`,
Thrum AGENTS/CONTRIBUTING, repo-local module-design skill and blue-sky/review/performance references,
ARCHITECTURE-BRIEF, C4 CONTRACTS, BUILD-PLAN, relevant CONSTRUCTION-PLAN/BASELINE-SPEC and
PLATFORM-RUNTIME context, and ENVIRONMENT's neighbouring contracts. No CLOUDS.md,
CLOUD-CAPABILITIES.md, legacy cloud implementation or proof code was read. BUILD-PLAN and C08 exposed
incidental summaries of the existing cloud extension; those are not used as target mechanisms. This is an independent
capability discovery, not a claim of blindness to the existence of earlier work.

Morphology was checked against the primary
[WMO Cloud Atlas identification guide](https://cloudatlas.wmo.int/en/cloud-identification-guide.html)
(text inspected, not an independent photographic reference review). It distinguishes puffy isolated cumulus,
large cumulonimbus towers, fibrous cirrus, low rounded stratocumulus fields and low stratus layers. The cue-level
acceptance deductions below are proposed engineering witnesses; WMO does not specify their lifecycle algorithms.
A later appearance review must inspect the project's actual selected reference images at native scale.

No physical simulation fidelity beyond the requested observable behaviours is inferred. An authored causal model can
qualify if it actually produces those behaviours consistently. A broad atmospheric simulation, microphysics solver,
fixed numerical cloud altitudes or a particular volumetric representation is not commissioned by the wording alone.

## Requirement classes

**U — explicit user commission:** all five families; shapes, variation and full lifecycles; wind/altitude/coverage/weather
controls; isolated and complete-sky design; shadows/reflections; normal and accelerated sampled motion/native images;
combinations and named changing-weather sequences; precipitation/lightning seams; progressive measurement and final
whole-sky assessment; pass/fail/unproven accounting and best verified paused proof.

**E — existing engine requirement:** independently selectable clouds, attenuation and reflected/environment services;
orthographic and perspective/multiple views of one world; shared environmental/time/unit semantics; valid media
composition; versioned authoring, content, histories and resource lifecycle; desktop/browser/mobile quality profiles.
These are integration obligations, not proof that every target has already been built.

**N — necessary implication:** identity, causal continuity, consumer consistency and bounded representation transitions
needed to satisfy U/E; controls with defined scope and invalid-input behaviour; enough diagnostics to reject false
success. These can be implemented with appropriately different consumer fidelity.

**O — optional extension:** full meteorology/climate; every WMO genus/species; hurricanes/tornadoes/mammatus;
precipitation or lightning rendering/physics beyond existing Thrum scope; lightning illumination internals; optical
haloes/rainbows; cloud collision or aircraft gameplay; unrestricted camera travel inside/above clouds; arbitrary
world streaming/live model swaps; network weather prediction; all physical cloud mass/energy conservation. No O item
may silently become an acceptance blocker. Required storm development is U, not O.

## Required families and isolated witnesses

Every family runs a seeded, repeatable scenario with explicit world scale/base/top, family controls, initial state,
forcing timeline, Sun/sky conditions and view routes. Each must demonstrate formation, growth, a sustained mature
interval and breakup. Cloud formation requires meaningful support/shape evolution: fading a fully formed object in
and out or moving a noise texture alone is not enough. A procedural/artist-directed lifecycle is allowed, but must
explain the state changes that produce the claimed cue. Separate phases can be separate clips of one reproducible
history if a complete lifecycle is too long to inspect continuously.

| ID / basis | Family-specific required outcome | Concrete acceptance witness | Reject if |
| --- | --- | --- | --- |
| F1 U | Fair-weather cumulus: separated low formations with recognizable bases and rising lobes, different sizes and silhouettes | Side-lit and backlit routes show coherent base/body/top relationships; new lobes develop, mature bodies persist while local detail changes, fragments erode and disappear; distinct seeds visibly vary gross morphology | All formations are scaled copies, buoyant blobs without a credible base, static geometry with scrolling detail, or entire-body opacity pulses |
| F2 U | Towering cumulus and cumulonimbus: substantial vertical development and anvil formation | One tracked formation develops upward from its base through a tower to a laterally spreading upper structure; independent nearby example varies proportions; lower body persists during upper growth; mature storm and decay stages remain distinguishable | A prebuilt anvil fades in, whole-body scaling is the only growth, a tower is merely a taller F1, or anvil/storm outcome is omitted |
| F3 U | Broken low cloud and stratocumulus: low patch/roll/cellular structure with gaps and local connectivity | Wide and close routes show rounded connected areas with openings; growth/erosion changes cell boundaries and gaps while mature structure advects coherently; variation changes connectedness, thickness and arrangement | Independent cumulus balls stand in for a layer, tiled cells are conspicuous, coverage changes by global opacity alone, or holes appear solely from camera sampling |
| F4 U | Layered stratus and overcast: broad horizontal coherence and meaningful thickness/coverage | Layer formation/spreading closes gaps toward overcast; mature underside remains extensive; thinning and fragmentation reopen sky; include at least one overlapping layer case and a low-Sun view | Overcast is a flat grey screen overlay, only disconnected puffs exist, depth/altitude is unknowable, or the whole sky resets to switch weather |
| F5 U | High wispy cirrus: thin directional strands and high-layer scale distinct from low cloud | Several fibrous/filamentary formations vary direction/extent; strands form, stretch/reshape coherently and thin/break; high versus low layers differ believably in parallax, projected scale and motion | Puffy low-cloud noise is flattened, strands flicker/reseed, the high layer is glued to the camera, or cirrus uses the low layer's identical motion without a declared wind profile |

The base/top/strands/rolls cues are acceptance deductions used to distinguish the requested families, not a requirement
to reproduce every real-world subtype. F2 contains two necessary stages/subcases, towering cumulus and anvil-bearing
cumulonimbus, rather than a choice between them. F3 must cover broken distribution and stratocumulus structure; F4
must cover layered stratus and full overcast rather than only a generic middle cloud deck.

Shared isolated gates (U): at least three meaningfully different formations per family are a useful proposed witness,
not a new capacity limit; inspect normal and accelerated motion at identical seeded times, native full frames and
native crops, near/intermediate/far views relevant to the family, horizon and upward/slant views. Establish the
minimum supported distance and camera-height envelope before representation selection. Ground/shore/height markers
or diagnostic world-space views must make scale and altitude assessable. Camera-inside-cloud flight is optional
unless selected by the actual supported-view envelope; low stratus around the supported camera cannot be ignored.

Required lighting conditions must be closed explicitly in the revised design. A discriminating proposed matrix is
high Sun, low Sun, side/back light, and overcast illumination, with darker interiors/edges assessed without display
clipping. Day-to-night, Moon, local-light penetration and halos are not implicitly required by the original commission.
Use Sun/sky changes actually required by Thrum; label unsupported regimes. Each isolated family also needs an actual
receiver-shadow and reflected/environment witness. A near-zero cirrus shadow is acceptable when supported by its
optical source, but absence must be demonstrated rather than skipping the consumer.

## Complete capability and interaction account

| ID / basis | Required capability / owner boundary | Acceptance and rejection witness |
| --- | --- | --- |
| C1 U/N | One causal operating model: drivers → persistent formation/layer state → structural evolution → rendered/query representations | For each F1–F5 trace one full lifecycle without inventing a missing operator. Reject labels/equations lacking producers, restart-on-change or shading animation masquerading as formation |
| C2 U/E | Weather/wind owns shared forcing; clouds consume altitude/position/time-indexed wind and expose coherent response | Change direction/speed/shear with tracked formations at two altitudes. All views/consumers agree on world location/time; reject unrelated drift phases, per-camera updates or teleporting history |
| C3 U/N | Authoring controls for family, placement/domain, base/top/extent, coverage/distribution, variation/seed, lifecycle pace/state and weather forcing | Reproduce the same scenario from ordinary admitted sources; change one causal control and inspect predicted effect. Document valid ranges, scope, dependencies and reset/restart semantics. Reject hidden transient editor state or a coverage knob that cannot produce requested topology |
| C4 U | Multiple formations and overlapping layers share resources while preserving individual variation | Two or more family layers overlap in view and on Sun paths; high/low motion differs according to shared profile. Reject synchronized lifecycles, family replacement at overlap or lighting dependent on draw order |
| C5 U/E | Common cloud causes supply visible cloud, Sun attenuation/receiver shadows and reflected/environment appearance | Track a cloud moving offscreen while its shadow/reflection remains relevant; change Sun direction, weather and view. Reject mismatched shapes/epochs, missing offscreen casters, doubled cloud attenuation or reflection independent of sky state |
| C6 U/E | Cloud/air/fog/opaque/water transport composes under C08 with one owner per contribution | Occluding terrain/receiver, a fog/cloud overlap and water reflection use declared common incident lighting. Reject additive independently lit overlapping composites, double attenuation, cloud drawn over foreground geometry or wrong air/water ordering |
| C7 U/N | Coverage, depth, distribution, scale variation, layering, distance and horizon compose a complete sky | Fair-weather, broken, overcast and storm skies each run wide and closer routes. Reject repeated stamps, visible domain edge, horizon wall, unexplained empty far sky, camera-centred cloud motion or silhouette popping |
| C8 U/E | Near/far/LOD representation transitions and resource sharing preserve causes and consumers | Walk/dolly/rotate through transition distances, change view/FOV, add a secondary view and return after skips. Reject disappearance, changed identity/coverage, old shadows, reflection-only resets, seams or unbounded duplicate residency |
| C9 U/E | Normal/accelerated time, pause/restart and repeatable inspection share explicit time semantics | Same source/seed/forcing yields the same declared state envelope; accelerated inspection reaches expected stages without hiding discontinuities. Reject renderer frame rate controlling growth or added views advancing weather |
| C10 E/N | Visibility, attenuation and reflected/environment consumer inclusion are independent | Omit main sky draw while retaining shadow/reflection consumers, then disable all cloud consumers. Reject a retained consumer disappearing or unused work/assets still resident without declared need |
| C11 E/N | Histories, corrections, edits, device/profile generations and content replacement have bounded lifetime | Camera cut, source reset, resize, skipped secondary view and provider-off cannot show stale cloud trails or publish stale async products. Unknown prior structure declares invalid/reactive history rather than fake rigid motion |
| C12 U/E | Stable weather-output seams for precipitation, lightning, water, surfaces, vegetation and audio | Weather/storm state emits spatially/time/identity-qualified causes with explicit units/support/validity. Test consumers with stubs; reject rain truth derived from visible particles, camera culling suppressing causes, or separate wind/weather authority |
| C13 E | Editable/cooked provenance, schema, dependency closure and authoring changes follow F4/T1/P7 | A saved scenario restarts against exact products/revisions; preview versus source mutation is explicit; invalid changes retain the last valid source/run. No general seamless streaming is inferred |
| C14 U/E | Work/memory/scaling and whole-frame cost remain independently reported from quality | Record per-family and combined CPU/GPU work, sample/march/coverage/view multipliers, shared versus per-view bytes, transition peaks, startup/pipeline work and full-frame distributions. Reject extrapolated full-sky viability from one isolated cloud |

C12 needs seam implementation and a consumer demonstration/stub, not automatic implementation of rain, lightning,
wetness, audio or water internals. If storms require additional weather variables, the revised design must assign
producers and semantics at that seam; a cloud archetype label is not a complete storm driver. Conversely, clouds need
not own climate, water or precipitation simulation just to expose coherent causality.

## Named weather sequences and proof order

1. **Complete design/review:** close C1–C14 across F1–F5, including content preparation and authoring; review before
   extending dependent implementation. Unknown numeric performance/visual qualification remains an honest open gate.
2. **Isolated family proofs:** all F1–F5 individually pass structural, lifecycle, motion, scale/altitude, lighting,
   shadow, reflection and relevant-view gates. Correct a failed family before integrating it as accepted content.
   Cheap analytic/contract probes can precede family visual proof; they cannot substitute for it.
3. **Combinations/changing weather:** demonstrate overlapping low/high and thick/thin formations; shared lighting,
   layer-dependent winds and continuity. Required named transitions are clear-to-scattered, increasing cover,
   developing overcast, and developing or approaching thunderstorm. At least one storm must show F2 vertical/anvil
   development somewhere in its family proof; a static approaching storm alone cannot close that requirement.
4. **Complete skies:** fair weather, broken cloud, overcast and storm scenarios with distant/horizon content, wide and
   close routes, receiver and reflection consumers. Preserve mature formations while weather changes around them;
   stagger births/deaths. Record repeatable full-sky weather sequences and normal/accelerated samples.
5. **Complete workload qualification:** retain the same visual/behavioural witnesses while measuring the assembled
   workload and all active consumers. Progressive measurements can reject architecture early, but a cheaper reduced
   demonstration is not completion of missing required behaviour. Leave the best verified proof paused.

For the four transitions, the design must state which forcing changes, which existing formations grow/erode/advect,
where new support is created, how coverage topology changes, and which shared outputs update. Reject whole-field
crossfades between unrelated skies, identity resets, instantaneous domain replacement and moving detail alone.
Authored forcing timelines are allowed; a scientific weather forecast is unnecessary.

Each proof record needs exact source/build/configuration/seed/time/view/product identity, inspected native images,
actual normal/accelerated motion or explicitly labelled sampled-frame evidence, quality verdict by indispensable cue,
consumer observations, measurement scope and unresolved limitations. Reference selection, candidate generation,
engineering pass and user appearance acceptance are distinct. A still, byte hash, optical scalar or finite-value test
cannot close dynamics or experienced appearance. No actual capability is marked passed by this discovery report.

## Thrum dependencies and scope conflicts to reconcile

Construction remains I1 authority → I2a immutable source/content/private-save/restart → I3a minimal presentation,
with I2n real networking alongside it; broader cloud/environment consumer integration follows relevant D2/I2b/I3b
contracts. This commission authorizes cloud proofs in the existing worktree, not skipping that engine sequence or
claiming the proof host is the engine runtime. Retain useful renderer/reference/research tools where compatible.
Cloud capability design is complete across the commissioned scope even when engine consumer binding is unbuilt.

A bounded proof-host can exercise seams now; report actual engine bindings separately. Bindings depend on F2
configuration, F4 content/residency, W1 time/state, V1 graph/lifetime, V2 visibility, V3 transport/reflections/shadows,
V4 reconstruction, T1 authoring and T2 evidence. P7/N1 matter only for selected authoritative weather/query use;
presentation-only detail cannot create gameplay authority or require headless GPU allocation.

The numeric complete-frame proposals are desktop CPU ≤5 ms/GPU ≤10 ms at 60 Hz, browser CPU ≤8 ms/GPU ≤16 ms
at 30 Hz and mobile CPU ≤7 ms/GPU ≤16 ms at 30 Hz, with respective 3 GiB/512 MiB/1 GiB planning caps.
These are unqualified envelopes, not runtime results. Cloud suballocation must be read from its owning revised
cloud design during comparison; this discovery deliberately does not inspect that design. Count shared resources
once, all simultaneous consumers/views, and old/new transition/staging/history peaks. A cloud suballocation miss is
an open cost gate, not authority to make storm/cirrus/lifecycles optional. Native proof measurements cannot certify
portable profiles or sustained thermal behaviour.

**Concrete already-visible gap:** BUILD-PLAN's cloud handoff says storm progression remains optional after the base
and places local cost qualification before broader weather scope. The exact current user commission explicitly
requires storm progression and complete weather/sky behaviour, with progressive measurement throughout. Reconcile
that handoff and any inherited narrower claim with the commission. Cost study can be an early rejection probe;
it must not silently replace the required design/proof queue. No findings about CLOUDS/CLOUD-CAPABILITIES or code
are claimed until the later comparison review inspects them.
