# Selected architecture and review resolutions

1 November 2026. This records the coordinator's engineering selections after the independent decision audit,
whole-plan review and three coordinated Sol High decision studies. The user's additional requirement makes future
multiplayer first-class. These selections resolve structural choices; they do not certify detailed algorithms,
working software, device support or measured quality. [C4 contracts](CONTRACTS.md) govern the interfaces; module
documents govern their mechanisms. Earlier reviews remain evidence of the baseline they inspected.

## The core we are choosing

A game is ordinary typed code and authored scenes, assembled with explicit setup and a named target. It selects
the capabilities it needs. Optional packages help reuse; they do not create a universal plugin framework. One
authoritative world advances per simulation tick. Rendering, sound and networking consume appropriate committed
projections. The source build graph, CPU schedule and GPU pass graph remain separate because their dependencies
and lifetimes mean different things.

Own the engine's public identities, schemas, composition, world authority, content lifecycle, rendering policy and
creator operations. Select Rust, wgpu and platform-specific shells, using winit where appropriate. The W1
component-store choice is hecs 0.11.2 for V1 after the [three-candidate measured spike](decisions/A0-WORLD-STORAGE.md);
production A1 integration and target qualification remain open. Specialized physics,
pose, navigation and GPU stores remain with their owners. Do not build a second generic sparse ECS without measured
benefit. F1 scheduling and F3 schemas stay ours. A backend
handle or Rust type ID is never a public object, source, save or network identity. Compiler/library pins and target
qualification remain mandatory before construction uses these dependencies; selecting a library is not installing it.

This is a reusable 3D engine for desktop, browser demos and native mobile. Apple silicon Mac and Windows are the
initial desktop interpretations, WebGPU the browser profile, iOS/iPadOS and Android the mobile interpretation.
Linux/Intel coverage is deferred, not a claim of impossibility. Authoring OS, build host and player support are
separate. Native mobile packaging uses its required SDK/toolchain; browser demos do not substitute for app-store
delivery. Named qualification candidates and workload limits are selected in [the baseline specification](BASELINE-SPEC.md).
No minimum device or OS is certified by this document. The [single budget table](modules/PLATFORM-RUNTIME.md#whole-scene-budget-proposal--all-unmeasured)
remains an unmeasured engineering envelope. Use the named candidates for target-dependent implementation and qualification; do not turn their selection into a
certified hardware floor.

## A01, A02 and J04 — Source, play and iteration

**Choose isolated play sessions and immutable admitted source revisions.** Native authoring launches a child player
process by default; platforms without that facility use a logically isolated world instance and supported worker
boundary. Isolation is a lifecycle/authority guarantee, not a security sandbox. Each run pins its authoritative source/code/schema/gameplay manifest and admitted visual transition set, with fresh
world/session epochs and a private test-save namespace. Active presentation profile and compatible visual-product
manifest may advance transactionally at extraction; code/physical changes need a new run or explicit migration. Stop discards unretained
gameplay state and retires work/resources by lease. It never writes simulation results back into source implicitly.

**Choose file-first authoring:** ordinary files → explicit immutable snapshot admission → build/run.
Tools and agents propose reviewable patches against an admitted revision, apply them under a coordinated writer
handoff, and only then admit the captured bytes. A Git tree or declared finished dirty capture identifies inputs;
watchers report pending changes. No accepted tool edit is hidden behind unchanged working files. There is no automatic
checkout mirror. A noncooperating writer or branch drift produces a conflict/preserved proposal, not an overwrite.

Retain native SQLite as the internal immutable snapshot/outcome store: it solves durable admission and recovery without
adding another editable property format. Shared hashes avoid repeated copies across admitted revisions, but the extra
store copy relative to working files/LFS is explicit and budgeted. File application and durable admission are separate
failure boundaries; FOUNDATION-SPEC owns their exact recovery, retries, undo and retention. The source root advances
only with verified closure and an atomic outcome. Builds use admitted bytes, never partially written files.

Authoring, private play saves and released standalone/server saves have separate roots. Checkpoints bind the original
source/build/overlay tuple; newer source cannot silently reinterpret an old save. Browser play retains content cache
and player-save persistence policy. **Full browser authoring is not part of the first SDK**; native tools export the
same content formats for browser/native/mobile players. Authoring OS and player platform remain independent.

**Choose restart-first iteration.** Code, schema and gameplay-affecting edits build a new run. A failed build leaves
the last ready build identifiable. Qualified visual-only fields/products may preview with their own safe publication;
no binary hot reload or arbitrary live replacement is promised. Apply-back is an explicit three-way patch on marked
authored fields, never a copy of earned inventory, runtime IDs or solver internals. Server matches remain pinned.

Initial R0/R2/N1 gameplay worlds are bounded and preloaded; an explicit stop/load can change levels. Retire the old
world and leases before allocating a full replacement when overlap cannot fit. This removes the initial requirement
for every provider to serialize complete live resume state. The larger W2 streaming capability retains identity,
consumer-closure and commit invariants; a later live streaming recipe needs scoped staging and a derived peak budget.
It does not inherit implementation readiness from the bounded initial game.

**Why:** a file-first path keeps ordinary editing, review and agent branches understandable. Immutable admission
preserves exact builds and recovery. Restart handles substantive changes without a general live state-transfer system.
Keeping a narrow proven storage transaction mechanism is simpler than inventing a new cross-file durability protocol.

## J01–J03 — Composition, renderer boundaries and reuse

Choose direct typed registration and finite explicit target recipes. Validate required capabilities, schemas, access
and ordering before activation. Optional reusable packages use these same public surfaces. No dependency search that
chooses physical laws, universal service locator, binary plugin ABI or arbitrary live gameplay removal is selected.
Static inclusion, load-time choice and explicitly admitted live visual transitions are different guarantees.

Keep the clustered-forward baseline and a headless mode with no renderer. Expose material/style closures, geometry
and environment providers, typed pass hooks and backend adapters. A pass must obey the same resource, light, history
and lifetime contracts. Full renderer replacement and live renderer swaps are not baseline compatibility promises;
a materially different pipeline can be designed as a new qualified assembly. This supplies flexibility for game
styles and optional sky without maintaining two unrelated renderers or making every subsystem interchangeable.

hecs supplies component storage, not a host, scheduler or persistent schema. W1 wraps its short-lived handles with
independent world/slot/generation identities, retires public slots before generation wrap and sorts order-sensitive
work by stable identity. Structural changes occur at barriers; borrowed query views cannot escape them. Backend
clear/spawn-at resurrection paths are excluded. Activation validates/reserves before mutation; any remaining failure
needs a staged replacement or undo strategy before publication. No claim that hecs itself is transactional follows.
An owned dense/sparse ECS was rejected as initial maintenance cost without demonstrated need; standalone Bevy ECS
is a legitimate comparator, but a larger surface than this storage job requires.

## A03 — Shader demand is content, readiness is local

F2 declares the finite reachable material/geometry/pass/provider/target combinations and allowed transitions. F4
cooks their shader modules and pipeline descriptors in the existing dependency DAG. Enumerate reachable tuples,
not the Cartesian product of every knob. Values that do not change interfaces/layouts/pass structure or remove
material work remain uniforms or bounded branches. Dynamic content declares a finite family/envelope or triggers
new explicit admission; an unknown pipeline request is not permission to compile silently during drawing.

V1 prepares actual pipelines for the current device generation before a required view becomes ready. Driver caches
are disposable accelerators. Required missing products keep that view/region loading; an optional fallback must
already be declared, cooked and prepared. Profile switches reserve overlap, prepare replacements, publish at an
extraction barrier and retire the old leases. Device loss repeats local preparation. Async preparation uses actual
backend capabilities; no universally nonblocking creation or cross-API binary cache is promised.

Headless authority has no shader demand. A network replica can be synchronized while its local view is loading;
one client's GPU cannot pause the server. Per-target visual products may differ while gameplay compatibility matches.
Count/byte/preparation budgets reject excessive demand with its dominant axes visible, rather than quietly stripping
required shadows. [RENDER-SPEC](RENDER-SPEC.md) now fixes WGSL, the toolchain, binding ABI, material closure and readiness limits.

## A04 — Different water models have explicit physical boundaries

Choose authored disjoint physical domains, each with one query authority and declared model/units/support/validity.
An edge is closed, a prescribed reservoir boundary, visual-only adjacency, or a specifically derived conservative
coupling. Initial compatibility includes the first three; it does not pretend a generic conservative converter exists.
A shallow river can use prescribed tide at its mouth and account for exchange with an external sea reservoir. That
does not mean its discharge updates the sea. A decorative falls can connect separate named inlet/outlet reservoirs;
a game needing the receiving pool to rise from exact discharge must provide a coupled operator or suitable common
solver. Missing physical coupling rejects that requested behavior.

For a finite domain, volume change equals signed integrated boundary flux plus declared sources/sinks, within the
solver's stated residual. Coupled edges share one time-integrated flux register with opposite signs; momentum/traction,
energy and stability claims require their own operators. Substeps reconcile at the common committed tick. Blended
surface heights do not establish any of these laws. P3 integrates bodies from a named query snapshot; disturbances
return once under the declared causal schedule. Height fields do not answer unsupported volume/multi-interface queries.

Solver domains are independent of stream cells and peer interest. W2 pins required query/boundary support or uses a
proved transfer/reduced-state scheme; offscreen is not permission to discard live water. Save/replication compatibility
includes model, domain/edge topology, forcing and required state. Each domain's GPU state is its authority and water needs no exact replay, per the [water decision](decisions/E1-GPU-WATER.md).
Multiplayer shares water causes, not state; the authority computes forces, and a player's own vessel may be predicted
against local water fed by the same causes. Visual residuals have bounded contact/shoreline error.

**Why:** a universal solver imposes unsuitable cost and topology; automatic seam blending invents physics. Explicit
limited interfaces let simple water remain simple and reserve richer coupling for a complete mathematical design.

## A05 — Media composition respects what a representation retains

For disjoint near/far segments, use `(Tn Tf, Sn + Tn Sf)`. Overlapping independent whole-segment `(T,S)` pairs lose
spatial information and cannot generally produce the correct joint result. Admit exactly these forms:

1. Ordered, nonoverlapping preintegrated segments with declared support and validity.
2. Extinction/scattering/phase/emission fields evaluated by one joint V3 transport integrator.
3. One owner that jointly preintegrates the complete overlapping set, suppressing duplicate constituent evaluation.

A named approximate layering policy is possible with an explicit error/validity claim, never relabeled exact overlap.
For coefficient integration, sum extinction and evaluate source terms under the **joint incident illumination**.
Summing independently lit in-scattering still double-counts or misses attenuation. V3 owns transport; E1 supplies
fields; V1 executes; V4 displays. Background, ambient/reflection light, aerial extinction and cloud attenuation remain
separate requested outputs. No-sky games retain the outputs they actually use. [RENDER-SPEC](RENDER-SPEC.md) now chooses photometric Rec.709/D65, a joint single-scatter air/fog operator, sample
caps and numerical acceptance targets; richer media remain explicitly outside this initial provider.

Even disjoint `(T,S)` segments compose exactly only relative to a shared declared transport closure: each source
term must use incident lighting consistent with the combined scene, including other media on its light paths.
View-ray disjointness alone does not justify independently lit preintegration. Otherwise recompute jointly or
declare the lighting approximation and its validity.

## WP01 — Authored transforms remain TRS

Select local translation, proper quaternion rotation and signed scale; do not silently add affine/shear authoring.
For preserve-world reparenting, compute the candidate local matrix from the new parent's inverse, check conditioning,
decompose/recompose TRS, and reject if it cannot preserve the world transform within the declared numerical envelope.
The envelope bounds linear-map residual and absolute spatial error over the supported object extent; it cannot loosen
without bound at distant world coordinates. Ill-conditioned, singular or nonfinite cases reject. Error diagnostics
offer explicit preserve-local behavior, which intentionally changes world placement. [FOUNDATION-SPEC](FOUNDATION-SPEC.md) fixes the numerical envelope, deterministic decomposition, residual tests
and source operation schema. These are versioned admission rules, not artist knobs.

A rotated object under nonuniformly scaled ancestry can require shear and must fail this operation. Derived world
matrices may still contain shear even with TRS locals: rendering handles normals/parity, while physics/import consumers
must support, bake or reject their required transform regime. This is narrower and easier to explain than granting
every physics/animation consumer arbitrary authored affine transforms.

Preserve-world authoring evaluates old and candidate-parent world matrices from the operation’s expected admitted
SourceRevision, never a runtime/play pose or unadmitted working-file edit. Cycle, conditioning, TRS representability
and affected-consumer checks validate the candidate before file application and admission. Working-file divergence
requires explicit sync/rebase; it is not an implicit transform input. Rejection leaves the admitted source and running
world unchanged. FOUNDATION-SPEC owns the proposal/application/admission outcome and recovery protocol.

## Multiplayer — N1 is a first-class responsibility

Select server authority, typed actor intents and explicit replicated state. Standalone runs the same authority locally;
listen hosts and dedicated servers use that model. Each world has a scoped clock/epoch; wire identities and match IDs
are distinct from local runtime and durable IDs. Clients must satisfy the semantic protocol/physical manifest and their declared consumer requirements; local rendering
builds can differ. Exact server product hashes are provenance, not mandatory downloads of server solvers. Replica
query projections may suffice; local query/prediction products need explicit compatible mappings and exact own hashes. A match pins its authoritative gameplay build. Ordinary source edits
cannot mutate a live match: restart or an explicitly designed migration/re-handshake is required.

Baseline presentation interpolates confirmed state. Prediction is opt-in and requires a complete bounded dependency
closure, correction and effect deduplication. Do not promise universal rollback or cross-platform lockstep. Bound
queues/history, reject stale connections and expose missing content. Networking-off removes sockets, replication scans
and prediction buffers. [N1](modules/NETWORKING.md) owns join/reconnect, input admission, interest, snapshot grouping,
transport capabilities and failure behavior. Its first transport reference is reliable WebSocket-compatible messaging;
latency-sensitive datagrams remain a separately qualified adapter, not a browser UDP assumption.

## WP02–WP06 — Simpler delivery and honest maturity

Keep one construction sequence with an early complete R0 game: selected source, local authority, one action/rule,
one camera/material, save, inspect/restart and a clean recipient export. Only its required I1/I2/D2 subset precedes
I3a; full region streaming and editor machinery do not. Include an in-memory authority/replica fixture early and real
I2n alongside this small game, before broad I3b integration. Prediction, delta compression and water are not arbitrary
prerequisites; the first N1 encoding is selected against the concrete R4 byte/cadence recipe.

Thin iOS/Android development shells exercise boot, tick, surface recovery, suspend and input alongside the early
runtime work before dependent assumptions harden. Full thermal/device/accessibility/store qualification remains I4.
I3b later exercises richer play/presentation and explicit two-region transitions; live streaming has its own complete
recipe and admission proof rather than an implicit whole-world-copy implementation.

The first SDK is pinned-release-only: a game locks its SDK/tools/schema/content format and uses matching examples and
skills. No automatic upgrade, stable binary ABI, indefinite support window or cross-release save compatibility is
promised. Distribution retains the promised release/restore closure under its stated availability policy; private
local caches are insufficient. Before offering an upgrade, preflight on a copy, preserve old roots, declare manual API
edits/recook/save migration, and qualify an ordinary game, extension and save including abort. Never rewrite game code
silently. Establish release availability/support terms before external distribution, not before the first local slice.

Artist acceptance includes import, place/rotate/scale, duplicate/instance, actual game camera, undo, reparent including
the TRS rejection, reimport preserving placements/overrides, and target build. An inspector alone does not pass it.
Maturity queries must include applicable ancestor findings and shared-contract obligations, while retaining local
historical review evidence. Selected architecture is separate from detailed design, implementation and qualification.

## Detailed baseline and evidence

The user subsequently requested closure of the named remaining gaps. [BASELINE-SPEC](BASELINE-SPEC.md) itemises G01–G10
and selects concrete source/storage/transform, shader/colour/media, play/water, networking and target/workload rules.
Its four detailed specifications govern the admitted initial implementation. They replace the earlier open choices
above, not the larger engine's ownership model. The network profile includes real WSS, committed per-action outcomes, atomic lifecycle/state groups, exact gameplay
compatibility, bounded baselines and queues, and a concrete simultaneous R4 workload. Prediction and water event records
are optional extensions, with an explicit early I2n construction milestone.

Dependency resolution, executable APIs/conformance, recovery injection, measured performance and visual/device
qualification remain implementation evidence. Unsupported richer providers are recorded as extensions with design
obligations; they are not promised by the selected baseline. No experiments or builds were used to invent these rules.

Three decision studies and their cross-reviews informed this synthesis: [source/sessions](../evidence/handoffs/2026-11-01-engine-decisions/SESSIONS-SOURCE.md),
[shader/media](../evidence/handoffs/2026-11-01-engine-decisions/SHADERS-MEDIA.md),
[water domains](../evidence/handoffs/2026-11-01-engine-decisions/WATER-DOMAINS.md) and the follow-up
[storage comparison](../evidence/handoffs/2026-11-01-engine-decisions/STORAGE-REUSE.md). They contain source citations and
rejected alternatives. The later FOUNDATION-SPEC supersedes the first source report’s proposed transform constants with a derived finite
world/object/error envelope and an explicit residual test.

The reusable storage precedent is [hecs' library contract](https://docs.rs/hecs/0.11.2/hecs/); network authority and early
gameplay integration are informed by [Unreal's networking overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/networking-overview-for-unreal-engine).
These sources motivate choices, not performance or portability claims. No runtime code, dependency installation,
asset promotion, build or device experiment was performed for these decisions.
