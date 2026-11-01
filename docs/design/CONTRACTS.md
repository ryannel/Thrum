# Shared engine contracts

Revision C4, 1 November 2026. Reconciled source, iteration and multiplayer baseline; not implementation-qualified.
[Decision record](ARCHITECTURE-DECISIONS.md) resolves A01–A05/J01–J04 and integrates the whole-plan review.
All domain authors read this contract set and the [brief](ARCHITECTURE-BRIEF.md). Propose changes with affected owners;
do not create a private alternative. The coordinator reconciles both producer and consumers before accepting a change.
[The baseline specification](BASELINE-SPEC.md) fixes the initial admitted implementations and numerical envelopes.
This document owns cross-module invariants; the five baseline specifications own selected algorithms and adapter
policies; module designs own responsibility and consumer consequences. These are responsibilities, not
a demand for separate crates or a universal messaging framework.

## C01 Identity, units and coordinate frames — F3 with W1

Stable authored/save identity is distinct from a transient runtime slot/generation and from a content revision/hash.
Reuse of a runtime slot never makes an old reference valid. Cross-thread results carry target identity and generation.
Object destruction is a committed operation; listeners cannot revive a retired generation by publishing late work.

Recommend metres, seconds, kilograms and radians for physical interfaces; explicitly name artistic normalized controls.
Recommend one right-handed world convention with Y up, with import adapters performing coordinate/unit conversion once.
Local GPU coordinates are f32 relative to a declared origin; persistent spatial identity and origin changes do not
depend on the camera. Exact large-world representation remains a bounded W1 design choice, not a duplicate coordinate
system per subsystem. A rebase changes representations, not physical positions, velocities or accumulated time.

Authored SourceRevision, target BuildRevision, local SessionId/WorldEpoch and shared network MatchId are distinct.
A build pins an immutable admitted source tree and product closure. A play run pins authoritative code/schema/gameplay content and an admitted visual transition set; compatible
presentation generations may advance under C04/C06. It gets fresh local epochs; peers share a
match identity, never local runtime slots. Public W1 handles do not expose the selected private hecs storage backend.
Authored local transforms are TRS. Preserve-world reparenting must reject unrepresentable shear, singular/ill-conditioned
parents and excessive recomposition error. [FOUNDATION-SPEC](FOUNDATION-SPEC.md) bounds linear and absolute spatial error over the
selected extent; no silently lossy decomposition. Derived world matrices may have shear, with explicit consumer support.

Preserve-world authoring evaluates old and candidate-parent world matrices from the operation’s expected admitted
SourceRevision, never a runtime/play pose or unadmitted working-file edit. Cycle, conditioning, TRS representability
and affected-consumer checks validate the candidate before file application and admission. Working-file divergence
requires explicit sync/rebase; it is not an implicit transform input. Rejection leaves the admitted source and running
world unchanged. FOUNDATION-SPEC owns the proposal/application/admission outcome and recovery protocol.

The source/storage adapter follows FOUNDATION-SPEC. Ordinary files are the editable project; immutable admitted
snapshots are exact build/recovery inputs. Tool edits apply to files before admission. Native authoring and browser
runtime cache/save adapters have different scope; a full browser editor/source-authority store is not initial SDK work.

## C02 Time and committed state — F1 coordinator, W1 world

Each authoritative world advances once per its fixed tick: X[n+1] = F(X[n], ordered commands[n], dt). A render view cannot invoke F.
Simulation-affecting content/region/profile changes commit at the pre-tick barrier before queries against X[n].
Physics/gameplay/environment then step and publish X[n+1]. Presentation-only products may publish before extraction
with their own revision; they must not change the authoritative state or swap required collision/nav semantics late.
Initial runs preload their bounded gameplay closure. Substantive source edits start a new run; level transitions use
an explicit stop/load boundary. The barrier invariant does not require live whole-world replacement or provider resume
code. A later live streaming recipe must specify scoped staging, consumer commit and peak admission before selection.
Selected default timestep is 1/60 s, configurable for game/solver stability requirements; substeps are internal to an
explicit solver schedule and do not create another world clock. Rendering and replication cadences are separate.
This is an engineering cadence, not a measured cost. Local play, server, replica and speculative worlds have distinct
roles and epochs; replicas publish admitted server state, while prediction runs only its declared capture/replay closure.

Presentation reads immutable committed state, optionally interpolating adjacent snapshots with alpha in [0,1].
Interpolation is defined per quantity: positions/rotations may interpolate; events, destruction and inventory changes
do not blend. Render interpolation adds latency that must be accounted for in control/camera policy. Replay records
inputs, ordering and required nondeterministic results; fixed dt alone does not promise cross-platform bit identity.
Suspend/long gaps follow explicit pause/catch-up policy with a work cap; never apply hidden wall-clock elapsed time
unboundedly. Every asynchronous result has an epoch/version check before publication.

F1 owns a bounded publication pool per world with whole-process admission. W1 publishes a `SnapshotId(world_epoch,tick)` **immutable read-model bundle**:
copied or copy-on-write pages, identified events and versioned provider-sample/product handles. Published pages never
alias arrays that the next physics/world step can mutate. Registered consumers acquire `SnapshotLease` and required
CPU product/provider leases; raw borrowed references cannot outlive them. Baseline P3 queries execute within the
pre/post-step stage and return owned results; async navigation uses leased immutable tile/obstacle snapshots.

Retain adjacent tick bundles for interpolation. Per-view prior deformation/effect samples have separately bounded
retention; they do not pin arbitrary whole-world epochs forever. V2 extraction creates an owned `RenderPacket` with
staging/GPU resource leases. Release the world snapshot only when CPU extraction and required copies finish; GPU
submission completion governs the packet/resources independently. Region retirement cannot free leased old data.

Before publishing beyond the pool's count/byte admission cap, cancel/defer optional unsubmitted stale-view work and
invalidate its histories, then await actual lease release. Never reuse in-use pages. A required reader that cannot
finish causes backpressure or the explicit overload state, not unbounded retention or silent tick loss. A skipped
view outside its retained sample horizon resets history and resumes from a valid snapshot. Exact page layout and
caps are detailed parameters; these lifetime rules are mandatory for the design.
For an authority that promises admitted-input progress under competing pool use, each accepted input protects its
future rule-created immutable output bytes separately from snapshot and group storage. Rule callbacks spend only
their own preadmitted output allowance; pre-existing charged owners may be shared without another byte charge.
An exhausted declared allowance is a terminal rule-contract fault before that action's edits, while ordinary reader
backpressure remains retryable before the tick starts.
An admitted structural-growth profile declares separate finite physical-slot and live-object caps. A pure typed-action
classifier supplies a maximum spawn count before sequencing; the authority checks prepared spawns against the stored
bound at execution. A new input that raises the possible page/root high-water must atomically fund every older pending
input's increased future credit before its own sequence advances. Due preflight uses that admitted absolute horizon;
clean idle ticks do not dirty pages merely because a future spawn is pending. Stable-ID-keyed lifecycle changes retain
distinct despawn/spawn records even when one physical slot is reused before publication. A required replica's object
headroom is part of admission, and local and multiplayer submissions use the same classifier and rule callbacks.

## C03 Authority and interaction ordering — W1 and P3/P4/P6

Game/AI generates intent; a movement policy requests motion; physics owns resolved body pose/velocity; animation owns
the skeletal pose. Each body's mode selects its transform authority (static, kinematic, dynamic, animation-driven).
Root motion contributes a displacement request before physics; it does not apply a second world transform afterward.
Ragdoll changes authority explicitly. Attachments consume the committed parent pose with a declared tick/render time.
P3 returns achieved displacement/contact constraints and residual motion to P4/P6. Foot locking and gait adaptation
respond to that result rather than independently moving the body. Pre-step queries name X[n]; resolved contacts name
X[n+1]. Async navigation/query results carry world, region and obstacle generations and are rejected when stale.
Authoritative contact/event IDs emit once; render interpolation cannot emit duplicate gameplay or audio events.

Feedback has a defined causal boundary: derive forces/constraints from a stated state, integrate, publish contacts and
new state, then schedule dependent reactions. Same-step iterative coupling belongs inside one declared solver with
convergence/iteration policy. No arbitrary event callback may create an implicit circular solver across modules.

Standalone/server authority alone commits gameplay consequences. Actor control is permission to submit validated
intent, not arbitrary state mutation. Confirmed replica state and opt-in prediction remain distinct. Irreversible
side effects execute only on authority, with stable event/outcome identity; C13 defines network admission/correction.

## C04 Feature composition and quality — F2

A profile selects capabilities, providers and budgets independently. A provider declares required inputs, outputs,
valid regimes, lifecycle and allowed substitutes. Validate explicit typed setup and the selected finite target recipe before activation; no automatic provider search
chooses physical laws. Optional packages use the same contracts as direct game systems. Disabling visible
clouds may retain a cloud-attenuation provider; disabling all attenuation supplies the explicitly clear value 1.
No consumer silently finds a global singleton to obtain an absent provider.

Build exclusion, load-time selection and live changes are separate guarantees. Live changes are transactions: validate
dependencies and memory headroom, prepare replacements, commit a new generation at a safe boundary, invalidate derived
state and retire old resources. Unsupported changes fail without partially changing the running profile. Visual quality
may reduce samples/resolution/representation; changing authoritative gameplay resolution requires game-level consent.

Renderer extension covers material/style closures, typed passes and providers under V1–V4; headless inclusion is
independent. Arbitrary full/live renderer replacement is not a baseline promise. Physical substitutions must preserve
declared game semantics. Unsupported water edges or media overlaps fail admission rather than becoming visual blends.

If appearance conveys gameplay-significant information, F2 constrains per-client visual fallbacks to a declared cue/error
envelope or rejects the tier. Cosmetic fog may differ; concealment/privacy never relies on client fog pixels. N1
interest/authorization and P6 visibility facts remain authoritative and independent of local rendering.

## C05 Assets, streaming and retirement — F4 with W2, V1 and P7

One manifest maps logical assets and revisions to dependency-hashed runtime products. Import/cook reads the admitted immutable source revision and is separate from
runtime loading. Filesystem watcher events cannot activate a partly written external change set. F4 owns byte/decode/derived-product availability and resource leases; W2 owns region activation;
physics/navigation/render/audio own their representations. A region becomes active only when required products are
ready, or through an explicit fallback policy. Optional presentation placeholders cannot impersonate missing collision.
W2 stages the save overlay and required collision/navigation/gameplay representations off-world, validates epochs,
hashes and memory admission, then commits them together before the tick and opens traversal portals last. Retirement
closes new entry first; active contacts, swept body support, joints and navigation traversals pin required products
until resolved. If preparation or pinned residency cannot fit, apply an explicit blocked/loading transition; never
remove the floor beneath an active body. Paths carry region/tile/obstacle generations and invalidate after changes.

Substantive shared-prefab/gameplay edits build a new run initially. A qualified visual preview computes the entire
affected presentation closure, reserves/prepares replacements and publishes at extraction without changing gameplay.
For a later live gameplay/region recipe, recheck the active-set generation and include newly active dependents or
abort/reprepare; manifest and required consumer representations must publish together. Inactive regions retain explicit
base/overlay ancestry. These are correctness invariants, not an obligation to implement whole-world live replacement.
Initial bounded worlds and stop/load transitions use FOUNDATION-SPEC's simpler construction and retirement path.

Budget peak residency, not just steady state:
M_peak = M_live + M_pending_decode + M_upload_staging + M_replacement_overlap + M_transient + M_reserve.
Admission control defers new work before this bound exceeds the selected budget. Do not double count aliased or
shared/unified allocations; retain separate measured physical, allocator-estimated and logical byte accounts.
Retire a resource only after all CPU leases and relevant GPU submissions have completed. Eviction removes derived
products, not durable identity or saved state. Disk/browser cache entries are optional accelerators, not authority.
CPU snapshot/provider leases and GPU packet/fence leases are separate references in this accounting. Cancellation
requests do not prove a reader has stopped; reclamation waits for its lease to be released.

Shader modules and pipeline descriptors are ordinary dependency-hashed F4 products. F2 enumerates reachable tuples
of material, geometry, pass, provider and target plus admitted transitions; dynamic content declares a finite envelope
or a new admission. Keys include source/includes, compiler/schema, target features and descriptor. Unneeded combinations
may be stripped only outside this closure. Driver/device caches are disposable; they are not portable cooked authority.

## C06 GPU execution and lifetime — V1

Rendering modules declare passes and resource reads/writes; one GPU executor owns hazard ordering, allocation and
submission. CPU system scheduling is not a substitute for GPU resource ordering. A dependency A -> B exists for an
overlapping resource use with a write; last-use information bounds transient reuse. Persistent histories/caches have
explicit owners, versions and invalidation; pass removal does not alone release them.

One platform surface lease and device generation govern valid GPU work. Surface replacement and device loss are
distinct events. Lost-device resources are recreated from retained/cooked inputs through F4; old-generation callbacks
cannot publish live handles. Main-thread requirements belong to the platform adapter. No feature privately creates
another device or submits background work outside the coordinator's lifecycle policy.

V1 prepares required pipelines for the current device generation before local view readiness. Missing required work
means bounded loading/failure, not hidden first-draw compilation or silent shadow loss. Optional fallback must already
be declared, cooked and prepared. Profile changes prepare/reserve old-new overlap before extraction-barrier publication.
Use async preparation only where actually supported. Device loss invalidates pipelines, not the F4 source manifest.
A synchronized network replica and its local GPU-ready view are separate states; one slow client cannot pause authority.

## C07 Render scene, views and temporal data — V2 with P4/V3/V4

W1 owns world state. V2 owns derived draw instances, visibility and render spatial indexes. Views own projection,
viewport, origin-relative transforms, history identity and budget. Shadow/reflection visibility may include objects
outside the main camera. Geometry producers provide consistent current/previous deformation or explicit invalidity;
V2 converts these into the agreed motion convention. Static cameras do not imply zero motion for animated geometry.

For each view/provider, retain the sample time and transforms of the image actually used as history. This is not
necessarily tick n-1, the preceding host frame, or the last OS-displayed image. P4/E1/V5 and material animation own
evaluable prior deformation/parameters at that history sample time, or declare invalid correspondence. Skipped views
need bounded retained data, analytic reevaluation with preserved parameters, or reset. V2 projects the corresponding
material point through unjittered current/history transforms. Recommended vector convention is history-pixel minus
current-pixel, top-left origin with X right/Y down, in current render-target pixels; resize resets by default unless
a provider explicitly converts both sample grids. Teleport, changed topology/LOD or missing prior pose cannot use a
fabricated rigid vector.

Each history key includes view, provider, resolution and relevant content/device/profile epochs. Reset or reproject
only under documented validity conditions. A camera cut cannot retain old temporal samples as if it were smooth motion.
V4 owns image reconstruction/exposure/display; renderer features supply semantic inputs, not private rival upscalers.
V3/E1/V5 own their effect-specific histories; V1 owns persistent GPU lifetime. Shared invalidation reaches all affected
owners. Detailed derivation is in the [temporal composition study](research/TEMPORAL-COMPOSITION.md).
History retention does not bypass C02's snapshot budget. Extracted prior poses or leased products have explicit caps;
an old/skipped view without valid retained data resets rather than borrowing reclaimed world memory.

Network correction and prediction rollback invalidate affected geometry, water, atmosphere, effects and display
histories through the same owner-directed mechanism; resetting V4 alone is insufficient.

## C08 Light, materials and transport — V3 with E1 and V4

One scene light registry defines source geometry, spectrum/colour/intensity convention and transform/time. Environment
providers supply illumination and attenuation; drawing a sky is optional. Materials expose a selected shading closure
and surface properties; a style policy may alter that closure explicitly rather than hide multipliers in every feature.
Linear scene-referred colour is kept separate from display transforms; V4 owns exposure/tone mapping/output encoding.
W1 owns authored light identity/source descriptors and E1 owns its celestial/environment drivers. V3 owns their
render transport evaluation. A headless physical surface model uses a declared E1 irradiance/thermal provider and
shared source facts/exposure data; it never reads rendered pixels as the authority for heat or weather.

Distinguish incoming radiance, irradiance and dimensionless transmittance. Do not sum quantities with incompatible
units or apply the same shadow/air attenuation twice. A transport query specifies its path/segment and contributing
media. For a segment, radiance transforms as L_out = T * L_in + S, where T is transmittance and S in-scattered radiance.
Composition order and overlapping media require one agreed integrator/approximation, not feature-local screen blends.
Rays crossing water use a declared medium boundary; caustics are an illumination contribution with an explicit owner.

V3 owns transport semantics and depth ordering; E1/V5 provide typed media/surface inputs. V1 validates and executes the
resulting pass/resource graph. V4 receives composed scene-linear colour and reconstructs/displays it; it does not own
physical media integration. Admit preintegrated `(T,S)` pairs only for ordered disjoint segments; overlap requires coefficient/phase/emission
access to one joint integrator, or one owner jointly preintegrating the entire overlapping set. Independent overlapping
whole-segment pairs lack the needed spatial information and cannot be combined exactly. A separate approximate layering
policy must state its validity/error. Sum extinction and evaluate source terms under joint incident illumination;
summing independently lit source terms is insufficient. Suppress duplicated constituent attenuation/integration. Disjoint near/far intervals compose as
(T_near*T_far, S_near + T_near*S_far). Transparent reflection/refraction and its background generally cannot share one
exact depth/motion pair; the material/profile selects an explicit layer/confidence/reactivity policy. Approximate
transparency remains labelled and does not silently alter the transport contract.

Even disjoint `(T,S)` segments compose exactly only relative to a shared declared transport closure: each source
term must use incident lighting consistent with the combined scene, including other media on its light paths.
View-ray disjointness alone does not justify independently lit preintegration. Otherwise recompute jointly or
declare the lighting approximation and its validity.

## C09 Environmental fields and physical exchange — E1 with P3

Weather/wind, atmosphere, cloud coverage, surface response, vegetation and water are separate providers with shared
position/time semantics. Every sampled field states units, support, interpolation, validity and represented scale.
Authored constant providers are valid alternatives to dynamic simulation. Render-only detail cannot silently become
a physical contact surface.

Phase changes conserve the represented material: melting transfers snow/ice into liquid storage; freezing reverses
that transfer. Evaporation/runoff are named exchanges with other domains or declared sinks. A cosmetic decay field
is allowed as an authored approximation, but cannot claim the mass balance of the physical surface-state provider.

Baseline coupling samples surface state X[n] for P3 contact/traction during tick n -> n+1. E1 advances surface
reservoirs using tick-n forcing, then publishes X[n+1] with the next world commit. Contacts/deposition produced by
that step enter the next explicit update, once by event/flux ID. Appearance reads/interpolates committed state; the
one-tick coupling lag is declared and must fit the game's traction-response tolerance. Tighter coupling requires one
specified substep solver/order, not callbacks that let P3 see a partly updated surface. E1 owns prescribed thermal
drivers or the chosen surface energy model; source absence selects an authored temperature/rate fallback without an
energy-accuracy claim. Runoff goes to named receiving surface/water regions or an explicitly exported boundary sink.

Water owns surface/flow queries and fluid response at its selected scale; rigid-body physics owns body integration.
The force model names the displacement/pressure/drag approximation and sample state; body reactions return identified
impulses/disturbances once, at a specified tick boundary. Wind is sampled from one field; foliage and waves consume it
through their own response operators. Surface wetness owns accumulated state; precipitation is an input, not a second
wetness accumulator. Environmental coupling must explain its full causal loop before either side is built.

Water models occupy explicit physical domains independent of W2 stream cells or peer interest. Each sample selects one
physical query owner with support/time/validity; visual seam blending never defines flux or body contact. Domain edges
are closed, prescribed reservoir, visual-only, or a specifically derived conservative coupling. Initial admission
includes the first three; unsupported heterogeneous exchange rejects the requested physical behavior. Finite-domain
volume change accounts for integrated signed boundary flux and sources/sinks. A conservative edge uses one common-tick
flux register with opposite signs; momentum/energy/traction need their own specified operator and stability limits.
An imposed sea level may drive a river without river discharge changing the sea: this is external reservoir exchange,
not global conservation. Stateful solver support/forcing stays resident or transfers by a proved policy.

Baseline physical queries are CPU-owned or analytically reproducible headless. GPU visual residuals have bounded
shore/contact error. Saves and semantic network manifests include required model/domain/edge/forcing revisions.
Water-contact prediction is off unless the full P3/E1 state, dependencies and once-only disturbance ledger are replayable.

## C10 Persistence, reproducibility and authored operations — P7/F3/T1

Save schemas contain durable identities and authoritative state, not GPU handles, cache pointers or runtime entity slots.
P7 requests an immutable save-state capture at a committed barrier, including provider-owned durable fields. This is
a separate purpose-specific capture, not an assumption that the render read-model contains the full save. Its bytes
and serialization-reader leases participate in memory admission; background serialization cannot read mutable solvers.
Game save compatibility and asset/tool schema evolution are versioned policies. Checkpoint at a committed boundary;
restore content dependencies before publishing the world. Replays declare their determinism envelope and captured
external results. Tests distinguish reproducible world state from asynchronous presentation.

Ordinary versioned files and masters are the editable project. A T1 operation produces a reviewable patch against
an expected admitted source revision. Coordinated file application precedes immutable snapshot admission: no private
accepted edit may silently become a run's source while the working files still show the old edit. A watcher reports
pending changes; it never changes a running build. Direct file edits and tool edits enter the same admission boundary.
Dirty capture requires the initiator's declared completed set; repeated scans cannot prove a coherent instant against
arbitrary noncooperating writers. On uncertainty, preserve files, proposal and last admitted root and report conflict.

Admission verifies exact bytes and dependency closure, then atomically publishes the authoring root and retry outcome
with a generation precondition. **Accepted** means admitted source durability under the adapter's stated guarantee;
file application alone is not acceptance, and acceptance is not cook success, play activation or save completion.
Builds read only admitted immutable bytes. A lost response returns the stored admission outcome for its valid retry
key; retired keys report expired. Undo and apply-back are new reviewable source operations, never a root rewind or
implicit copy of solver/earned state. FOUNDATION-SPEC owns file-writer handoff, interrupted-application recovery,
operation retention, storage transactions and GC. There is no automatic checkout mirror.

Use a separate durable root per authority: authoring snapshot/admission outcomes, each private play save, and each
released standalone/server save. A save root binds tick, base authored revision, build/manifest, authoritative overlay
and schema/provider versions. Prepare and verify its restore closure before atomic publication; a failed save preserves
the previous root. Never restore an old overlay against a newer admitted source. Replicas cannot publish authority saves.
Runtime may be ahead of its last checkpoint; periodic capture bounds that loss without pretending each frame is saved.

Play uses a pinned ready build in an isolated world/session, native child process by default, with fresh epochs and
private saves. Initial code, schema and gameplay-affecting data edits rebuild/restart; compatible visual preview is
allowed only for a qualified field/product set and its preparation/lifetime contract. A live server match stays pinned.
Apply-back explicitly merges marked authored fields against play base and current source; runtime handles, physics
internals and earned inventory are excluded. Stopping discards unretained test state, not authored files or retained saves.

Retain the exact products or qualified input/toolchain closure required by supported source/save/release roots.
Publication, readers and collection share pin/lease rules so a new root cannot reference collected bytes. Reserve old
valid data, candidate writes, retained restore closure and headroom before acceptance, counting shared content once.
Automatic cleanup cannot delete unique masters, selected revisions or another agent's live work. An evictable asset
cache is not a durable save. Browser runtime save durability/quota/export and native guarantees are declared separately;
full browser authoring is outside the initial SDK. Unsupported storage loss is diagnosed, never repaired with invented state.

Editor, CLI and agents share typed operations and queries. Expose working files, selected snapshot, running build,
saved checkpoint and one clear operation status; advanced diagnostics can expose generations/leases. Operations carry
identity, preconditions and correlation, with explicit retry and undo behavior. No agent-only mutation bypasses these
invariants. Disk lifecycle and campaign retention follow STORAGE-LIFECYCLE; the concrete source/save mechanisms follow
FOUNDATION-SPEC rather than being duplicated here.

## C11 Input, audio and UI lifecycle — P1/A1/U1 with F1

Platform events become game actions through focus/context mapping. UI consumption and game actions have deterministic
priority; touch gestures/controller/keyboard do not create separate gameplay implementations. Cameras consume intent
and committed body state under an explicit latency policy. Safe areas, text input, accessibility and localization are
part of UI layout/authoring, not renderer special cases.

Audio events reference durable source/event identity and simulation time; the audio clock remains distinct. Pauses,
seeks, restore and interruption define which events replay, cancel or remain continuous. Audio callbacks cannot block
on world locks, allocation-heavy asset loads or GPU completion. Listener policy is explicit for multiple views;
offscreen sound does not disappear with visual culling.

P6's event cursor is distinct from A1's playback bookkeeping. A1 tracks seen, queued, rendered-or-expired event IDs
with a separate audio-device epoch; it cannot know that a particular sample was physically heard. On live interruption,
already queued one-shots are consumed and do not replay by default; continuous ambience reconstructs from current state.
Replay/seek has its own explicit policy. The default is one selected listener per output mix, normally the primary
interactive camera; adding an editor/reflection view cannot silently move the listener. Acoustic queries distinguish
unknown/unloaded geometry from a clear path and use a named fallback.

F1 supplies audio route/interruption, gesture/focus, IME and safe-area events to the relevant adapters. Focused text/IME
suppresses only consumed bindings/context; a global game pause is game policy. Accessibility, localization and hit
testing consume U1's shared semantic/layout model, not a separate invisible agent UI. Platform accessibility bridges
and text composition behavior require native/browser-specific qualification.

Networked P1 actions route through N1 envelope validation and P6 authority checks; local listen-host inputs follow
the same path. Predicted cosmetic effects have stable input/effect keys reconciled with confirmed events. Replay
cannot repeat one-shots; already played audio cannot be undone. Corrections reset only according to explicit policy.

## C12 Budgets, evidence and acceptance — T2 with all owners

A frame is a dependency schedule, not the sum of every parallel job. Its lower bounds include the longest dependent
path and total work divided by available parallel capacity. Shared bandwidth, synchronization and queues can increase
actual cost. Authors supply cost variables and worst-case work caps; coordinator reconciles simultaneous consumers.
Quality allocation has hysteresis and bounded transition cost; it never oscillates providers every frame.

Evidence identifies code/design revision, configuration, content hashes, device/backend, duration and method. Design,
implementation, correctness, perceptual acceptance and platform/performance qualification are separate states. Human
appearance approval does not certify performance; a fast frame does not certify the complete feature. Existing branch
evidence is unassessed until its applicability to the new assembly is established.

[RENDER-SPEC](RENDER-SPEC.md), [PLAY-ENVIRONMENT-SPEC](PLAY-ENVIRONMENT-SPEC.md) and
[NETWORK-SPEC](NETWORK-SPEC.md) select the corresponding initial algorithms, limits and codec contracts. Targets
remain unmeasured; a future extension needs an equally complete design before changing these admitted semantics.

## C13 Network authority and replication — N1 with F1/W1/P6

[Network sessions](modules/NETWORKING.md) select server-authoritative simulation and typed actor intent; standalone
uses the same rules and local authority. A listen host adds local presentation, a dedicated host may omit it. Match,
wire, local world and durable identities have different scopes. Wire IDs never reuse within a match. Replicas and
optional prediction branches cannot commit authoritative gameplay or saves.

Pin the gameplay semantic manifest: rule/action meaning, tick/authority policy and required physical/query content.
Protocol identity covers the envelope and wire schemas. These are exact compatibility closures, not all local tool or
visual schemas. Each role names exact product provenance and compatible mappings; a nonpredicting browser replica can
use declared query projections without downloading the server's solver. Visual builds may differ. Source edits cannot
mutate a running match without an explicitly designed migration or restart.

N1 checks sender, size, epochs, ordering and action timing policy; P6 checks legality and owns committed outcomes.
Transport receipt does not imply gameplay execution. Each admitted input reaches a bounded committed/rejected result,
identified by connection/input origin and authority tick; result delivery, retention and overload behavior are specified
in NETWORK-SPEC. Prediction consumes the resolved prefix and individually resolved higher input IDs together with
the exact authoritative state/tick that represents them; a pending gap may hold back the prefix alone.
Confirmed events retain origin identity for speculative effect reconciliation. Reconnect takes a new baseline and
discards old pending input; uncertain one-shots are not retried automatically. Durable game transactions requiring
cross-reconnect retry need game/P7 operation identity and outcome, not a transport exactly-once promise.

Replication is a purpose-specific immutable projection, not render/save/ECS serialization. Each published group includes
its lifecycle changes and required dependent state atomically: no spawn/re-entry before its first usable state. Dependent
removal effects commit together. Missing required content keeps the group unadmitted, never defaulted. Interest exit
has its own generation and is distinct from destruction and authoritative region streaming. Optional water projections
pair a sample with its matching body pose/time inside the same admission unit. Full-group versus delta encoding is
adapter policy; the initial recipe and its simultaneous byte/cadence bounds live only in NETWORK-SPEC.

Slow peers cannot pin world snapshots or stall the authority. Bound per-peer and aggregate queue/history/work resources;
retain reliable results until application acknowledgement or explicitly resynchronize/disconnect before overflow.
Initial matches preload a finite authoritative gameplay closure. Client-local content/GPU loading is independent;
future server streaming must stage without a client-induced server pause. Networking-off removes sockets, replication
scans and prediction buffers. Adapter settings are profile limits, not universal engine limits.

Prediction is optional complete-state replay in a separate branch; confirmed state remains intact. Partial body/fluid
rollback is unsupported. Confirmed interpolation, outcomes and reconnect complete the first multiplayer path without
prediction. No browser raw-UDP, cross-platform bit identity or universal latency claim follows. Later adapters/providers
must preserve these authority/identity/content contracts and supply their own complete mechanisms and qualification.

## Active integration scenarios

The coordinator traces each scenario with owners before declaring the high-level design integrated:

| Scenario | Required handoff chain | Failure that must be prevented |
| --- | --- | --- |
| Animated character crosses a streaming boundary | P1/P5 -> P6/P4 root request -> P3 resolve -> W1 commit -> V2/P4 draw; W2/F4 activate geometry/nav/audio | Double motion, absent collision, old skeleton bounds or stale loaded content |
| Two cameras change quality | W1 once -> V2 per view -> V3 -> V4; F2 validates transition | Double world ticks, shared histories, duplicated environmental authority |
| Rain darkens a moving shoreline | E1 weather -> surface/water state -> P3 interaction -> V3 light/material -> V4 | Duplicate wetness, unrelated clocks, caustics/shadows counted twice |
| Region unload/save/edit/reload | T1 file patch/admission -> F4 cook -> new run or P7/W2 stop/load restore -> consumers | Lost changes, stale async publication, dangling physics/audio/GPU references |
| Multiplayer join/correct during source edit | T1/F4 pinned build -> N1 semantic handshake/baseline -> W1 replica -> P6 intent -> V2/E1/V4/A1 correction | Live-match mutation, mixed partial state, slow-client server stall, duplicated effects |
| Mobile suspend during upload | F1 lifecycle -> V1 submission stop -> F4 job/lease handling -> A1/P7 policy -> restore epoch | GPU work after surface invalidation, unbounded catch-up, corrupted save, repeated audio events |

Domain authors must identify contradictions, duplicate owners and missing inputs in these paths, including outside
their own files. Report a proposed resolution to the coordinator and affected peers; do not silently patch around it.
