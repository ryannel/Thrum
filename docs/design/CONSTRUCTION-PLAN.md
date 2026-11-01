# Detailed design and construction plan

1 November 2026. Selected ordering and remaining detailed-design commissions for the [integrated design](SYSTEM-DESIGN.md). Runtime construction is not
performed in this commission. Derive mechanisms analytically first; qualification checks stated predictions and
empirical limits. An unresolved producer, representation or coupling remains a design task, never a disguised test.

The [simplification review](SIMPLIFICATION-REVIEW.md) led to the smaller R0/I2n order below. The
[baseline](BASELINE-SPEC.md) owns portable transition and browser admission worksheets; the
[network specification](NETWORK-SPEC.md#r4-encoded-admission-fixture) owns R4 encoded-byte and latency admission. Those are analytical gates before
dependent construction, not measured runtime or platform qualification.

## Decisions before construction

The [selected decisions](ARCHITECTURE-DECISIONS.md) resolve A01–A05/J01–J04 and WP01–WP06. The subsequent
[baseline specification](BASELINE-SPEC.md) itemises G01–G10 and records initial selections and scoped admission gates,
including first-class multiplayer. These specifications govern construction; the original broader family designs
remain the destination and extension map. A richer provider does not inherit readiness from the smaller baseline.

| Area | Concrete initial selection | Implementation/qualification obligation |
| --- | --- | --- |
| Host/targets | Rust 1.99.1, winit 0.30.13, wgpu 30.0.1; named Mac/Windows/browser/iOS/Android reference candidates | Resolve/pin full dependency/native toolchains, compile adapters, then qualify actual target behavior |
| Source/world/storage | Ordinary working files admitted as one immutable native source snapshot; private hecs 0.11.2; finite TRS envelope; restart/visible stop-load for first substantive replacement | [FOUNDATION-SPEC](FOUNDATION-SPEC.md): implement native admission/recovery, world and save failure traces; browser authoring follows a named need, while browser play cache and player save remain in scope |
| Rendering/media | WGSL/Naga ABI, finite material/pipeline plan, Rec.709 photometric transport and joint single-scatter air/fog | [RENDER-SPEC](RENDER-SPEC.md): codec/layout, pipeline preparation, numerical/visual and per-device proof |
| Play/environment | Rapier adapter, owned clip/IK runtime, Recast/Detour navigation; prescribed sea and finite shallow operator with explicit validity | [PLAY-ENVIRONMENT-SPEC](PLAY-ENVIRONMENT-SPEC.md): implement exact selected operators, compile/notice checks and coupled numerical/cost proof |
| Multiplayer N1 | Four-player finite preloaded server world, concrete WSS protocol, typed intents, committed outcomes and atomic full StateGroups; optional prediction/water recipes | [NETWORK-SPEC](NETWORK-SPEC.md): early I2n vertical path, encoded R4 fixture, playable/degraded latency cases and fault qualification; no deferred authority redesign |
| Creator/release | One native source/operation surface, isolated play/restart, one first-game development export, then spatial authoring; pinned-release SDK and bounded retained outputs | Implement actual CLI/workbench commands against the schemas; test clean recipient/export/recovery; no upgrade promise without its separate evidence |

These are specifications and bounded proof obligations, not completed runtime features. Unsupported initial combinations
fail clearly. Future capabilities retain the same small ownership/contracts and receive their own complete design
before construction; they do not require a universal extension or solver framework now.

## Performance is part of every work package

Apply [the investment map and cost account](AI-PERFORMANCE.md#performance-investment-from-the-first-design) during D1/D2
and each implementation slice. Resolve consequential scaling/layout/ownership choices before their APIs become dependencies;
bring relevant checks and instrumentation with the implementation. I1 starts with frequent world/runtime paths and
local/replica consumers; content, rendering, play, audio/UI and real networking extend this as they arrive. The first
interaction is an integration fixture, not the only scale workload or an upper bound on ambition.

I5 adds advanced capabilities; it does not defer performance-conscious representations, target specialization or
optimization campaigns until the engine is otherwise complete. Internal complexity is acceptable when its useful
advantage, invariant checks and maintenance/target costs are accounted for. Existing runtime contracts and initial
scope remain selected; no baseline becomes qualified simply by meeting this design obligation.

## Ordered work packages

Developer composition is foundational, not an I4 retrofit. The [scenario pass](SCENARIO-WALKTHROUGHS.md) recommends
explicit static game setup, direct typed registration and finite target tiers; optional packages serve justified reuse.
D0 records this selected baseline and its detailed obligations in the [developer experience](ENGINE-EXPERIENCE.md).
D1 specifies the required registration, schema/access/schedule validation and profile locks without constructing
general provider search or gameplay live-removal machinery by default. I1 implements the smallest game-owned extension
and a headless profile. I2a admits one snapshot and save path for R0; I2b grows content/region capability only after
the useful game and early network path have exercised those boundaries. The first I4 target slice accompanies I3a;
I4 qualifies the broader profiles. Public extension conformance and absent-feature cost remain acceptance criteria.

| Package | Deliverable and prerequisites | Analytical acceptance before building | Later qualification prediction |
| --- | --- | --- | --- |
| D0 Product and shared contract closure | Structural choices selected in C4; specify scales and named device candidates for each workload; budget simultaneous use | No contradictory owner/time/unit/version; baseline recipes resolve; [R0/R2 boot and transition worksheet](BASELINE-SPEC.md#browser-first-play-admission-worksheet--estimates-not-measurements) and [R4 encoded fixture](NETWORK-SPEC.md#r4-encoded-admission-fixture) fit their proposed reserves | Qualification can distinguish unsupported configuration from performance failure |
| D1 Foundation/content detailed design | Exact identity/schema, TRS error envelope, task/publication, lifecycle, working-file-to-immutable-snapshot admission, asset/cook and per-authority save protocols; preserve N1 authority boundaries; depends D0 | Trace normal and interrupted native file application/admission, stale results, memory admission, crash recovery and migration; specify wider streaming separately | No stale resurrection, partial authoritative region, lost accepted edit or unbounded queue |
| D2 Complete render/character/environment seams | Material/transport, geometry/deformation/LOD, physics/root motion and environment field/force contracts; depends D0, coordinates/schema from D1 | Derive common representations and approximation error over all required cameras/profiles; show same causes/geometry/time across queries and images | No duplicate motion/lighting/attenuation, missing history producer or incompatible off-state |
| I1 Custom substrate and headless runtime | Selected adapters, clocks, jobs, schemas, diagnostics and local authority/replica roles; thin iOS/Android shells with boot content, tick, suspend/resume, surface recovery and basic input in parallel with the small-game path; depends completed relevant D1 | Implementation plan names resource/lifecycle invariants, dependency costs and deterministic envelope; in-memory two-peer fixture starts with typed action and state boundaries | Headless and target packages obey same world semantics; named phones expose host/lifecycle faults early, without waiting for broad I3b |
| I2a Minimal source/content/save path | One bounded preloaded R0 region, working-file admission to one immutable snapshot, basic cook/load, private save/restore and isolated restart; depends I1 and required D1/F4 subset | Required gameplay/render products, exact source/build/save identities and interrupted admission/recovery are specified; substantive edits restart | One editable source change survives admission, run, private save, restart, restore and failed-cook recovery without region streaming or full workbench machinery |
| I3a Small rendered game and one development export | One camera, basic material, action/rule/state/result, inspect/restart, source edit and clean recipient handoff; depends I2a, required D2 subset and one I4 target slice | Source/play/shader/input mechanisms complete for this game; browser boot worksheet and render demand apply to any browser export; omitted capabilities require no runtime work | Newcomer and veteran create the same small game from empty/starter source, edit and export it without full character/environment/audio prerequisites |
| I2n Multiplayer vertical path | Run the same small-game actions through finite preloaded server/replica worlds and real WSS; typed intents, committed per-action outcomes, atomic bounded full StateGroups, compatible join/reconnect, interest and effect identity; depends I1/I2a identity/content and NETWORK-SPEC, develops alongside I3a | Trace content-before-baseline, epoch/action resolution, atomic spawn/first state, finite encoded R4 bytes/cadence/bursts and aggregate queues; full groups selected for the first recipe, with deltas/prediction/water optional | Four-peer reference session including native/browser clients, real delay/stall/reconnect faults, no duplicated committed effect, independent GPU loading and playable versus degraded whole-game frame/state-age evidence |
| I2b Broader content/region and artist path | Import/reimport, source operations, two-region load/unload, visual preview and spatial workbench; depends I2a and the relevant D1/D2 contracts | Scoped live region activation, provider transfer and old/new peak fit a named workload; otherwise visible stop/load is explicit; no portable seamless claim follows from R2 | The R1 authored scene edits, cooks, activates, unloads and restores without changing identity or losing state; separate live-transition qualification records peaks |
| I3b Integrated play/presentation path | Character motion/animation, renderer baseline, audio/UI and one selected environmental provider; depends the necessary I2b region/content subset and D2 | Trace full tick/frame, motion history, transport composition and simultaneous cost; no missing integration assigned to 'later' | One scene supports overhead and first-person views, required collisions, coherent light, source events and valid temporal inputs |
| I4 Whole-engine selectable profiles | Package native desktop/browser/mobile variants, feature removal and quality lifecycle; one development target slice accompanies I3a, bounded portable playback and save follow, full qualification follows I3b/I2b as appropriate | Specify every substitution, resource retirement, history invalidation, game-semantic invariant and offline/runtime product; browser authoring is outside first SDK, browser play/cache/save are included | Profiles preserve semantics and satisfy measured footprint/frame/startup/lifecycle limits on named devices; live streaming only where separately admitted |
| I5 Advanced and distinctive capabilities | Research-derived enhancements to lighting, geometry, environment, animation or quality allocation | Establish prior art/gap and full compatible design; baseline remains usable; no novelty claim without comparison | Advance improves a required cue/cost under fair content and full-scene evidence, without regressions elsewhere |

I2n is part of the core construction sequence alongside I3a and before broad I3b integration; it consumes I2a's
small identity/content path, not all of I2b's streaming/editor work. I3a first runs locally, then its network variant
exercises the same public actions. The bounded in-memory authority/replica fixture starts in I1/I2a, before real
transport. Production online services remain optional. The first SDK may ship the local game before the network
variant, but I2n is a committed near-term vertical path and its contracts cannot be postponed or redesigned later.

Work may overlap where contracts are settled, but integration checkpoints are sequential and owned. D1/D2 authors
exchange interface changes continuously. Do not fully implement one module while its coupled representation remains
undecided. The first assembled scene reduces content count, not the interactions that the architecture must support.

## Capability survey integration

The [official-page capability survey](CAPABILITY-SURVEY.md) records twelve engines and H01–H12 dispositions.
Integrate them into the packages above; do not launch a parallel feature backlog. N1 now selects first-class future
multiplayer, with server authority and optional replication. D0 must name device candidates with separate authoring OS,
build host and player/backend support. None of those choices
is inferred from a competitor's platform list. Preserve the user-selected camera/platform ambitions.

D1 owns dynamic asset roots/families and external-content admission, localization production formats and the edit-type
matrix with A01/A02; existing offline/startup/residency distinctions become creator-facing reports. D2 connects
imported material support and shader demand with A03, plus animation/sequence inspection. I2b/I3b provide selected
character, imported-world and style authoring paths using shared source/operations. I4 qualifies final artifact
inclusion/footprint, selected locales, mixed UI capture and real target bridges. Locale-specific voice/art production
and optional haptics follow selected game needs. Full node editors, native blockout/sculpt tools, timeline/movie export,
runtime scripting and arbitrary binary reload do not gate the first complete game.

## AI performance infrastructure

The [performance design](AI-PERFORMANCE.md) makes optimization a deliberate contributor capability. D0 selects the
first real workload/device, outcome and budgets; D1 specifies objective/evaluator/result identities, isolation and
resource limits. I1 includes a scoped CPU/headless measurement and correctness path with repeatable scale/churn workloads,
operation/allocation/byte attribution and A/A noise calibration before timing-based claims; A/B comparisons begin
as meaningful alternatives exist. A full autonomous search service is not an I1 prerequisite. D2/I2a/I2b/I3 add selected
content/GPU/coupled evaluations and A03 product identity. I4 supplies independent
target and sustained qualification. Start campaigns as correct executable slices become available; do not wait for
every engine module or promise gains from an unbuilt target. The selected A04/A05 compatibility rules govern dependent environmental claims; the selected detailed operators now appear in PLAY-ENVIRONMENT-SPEC; qualification remains required.

Broad search follows verified evaluation. The coordinator commissions algorithms/representations/schedules/content
changes with shared budgets and consumer ownership, then integrates candidates sequentially. Runtime features,
semantics and quality cannot be silently removed to meet a timing target. This does not start hardware/cloud spending,
install an optimizer, or alter the current game's test suite.

The [four campaign walkthroughs](PERFORMANCE-WALKTHROUGHS.md) are prospective acceptance cases for this infrastructure.
D1 must include meaningful completion events, unfinished-work/freshness reporting and consumer/resource handoffs in
the commission/results. D1/D2 owners finish the listed bounds, cache/lease, first-play and route-deadline mechanisms
before their dependent campaign. T2 qualifies compatible combined revisions and records the scope of each improvement;
the paper targets select no device floors and supply no measurement evidence.

## Asset promotion and storage discipline

The [storage lifecycle](STORAGE-LIFECYCLE.md) makes authoring/build disk usage a planned resource. D1 assigns F4 source,
product and release roots; T1 exact selection/publication status; T2 run identity, reservations, lease reconciliation
and retention reports. Specify crash and collector/publication races before automatic cleanup. I1 starts with bounded
local worker roots, inventory and dry-run ownership reports; admission must work before autonomous bulk campaigns.
I2a proves one selected asset's cold restoration, failed publication and retirement of only proven disposable
candidates; I2b adds two cooked profiles and broader region products. Share immutable products where supported; keep
mutable builds private and reusable.
Record selection and decisive evidence without retaining every trial workspace. Existing worktree recovery and app
conversation retention remain separate operational tasks, not implicit deletion steps in engine construction.

## Network-aware foundation gate

I1/I2a use scoped worlds, typed actor intents and authority-owned side effects even in the local-only slice. Before
claiming these APIs ready for network games, implement a bounded in-memory two-peer fixture under [N1](modules/NETWORKING.md):
server/replica separation, mixed accepted/rejected action outcomes, stale/duplicate inputs, atomic spawn/first state,
interest leave/re-enter, coherent baseline and missing gameplay content, distinct private saves and server build
pinning. This fits the small substrate/world path and does not make production online services a prerequisite for
I3a. I2n then qualifies real transports, join/reconnect, loss/delay/overload and effect deduplication against the
[R4 recipe and playable/degraded latency classes](NETWORK-SPEC.md#r4-encoded-admission-fixture). A game that selects prediction must qualify its
complete closure; the first network game need not select it.

## First complete reference scene specification

The I3b scene below is the first broad integration workload after I3a, not the only developer scenario. After relevant construction, use the
[S01 board game, S05 flood game and S09 shared-character workflow](SCENARIO-WALKTHROUGHS.md) to exercise ordinary
creation, physical-versus-visual scaling and target reuse. S06 adds media-composition qualification when implemented.
Their current traces are analytical design checks; none is an implemented sample or a measured usability result.

The [first-use pass](FIRST-USE-WALKTHROUGHS.md) adds a public starter gate: I3 supplies one complete
action/rule/state/feedback path, a meaningful edit, a useful failure and restart; the relevant I4 target slice supplies
prerequisites and qualified export. Keep example/docs version-matched. Replace/delete sample content and rules without
changing the setup/build model. D1 must specify source/schema/registration without redundant manual manifests. I2b
covers stable import mapping, authored overrides and conflict operations; I3 exposes these through the minimum visual
workbench before claiming artist usability. I4 qualifies additional targets independently. A plain library-backed
rule uses direct game code; a specialist navigation adapter fixture follows the relevant P5/content contracts. Custom
water/media integration examples must implement the selected A03–A05 rules and complete their operator-specific design. These are owner-specific usability obligations, not another
parallel engine framework.

The [from-scratch review](FROM-SCRATCH-WALKTHROUGHS.md) adds an independent empty-project path alongside the starter.
D1 distinguishes valid draft source from selected target readiness and specifies guided/direct setup with the same
public registration. I1 exercises a headless entry with no GPU; I2a allows empty drafting and inspectable source
construction; I3 constructs the first rendered action/state/result path without pre-existing sample content. Workbench
helpers must not mask absent game cameras/lights. The relevant I4 slice supplies a development export and recipient
handoff; full release/device/visual qualification remains a separate claim. Compare the same small game for newcomer
and veteran, then qualify artist handoff and cold-source-rebuild paths as their capabilities ship.

Use a small two-region environment with one controllable skinned character, a moving/supporting body, opaque/masked
and transparent surfaces, directional and local lights, some instanced detail, spatial sound and focusable UI. Include
one environmental field consumed by both appearance and another system (for example wind by vegetation and audio,
or precipitation by surface response and sound). This is a workload specification, not a water-first requirement.

Provide orthographic/isometric and perspective views of the same committed world; temporarily retain a skipped second
view. Include a streaming boundary, editor operation, save/reload, optional visible environment removal, quality switch
and suspend/resume during pending content work. The artifact records content hashes, camera routes, intended cues,
model predictions, supported approximation limits and whole-scene resource counts before performance claims.

## Spatial authoring and SDK release gates

Before I3 artist-usability claims, exercise import → place/rotate/scale → duplicate/instance → actual game camera →
undo → preserve-world reparent (including rejected shear) → reimport with placement/material overrides retained →
target build. T1 uses the same typed source operations as CLI/agents. No full modeling suite is implied.

The first SDK supports games pinned to one release and its exact tools/examples/skills. Before distribution, state
release availability and support limits and retain the promised source/product/save restore closure. No automatic
upgrade, stable binary ABI or cross-release compatibility is promised. If an upgrade is offered later, preflight a copy,
retain old roots, declare required code edits/recook/migrations and qualify an ordinary game, extension and save including
abort/rollback before making that claim. This gate does not delay the first local single-release game.

## Existing code reconciliation and reuse

Preserve the current `engine/rebuild` and other active worktrees. Inventory candidates by exact revision and public
contract after the target is reasoned, using the module reconciliation skill. Classify each as reuse unchanged, adapt,
replace or defer, with evidence and dependency cost. Existing rendering code already has a wgpu boundary; that is a
candidate integration advantage, not validation of the whole renderer or permission to transplant all its policies.

Bevy may contribute ideas about schedules, entity generations, table/sparse storage, scenes, reflection, extraction
and feature composition. Study its actual source/docs and alternatives; do not copy by resemblance or dependency name.
Copied components need exact origin/license/notices and a maintained adaptation boundary. Prefer standalone mature
libraries for expensive commodity functions where their supported targets and model satisfy the contracts. No code
has been copied or vendored during this design commission.

## Evidence and release

The [AI development plan](AI-DEVELOPMENT.md) is part of the ordinary tooling path. D1/I1 delivers pinned contributor
setup, public API/dependency boundaries, selected compiler/lint checks and headless fixtures. D1/I2a closes A01/A02 for
source/session mutations and supplies discovery, bounded queries and structured check/job results; A03 gates the
supported cooked render path. I3 adds actual input routing, state/capture/restart and version-matched SDK examples.
Write contribution/game/diagnosis skills against real commands as those paths ship, then run fresh-agent behavioral
tasks. Relevant I4 slices verify clean source and recipient handoffs. Shared-contract changes have one coordinating
owner; validate the integrated revision after parallel work. No skill, linter pass or JSON report alone promotes a
runtime, visual or platform maturity claim.

Qualification covers functionality, stability in motion, approximation error, actual disabled costs, CPU/GPU tails,
memory peaks, download/startup and sustained mobile thermal/lifecycle behaviour. Fix the input/content/profile and
capture provenance. Keep authored/artistic choices distinct from physical model claims. User visual acceptance applies
only to the reviewed build/camera/profile, and store packaging success is not store approval.

Promote maturity per [MATURITY](MATURITY.md), not by completing a checklist in one subsystem. A complete required
dependency closure must run together before a profile is qualified. If observations contradict the predicted model,
diagnose its assumptions/implementation and revise affected contracts before layering dependent features on top.

## Lifecycle usability proofs

Use the [four lifecycle walkthroughs](LIFECYCLE-WALKTHROUGHS.md) in the existing D1/I1/I2a/I2b packages: offline local
asset iteration with honest recovery state; stale source/delivery export rejection; independent game selections
and rollback bytes excluded from new builds; integrated campaign handoff retaining dirty/generated inputs while
retiring owned losing bulk. Distinguish available manual operations from unimplemented publication/lease/admission
tools. Prove the small path first and keep creator records proportional; these cases add no parallel build queue.
