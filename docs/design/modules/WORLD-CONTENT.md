# World, content and persistence

**Initial implementation specification:** [FOUNDATION-SPEC.md](../FOUNDATION-SPEC.md) now selects the concrete baseline mechanisms,
limits and failure rules. This family document retains the wider responsibility; specification does not imply runtime qualification.

1 November 2026. **Design status:** selected operating model for F3/F4/W1/W2/P7 under the [architecture decisions](../ARCHITECTURE-DECISIONS.md), subject to detailed schema/adapter design and open game-scale/device floors. **Implementation and qualification:** unbuilt. This design follows the [architecture brief](../ARCHITECTURE-BRIEF.md), [programme](../DESIGN-PROGRAMME.md), [shared contracts](../CONTRACTS.md) and [network session contract](NETWORKING.md). Its source inspection is recorded in [world/content sources](../research/WORLD-CONTENT-SOURCES.md). It does not promote current Broadsides code or its water experiments into the target engine.

## Commission and capability coverage

One custom-host 3D engine must support overhead, isometric, third-person and first-person games on native desktop, browser and iOS/Android. World identity and authoritative gameplay cannot change with camera or visual quality. Content must be editable by a human or an agent through the same typed operations, cooked for each platform, activated without missing physical ground, saved and restored after interruption, and observable with provenance. Pure 2D and MMO scale are outside scope. Initial extents, reference devices, networking and workload limits are selected in BASELINE-SPEC; qualification remains unrun.

| Requirement | Observable outcome and owner | Proof after design |
| --- | --- | --- |
| F3 identity/schemas | A saved object survives slot reuse, asset recook, and a move; schemas declare units/version/migrations | ABA, cross-thread stale result, rename/migration fixtures |
| W1 scene/world | One committed transform and lifecycle per object, with separate hot and specialised storage | hierarchy/physics/animation crossing fixture |
| W2 construction/streaming | Authored and seeded regions activate with collision/nav/gameplay consistency; edits survive unload | moving boundary, teleport, two-view route |
| F4 asset pipeline/residency | Rebuilds follow dependency hashes; products and leases obey peak memory bound | change one source; inspect DAG, overlap and retirement |
| P7 persistence/replay | Checkpoint at a world boundary, restore compatible content/state, diagnose replay envelope | save/edit/unload/reload and interruption fixture |

## Selected representations and authority

**Identity.** Every authored asset, prefab, region and persisted scene object receives a stable, opaque 128-bit ID from the authoring operation; path and display name are mutable metadata. A prefab node has a stable node ID within its prefab; an instance object is identified by `(instance ID, node ID)`, avoiding an arbitrary new ID for every copy. Dynamically spawned durable objects use `(world ID, monotonic spawn sequence)` allocated by the authoritative world and serialized with it. Ephemeral particles, query results and editor hover state have no durable ID. A deterministic procedural object's key is `(world seed, recipe ID, recipe revision, region ID, local candidate key)` with a stable hash and collision validation; changing a recipe revision requires an explicit migration or treats old and new outputs as distinct. Never derive a persistent key from a floating-point position or array order alone. Logical asset ID, source revision, cooked-product hash, runtime slot, and world-object ID are distinct types.

Runtime references are `(slot index, generation, world epoch)`; each free/reuse increments the generation before another occupant can be published. A generation wrap retires the slot instead of making an ancient handle valid. Job results also carry stable target ID, region activation epoch, product hash, and relevant host/device epoch. Publication checks *all* of them against the current manifest and live owner. A tombstone for a committed deletion prevents a late job or save overlay from resurrecting the object. Durable cross-object links use stable IDs with `required`, `soft`, or `external` resolution policy; only a live resolved link contains a runtime handle. This follows the generational safety mechanism in Bevy's entity documentation, without adopting Bevy as host.

**Schemas.** Registered component/resource schemas have stable field IDs, semantic units, validity/default rules and explicit integer schema versions. Editor/CLI/agent operations validate against the same registry before entering an ordered command log. Unknown optional fields may be retained as opaque data during a round trip; unknown *required* gameplay fields fail load. A migration is a pure, versioned function from an old serialized record to the current one, with source and target versions and recorded content dependencies. Migrations never silently reinterpret metres/centimetres, degrees/radians or changed enum meanings. Reflection is for inspection, serialization and tools; it does not require dynamic boxed storage in hot loops.

**World storage.** Select pinned `hecs` 0.11.2 as W1's **private** ordinary-component archetype store, behind a small W1-owned identity/access seam; it is not the F1 scheduler, F3 schema registry, save format or public plugin API. W1 maps durable IDs and public `(world epoch, W1 slot, W1 generation)` handles to current private `hecs` entities. A backend entity never escapes W1 into public, asynchronous, save or wire interfaces; its private identity map may retain the current backend handle, while query borrows are stage-bounded. Jobs, snapshots, saves, wire messages and enduring component links use owned IDs/data, never backend entities. Slot reuse increments W1 generation; generation wrap retires the slot. Exclude backend operations that can resurrect/reuse old handles across a live public mapping. Structural add/remove/despawn commands publish at declared barriers, and order-sensitive gameplay reductions use stable-ID order rather than backend query order. P3 bodies/broadphase, P4 poses, P5 navigation graph, V2 draw instances, A1 voices and F4 assets retain specialised stores keyed through checked W1 IDs. W1 owns the committed world transform/authority-mode tag; P3 owns solver-internal body mode/state and validates mode transitions requested by P4/P6. No generic sparse-set planner is built initially: if measured churn requires it, move that specific feature to an owner-specific side store while preserving W1 lifecycle. The selected dependency still needs pinned transitive/license and target-compile qualification; it has not been installed or measured.

**Hierarchy and transforms.** Runtime parent links form a forest (one parent, no cycles); authoring prefab references may form a DAG, but expansion is acyclic and checked with a recursion limit. Authored local transforms are finite translation, normalized **proper** quaternion and signed scale, composed `T * R * S`; world matrices are derived by parent multiplication. At a fixed tick, W1 evaluates dirty branches in stable topological order and commits current/previous world transforms. A reparent declares `preserveLocal` or `preserveWorld`. The former retains local TRS and knowingly changes world pose. For the latter, W1 computes `L* = inverse(new parent world) * old world` in f64, checks cycles/authority, decomposes to proper-rotation signed-scale TRS, recomposes it and rejects nonfinite, singular/ill-conditioned or sheared results if **either** local recomposition or resulting world residual exceeds the **declared finite transform/error envelope**. The positional bound is no looser than the game's spatial placement tolerance, including at large coordinates; it must not grow without bound with world magnitude. Operation diagnostics report the residual, limit and preserve-local alternative. The envelope's exact numeric precision, condition and scale floors are D1 prerequisites tied to qualified world extent/consumer error, not assumed here. Negative/reflected scale keeps determinant-aware render normals/winding; P3 collision and P4 skeletal consumers declare support or require explicit baked products. Zero-scale render-only nodes cannot serve an inverse, collider, dynamic body or normal transform. Import rejects unsupported affine/shear or performs an explicitly reviewed bake; it never silently projects it to TRS. Dynamic rigid bodies remain world-space P3 authority, with W1 publishing the solved pose; moving/nonuniform parent motion needs a declared joint or kinematic request. P4 root motion requests displacement before P3 and owns skeletal pose. This keeps C03's authority singular.

**Coordinates.** Authoritative world positions and region bounds use right-handed Y-up metres in f64 CPU coordinates. Region membership is an integer spatial index `cell = floor(position / L)` for a declared cell extent `L`; the index is an activation grouping, not a quantization of world position. Render and, where a selected solver requires it, physics operate in f32 local coordinates relative to a declared origin and origin epoch. Conversion is `p_local = f32(p_world - O)` at a barrier. Rebase changes local representation and invalidates stale spatial results, never world position/velocity/time or object identity. f32 roundoff is on the order of `2^-24 |p_world-O|` metres, so the chosen origin distance must keep this below each consumer's positional error budget; f64 world roundoff grows as `2^-53 |p_world|` and still needs an explicit maximum qualified extent. Different cameras may choose different view origins without changing world state. P3's local island origin, if used, must be converted by the same world-coordinate service and tagged with its own epoch; physics does not infer origin from the camera.

## Scene composition and procedural construction

An editable scene consists of region records, prefab instance records and explicit per-node/property overrides. Composition is deterministic: resolve a prefab dependency DAG; instantiate its stable nodes; apply ordered variants and then instance overrides; apply saved world deltas last under a declared precedence rule. A missing required prefab or cyclic reference fails the build/activation with a trace of the reference path. Depth and expanded-object counts are capped by the content profile. Do not import an entire USD composition engine into runtime. Godot PackedScene and USD demonstrate useful scene reuse/layering, but an explicit bounded override model is enough for this initial engine and makes AI edits inspectable. USD can remain an **import** format for complex offline authoring if a chosen toolchain needs it.

Authored placement and deterministic procedural placement produce the same validated instance records before cooking. A recipe receives immutable inputs: world/region seed, recipe revision, declared field assets, region bounds and neighbor-border context. Use counter/keyed RNG per candidate rather than an order-dependent shared stream, so parallel evaluation and neighboring-region load order do not change placement. Assign ownership of a border candidate by a canonical home cell; duplicate coverage in adjacent cells is an index entry, not a second object. Validate exclusion/clearance against the declared geometry and collision policy, with deterministic tie-breaking by stable candidate key. A recipe's *output* (IDs, transforms, dependency list and bounds) is hashable; gameplay-relevant output is cooked or reproducible on the authoritative CPU path, not generated only in a quality-cullable GPU pass. Environmental growth/damage is authoritative state layered over the recipe, not a new random roll on reload.

Terrain is a W2 geometry product family, not the world coordinate model. A landscape may cook heightfield and far meshes; caves/interiors use meshes or signed-distance-derived products as selected by their physics/render consumers. Cooked collision/nav/shadow/ray proxies may differ, but each product carries source ID, material/surface mapping, spatial bounds and declared positional/silhouette/coverage error. V2 selects LOD by projected error: perspective `e_px ≈ e_m f_px/z`, orthographic `e_px ≈ e_m s_px_per_m`. Orthographic LOD therefore cannot use distance alone. Hysteresis avoids oscillation; independently selected adjacent LODs require stitched borders, skirts or a shared boundary sample rule with an error bound. Collision and nav do not switch merely because a camera zooms.

## Import, cook and runtime residency

### First headless source-to-scene product slice

The first F4 implementation accepts an F3 `PinnedSource` only. It reads the verified
`project.json` and the single declared entry scene through the pin; no mutable checkout
path is accepted by the API. The fixed project parser names the entry ID, while the
authenticated manifest locates its stable `core.scene` record path. The fixed scene
parser retains its optional caption. A missing or incompatible record refuses the cook
without publishing a product. The source root remains F3 authority; F4 owns the derived
`SceneProduct` and its typed `SceneProductRevision`.

The v1 headless product key hashes a domain/version tag, target, the exact verified
project and scene input digests. The project bytes already contain the entry ID,
and the scene bytes already contain the caption. It does not hash the
whole `SourceRevision`: unrelated admitted files may advance the root without recooking
an unchanged scene. The product retains its source-root provenance separately. This
first profile has one entry scene and one CPU-only target. The product has a compact
bounded byte encoding and independent content hash. A load requires expected hash,
product revision and accepted root from a trusted future build manifest; it checks
byte integrity, format, product-key inputs and identity. Provenance stays in the
manifest, so identical scene products from different accepted roots have identical
encoded bytes. This is still not a `BuildRevision` or runnable game: code
binary/toolchain/schema closure, durable product publication/manifest and run/save
pins remain later F4/F1/P7 work. A complete build key must add those inputs before
W1 or N1 may activate a world.

The cook path allocates two bounded record buffers, parses only the selected project and
scene, and scans manifest record metadata once for the entry ID. Its cost is `O(project
bytes + scene bytes + manifest entries)` plus the native store's verified chunk reads;
it is offline and never runs in a world callback. At the admitted 100,000-entry maximum,
the one-time ID scan may need an index if it becomes a repeated cook bottleneck. Avoid
an always-live ID map until a workload justifies its memory. The product retains only
the caption and fixed identity/provenance/input digests; the input buffers are dropped
after cooking. Serialization allocates one bounded output buffer; load hashes the
bytes once before decoding fixed fields.
The [first executable proof](../../evidence/i2a/F4-HEADLESS-SCENE-COOK.md) checks edits,
unrelated files, historical pins and failure retention. End-to-end edit-to-ready,
product storage and runtime performance are still unqualified.

The [first-use review](../FIRST-USE-WALKTHROUGHS.md) makes reimport a concrete authoring obligation: F3/F4 preserve
source-to-canonical identity correspondence, while T1 records game-owned overrides and exposes mapping conflicts.
Imported node/material/skin/clip identities must be distinguished from mutable names and array positions. Where an
input format supplies no durable identity, FOUNDATION-SPEC selects conservative unique semantic-path/kind matching and explicit remapping when persistent IDs are absent; ambiguous or removed entries cannot
silently retarget references. Review added/removed mappings, conversions and affected overrides before accepting the
source change. Imported cameras/lights must be identified as accepted scene data or retained source data. This extends
the existing import/prefab boundary, not a separate authoring database or a claim that arbitrary reimports are lossless.

The source package owns editable masters, licenses, import settings and stable IDs; runtime packages own immutable cooked products and a manifest. glTF 2.0 is a supported interchange input for nodes, meshes, skins, morphs, materials and animation data, not a complete game scene, streaming format, collision contract or playback policy. Import validates glTF accessor ranges, coordinate/units and extension support, then converts once to engine Y-up metres and typed canonical assets. Material cook records colour space, alpha/coverage semantics, shading closure and texture encoding; mesh cook records current/previous deformation inputs, conservative bounds, LOD correspondence or explicit invalidity, and distinct shadow/ray/collision proxies. Animation cook preserves stable joint IDs/hierarchy, rest/bind and inverse-bind matrices, clip duration/wrap, event markers, morph target IDs and weight channels, retarget mapping, and a root-motion curve in metres/radians with per-clip extraction policy. P4 decides playback and sends the root request to P3; P3 returns solved displacement/residual to P4 while W1 stores only the final body transform. An incompatible skeleton/clip revision blocks activation unless the game explicitly permits a bind-pose fallback. Physics geometry must be validated by P3, navigation by P5, and visual appearance by V2/V3; no asset file implies those products automatically.

**Admitted source.** Ordinary project files and masters are the editable project. F3/F4 identify exact immutable
snapshots by path/byte hashes, stable IDs and declared dependency closure; a Git tree is provenance, not a substitute
for required LFS/master bytes. A tool/agent edit is a proposed patch, applied to files before admission. Direct file
edits enter the same admission path. Watchers report pending changes; only explicit admission selects build inputs.
F4 builds from that snapshot, never the changing checkout. A valid admitted draft need not be runnable. Source
admission, target readiness, running build and saved checkpoint remain distinct. FOUNDATION-SPEC owns coordinated
writer handoff, dirty capture, interrupted application, root/outcome durability and retention. There is no checkout
mirror or hidden accepted editor state. T1 exposes the same path to humans and agents.

Cook tasks form a directed acyclic graph. The key of a product is `H(canonical input bytes, dependency product hashes, importer/cooker version, schema version, target platform/API/quality settings, declared toolchain and compression settings)`. Canonical serialization sorts maps and normalizes numeric encodings; undeclared environment inputs or nondeterministic tools disable shared caching until fixed. A source change invalidates only reachable dependent products. Product bytes are verified against the manifest hash on load. A manifest revision atomically maps logical asset IDs to a compatible product set and dependency closure; an old manifest stays usable until the replacement is complete. A substantive source edit builds a new pinned run; failed cooking leaves the old ready build identifiable. Qualified visual preview stages its affected products before extraction and cannot change gameplay semantics. Any later live gameplay/region recipe must stage the complete affected closure, recheck active-set generations and publish required representations together; a new dependent forces inclusion or reprepare. Inactive saved regions resolve explicit base-revision/overlay ancestry before activation. This follows the dependency/artifact model described by Unity and Bazel, adapted to game-specific variants. Product hashes identify *bytes/revisions*, not objects in a save.

F4 owns the state machine `unrequested → fetching → compressed → decoding → CPU-ready → GPU/consumer-ready → retired`, with cancellable work and generation-checked publication. It supplies reference-counted leases for decoded CPU products and consumer-specific derived resources. A GPU lease includes device generation and retires only after its last submission fence. F4 does not own V2 draw tables, P3 colliders or P5 tiles; those consumers own their representations and release leases on detach. Browser cache, native files and mobile asset packs supply bytes through an async source interface; they are replaceable caches/delivery channels, never save authority. A lost GPU device invalidates GPU products and reuploads from retained/cooked source after a new device epoch; saved world state is untouched.

For admission, estimate `M_peak = M_live + M_pending_decode + M_upload_staging + M_replacement_overlap + M_transient + M_reserve`, counting shared allocations once in a physical ledger and each logical claim separately. Before starting a decode, region activation or hot replacement, reserve its worst-case staged and overlap bytes; if it fails, evict unleased products, shrink lookahead, select a qualified lower-cost variant, or delay activation visibly. F4 queues have byte and count caps, not just priority. If `B_c` compressed bytes expand by factor `r` and upload needs `B_u`, a simple two-copy path can require `B_c + r B_c + B_u` concurrently until decode and upload fences retire; chunking lowers the peak only when the decoder/format actually supports independent chunks. Peak is measured separately for OS physical memory, allocator commitments and logical product accounting, especially on unified-memory Macs/iOS. F1 owns total budget and jobs; F4 supplies reservations/lease telemetry, V1 owns GPU execution.

### Capability survey obligations

[H02/H06/H07/H10](../CAPABILITY-SURVEY.md) clarify this existing path. A creator can import geometry, including external
blockout; an outdoor profile additionally needs an imported terrain/recipe, surface and vegetation route,
with render/collision/nav products inspected together. In-engine blockout modeling, sculpting and spline-road tools remain optional.
Publish supported interchange/material extensions per profile as preserved, converted, explicit fallback or rejected;
do not imply arbitrary glTF fidelity. The material/path requirements feed A03's shader demand/readiness design.

Dynamic game-selected assets need explicit roots: stable IDs, declared asset families, or a selected external-content
admission policy. F4 computes their cooked dependency closure and T2 explains inclusion and byte cost. Export validates
known declarations/references; the runtime rejects undeclared required requests with a source/content cause where
available. Static scans cannot establish every future lookup in arbitrary game code. External content is a separately
validated path, not a raw-file bypass around version, integrity, schema and target compatibility.

Startup closure, on-demand delivery, installed/offline readiness and memory residency are already distinct in the
design; expose them separately for each named build and its first playable scene. A residency cache alone does not
promise offline availability. Detailed formats implement the selected source-admission boundary in D1 rather than a new registry.

## Streaming transaction and consistency

**Initial scope:** bounded preloaded gameplay worlds and explicit stop/load transitions, as specified in
FOUNDATION-SPEC. Readiness/identity/consumer invariants below also govern the wider W2 streaming capability; they
are not a selected seamless-streaming algorithm. Live scoped activation requires its own consumer commit/rollback
and peak-memory design before implementation. Neither an ordinary source edit nor first portable game requires
whole-world/provider live resume serialization. Visual residency may still stream independently of gameplay closure.

**Reader lifetime (C02/C05 reconciliation):** W1 publishes copied/COW immutable read-model pages in F1's bounded
snapshot pool. Render/audio/query consumers lease the committed epoch and referenced provider/CPU products; a region
cannot reclaim them while extraction still reads them. V2 then owns a RenderPacket and its GPU/staging leases, so
GPU completion does not pin the whole world. Optional stale views reset/cancel on the count/byte limit; required
readers cause backpressure/explicit overload until actual lease release. P3 baseline queries run within the pre/post
step and return owned results; async nav leases immutable tile/obstacle generations. P7 captures all durable fields
separately at a committed barrier, with bounded copy/serialization reservations. No worker reads a live mutable solver.

W2 derives desired regions from unioned gameplay relevance, physics bodies, nav routes, audio range and all active cameras; frustum alone is insufficient. Each region has a stable ID and a monotonically increasing activation epoch. States are `inactive`, `preparing`, `ready`, `active`, `retiring`; only W2 changes them. Unreal World Partition demonstrates stream sources and loaded-versus-activated states, but a universal grid and its exact cell size are not presumed. Region records may be a grid, authored portals/interior rooms or both. Large objects have one home region and coverage indexes in others; dynamic objects can transfer home region without despawn. Hard references, active contacts, joints, quests or current nav traversal pin needed regions or resolve through an explicitly durable external reference.

Preparation resolves manifest closure, P7 saved overlay and required P3 collision, P5 nav tile plus boundary/off-mesh adjacency when navigation is enabled, and gameplay state. P5 border portals are verified against both tiles' agent radius, maximum slope and step-height policy; off-mesh links need the same validity check. Optional V2/V3/A1 representations may use authored safe placeholders or arrive later. At the **pre-tick admission barrier**, W2 checks activation epoch/product versions and memory reservation, installs W1 objects and required consumer representations in one ordered commit, and exposes the region to physics/nav queries and gameplay together; only then do portals open. The world step reads this admitted `S_n` and later publishes `S_(n+1)`. Pending workers cannot mutate live world stores. A public query declares pre-step `S_n` or post-step `S_(n+1)` and includes region/content epoch; async path results additionally carry both nav-tile and obstacle generations, and invalidate on either change. A visual proxy never makes a blocked route traversable. Presentation-only product publication may occur before view extraction, but is tagged to the committed world/content revision and cannot change simulation state.

For a moving body, lookahead must cover `d_prepare ≥ v_max τ_ready + d_stop + r_body + margin`, where `τ_ready` is a *qualified upper-tail* fetch/decode/attach latency, not an average, and `d_stop` is the policy's stopping distance. Dynamic bodies pin collision cells intersecting swept bounds over the admitted motion horizon. If the bound cannot be met (teleport, network stall, cold browser cache) or pins exceed the reserved memory budget, delay the transition or block traversal with a visible loading rule; do not let a body move into absent collision or remove support geometry beneath it. On retirement, first close nav portals/new entrants at a pre-tick barrier, then let existing contacts/links resolve or pin the region; affected agents repath or stop under game policy. Checkpoint dirty durable state; remove W1/P3/P5 authority at a later pre-tick barrier only when no admitted query/body can still depend on it; detach render/audio; release F4 leases and wait for GPU fences. Retiring a region does not erase globally persistent identities. P6 cross-region references resolve durable IDs through generation-checked lookup to live, pending or absent, never raw slots. Source edits to an inactive or active region change ordinary authored files, invalidate the relevant cook closure,
and require a new admitted build/run by default. They do not mutate a player's saved overlay. A stop/load transition
restores a compatible P7 checkpoint or starts the declared new level; no old overlay is applied to unrelated new source.
If a later live-edit/streaming recipe is admitted, its consumer closure, old/new lifetime, failure recovery and peak
reservation must be specified together before it can publish at the barrier.

Multiple cameras read one committed world; each V2 view has its own origin and projected LOD. A region/content/topology/LOD change carries stable IDs and versioned affected bounds to V2/V3/V4/E1 so shadow, reflection, GI, effect and image histories can invalidate locally where safe. Previous geometry samples are evaluated at each history's **actual retained sample time** (previous accepted image for that view/effect), not assumed to be `n−1` or OS scanout time; if an asset/LOD/topology change cannot map points across those times, motion is marked invalid/reactive. A skipped view can retain an older sample than another view. Offscreen shadows and audio remain relevant even if no main-camera pixels show the region.

## Save, migration and replay

**Recovery ownership:** C10 and FOUNDATION-SPEC own the precise root/admission protocol. F4 retains admitted
source/product closure; P7 owns each private or released save's tick/base/build/overlay/provider tuple. Authoring
admission and player saves never share a global mutable root. Replica/predicted clients cannot save as the server.
A newer selected source snapshot does not reinterpret old checkpoints; restore either uses the matching closure or
an explicit migration. Failed publication preserves the last root, and cleanup respects roots plus active leases.

P7 owns atomic durable checkpoints at a committed tick, plus a bounded journal of accepted typed operations since the checkpoint. A save header records format/schema versions, persistence authority/world ID, tick, **base admitted source revision and build/rule compatibility**, game/profile compatibility policy, content manifest revision, recipe versions, required asset IDs and provenance. The body stores durable object IDs, authoritative component state, region deltas, tombstones, spawn allocator state, and provider-owned states such as physics motion, AI goals and persistent wetness/snow/plant damage/water-region variables. It excludes runtime slots, private `hecs` entities, network wire IDs, handles, GPU resources, decoded assets, culling indexes and audio voices. Each provider declares its save codec, migration and offscreen policy; W2 coordinates region checkpoint/unload, but does not fabricate another wetness or physics accumulator. If an unloaded field would evolve under weather, either save an analytically sufficient aggregate and integrate known inputs on reload, run a cheap offscreen simulation, or explicitly freeze it under game policy. The choice and error bound belong to that provider/game profile.

Use FOUNDATION-SPEC's declared per-platform atomic checkpoint publication, with verified bytes and dependency closure; do not invent a separate pointer/rename protocol here. Browser player-save transactions remain required even though browser authoring is deferred. Mobile suspend may terminate early, so periodic checkpoints precede suspend. Restore validates the manifest or an approved migration mapping, migrates records in version order, prepares required content/regions, reconstructs specialised stores, and only then publishes the first world snapshot. Missing required content or migration fails with a reason and preserves the old save; optional visual content may fall back under declared policy. A content recook alone does not invalidate a save because logical IDs remain stable, but a semantic prefab/recipe change can require migration of overrides and deltas.

Replay records initial checkpoint/content hashes, fixed-step commands in canonical order, seeds and all external/nondeterministic results needed for the declared envelope. Local same-build state-hash reproducibility is the first target; cross-platform bit-identical physics is **not** promised without chosen solver/float constraints and proof. Asynchronous asset arrival is logged as a tick-stamped activation decision when it affects gameplay; visual-only arrival need not change authoritative replay state. A diagnostic playback can use periodic state hashes/snapshots to find the first divergence. Save compatibility, user-authored scene compatibility and replay compatibility are separate versioned policies.

**Play and code iteration.** A developer play run constructs a separately scoped W1 world from a target-ready pinned source/build/product revision, with fresh session/world epochs, clocks, events, provider instances and private test-save routing. Two views read one committed session world; neither ticks it independently. Native development uses a child play process by default; browser/headless may use a separate worker or in-process instance with the same logical state/storage separation. Stop discards unsaved gameplay deltas; an explicitly retained private test save may be restored only against its exact compatible closure or an approved migration. Source edits during play do not change its pin. Code/schema/gameplay-affecting data edits rebuild and restart a new play session; failed compilation or migration leaves the old build and save closure identifiable. A reviewed apply-back of selected schema-marked authored properties is a T1 operation against pinned base and current source, not P7 copying arbitrary simulation state into scene files. The same local run identity/authority boundary supports N1's separate server, replica and predicted worlds: a network match ID is not the local world epoch, replicas use F3's purpose-specific projection and cannot save as the server. An authoritative server pins its rule/gameplay-content revision until an explicit restart or separately designed versioned migration/peer renegotiation.

## Whole-engine handoffs and failure paths

| Path | Ordered owners and invariant |
| --- | --- |
| Normal tick and two views | F1 admits ordered commands and W2's prepared simulation-affecting region/content transaction at the pre-tick barrier; P3/P5 query admitted `S_n`, P3 solves body pose, P4 builds pose, W1 commits `S_(n+1)` once; presentation-only products publish with that revision before V2 extracts two views with distinct origins/LODs/histories. A1 reads source events and time. Neither view advances W1. |
| Animated character crosses a border | P4 root-motion metadata yields request, P3 resolves against active colliders, W1 commits solved pose; W2 already opened nav adjacency only after neighbor collision/nav ready. If preparation is late, movement is gated and P5 repaths/stops under policy. |
| Region unload/edit/reload | T1 typed edit with expected revision reaches P7 delta, W2 closes entrances and commits retirement, consumers detach and release leases; F4 recooks affected DAG, W2 prepares new region plus delta, commits one generation; stale callbacks fail epoch checks and temporal caches invalidate. |
| Mobile suspend during upload | F1 closes submission; W2/F4 cancel publication or retain safe CPU inputs; V1 GPU lease/fence policy drains old generation; P7 relies on prior checkpoint and best-effort new one; resume revalidates manifest/device, reuploads, publishes new view histories. No hidden wall-time simulation. |

## Alternatives, cost, proof order and limits

| Alternative | Why it is not the baseline; reconsider when |
| --- | --- |
| One node tree for all world systems | Simple scene editing, but hot bulk state and physics/nav/render resources have incompatible iteration and lifetime needs. Keep the tree as authoring hierarchy and W1 transform relation; specialised stores stay separate. |
| One archetype ECS for every subsystem | Dense iteration is useful, but spatial indexes, solver constraints and GPU resource lifetime need their owners. Keep private `hecs` for ordinary W1 components and specialised stores elsewhere; no generic sparse planner initially. |
| Runtime USD or unrestricted prefab inheritance | Powerful composition adds complex ordering, dependency and invalidation semantics to every load. Support offline import or a later profile if complex collaborative authoring demonstrates the need. |
| Preloaded bounded gameplay world | Selected for R0/R2/N1; boot/residency must fit the named recipe. Larger scenes require explicit stop/load or a separately designed live streaming recipe; preloading is not a universal scale promise. |
| GPU-only procedural world truth | Fast visual population, but quality/device-dependent order and unreadable collision/save results violate shared gameplay identity. GPU generation remains an optional visual derived product. |

Symbolically, hierarchy update is `O(number of dirty descendants)` with a worst case of `O(N)`; a streamed activation is `O(objects + mandatory attachments + dependency edges)` plus I/O/decode, not a constant-time flag. A naive region relevance pass over `R` regions costs `O(R)`; an indexed source/region intersection should approach touched cells plus query overhead. Product DAG recook is `O(changed reachable nodes + edges)` plus cooker work. Save cost is proportional to dirty authoritative records/journal bytes, with full checkpoint cost proportional to durable world size. These are algorithmic bounds, not measured frame times. W2 must cap objects/attachments per barrier or present loading, because one huge atomic activation can violate F1's tick budget.

**Ordered build/proof:** (1) freeze ID/schema/coordinate and operation contracts with F1/P3/P4/P5/V2/P7; analytically specify a small scene's IDs, transforms, cell bounds, asset closure, staged bytes and save fields. (2) Implement headless scene composition, deterministic procedural keys, cook DAG and migration fixtures. (3) Prove one region border with a moving animated body, physics collider, nav adjacency, persistent field, two cameras and audio; predict activation time/memory from the declared products first. Reject if a query sees a half-active region, an old generation revives, or a missing collider is traversable. (4) Add hot edit, unload/reload, device loss and suspend paths, checking exact epochs/leases and state hashes. (5) Qualify realistic content on named Mac/Windows/browser/iOS/Android builds with p95/p99 activation latency, startup/download, peak physical/logical memory, shader/upload hitches and sustained mobile temperature. If measured latency defeats lookahead or peak exceeds profile, revise cell size, product variants or staging strategy under the same correctness contract; do not tune by accident into a different world model.

**Open qualification:** selected content density and finite transform limits have not been measured on reference devices; browser persistence quota, store acceptance and qualified live-streaming `τ_ready` remain unproven. Live streaming outside the bounded initial profile needs a separate detailed recipe. Platform budgets in [PLATFORM-RUNTIME](PLATFORM-RUNTIME.md) are proposals, not certified limits. N1 selects server authority and optional replication/prediction, while NETWORK-SPEC fixes the initial transport and static-controller prediction profile; additional services/protocols remain extensions. Save/cloud synchronization remains separate. A new prefab version's semantic mapping cannot be inferred generically; content authors must provide migrations for affected durable overrides. High-level integration review is recorded in [WHOLE-ENGINE-REVIEW](../WHOLE-ENGINE-REVIEW.md); runtime evidence remains unrun. Source code reuse requires exact revision, license/notice, modifications and update owner; Bevy/Unreal/Godot/USD are design references, not bundled runtime dependencies. Commodity glTF parsing, image compression, physics and storage libraries may be selected by narrow platform/license audits; the chosen private `hecs` version still requires license/target checks. Authoring and cook tools expose schema, dependency, source hash, error and provenance queries to agents and humans through the same T1/T2 interface.

## Source recovery and build-storage lifecycle

The [storage lifecycle](../STORAGE-LIFECYCLE.md) assigns F4 the canonical source/dependency graph, complete cook keys,
immutable product identity and protected release/rollback closures. Source masters and unique generated inputs are
not evictable caches. Exact dynamic-content/locale and supported save dependencies participate in retention roots.
T1 exposes working-file proposals, admitted snapshots and running builds; T2 owns disk reservations, worker storage and evidence
retention. Disk readers/writers require leases distinct from runtime resource/fence lifetimes. D1 must specify
publication/root-registration/collection races and recovery before automatic deletion; I2 proves cold restoration
and failed publication with one package. Existing asset records remain canonical, without a second asset database.

The [lifecycle walkthroughs](../LIFECYCLE-WALKTHROUGHS.md) add expected selected-source/current-delivery preconditions
before publication, independent game revision pins and rollback closures outside new build inputs. Local adoption
can be useful while remote recovery is unverified; no remotely recoverable claim follows. D1 must implement the
file-first patch/application/admission protocol; I2 must reject stale exporters without disturbing an existing release.
Writer coordination protects only cooperating participants; uncertain external bytes remain untouched for review.
A failed or partial file application is not admitted and cannot alter a pinned run. See FOUNDATION-SPEC rather than
implementing a competing source/mirror/recovery mechanism.

Preserve-world authoring evaluates old and candidate-parent world matrices from the operation’s expected admitted
SourceRevision, never a runtime/play pose or unadmitted working-file edit. Cycle, conditioning, TRS representability
and affected-consumer checks validate the candidate before file application and admission. Working-file divergence
requires explicit sync/rebase; it is not an implicit transform input. Rejection leaves the admitted source and running
world unchanged. FOUNDATION-SPEC owns the proposal/application/admission outcome and recovery protocol.
