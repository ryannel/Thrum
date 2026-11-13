# Content loop: edited meshes, textures and volumes appear in the running game without a stall

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C05, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[the G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md),
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) and
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) govern.

## The request

The owner's words weren't passed on; this design answers the lead's planning task of 13 November 2026, word for
word:

> Plan the CONTENT-LOOP module (row 12 in docs/plans/ARCHITECTURE.md: edited meshes, textures and volumes appear
> in the running game without a stall; needs APPLE-PRESENTATION M2). Write its design, milestones and done-when
> file so M1 is buildable as soon as APPLE-PRESENTATION M2 passes review. SEA, CLOUDS and RENDER-2D depend on it;
> account for what they will need from it.

What the module is for comes from [the brief](../BRIEF.md):

- Decision 1, the V1 exit, asks for "cooked meshes/textures" and "hot reload".
- Gate G5 reads: "Stat cache, BLAKE3 CAS, aligned mmap products, glTF meshlets, KTX2 ASTC and BC7, atomic
  manifest, watcher and dev server. Keep only the runtime validation the game needs; shipping never links
  authoring. Prove visible asset edits without GPU stalls."
- Decision 3: numeric performance figures are baselines, not gates.
- Decision 6: sea and clouds are finished as Thrum modules and are not part of the exit.

[The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) adds the track 2 condition: G5 cooks baked 3D
volumes as content-addressed products, and "placement, product identity and history invalidation are explicit
inputs". The module list's interface rules add two more: cooked meshes reach entities as mesh rows, and `SEA`,
`CLOUDS` and later `RENDER-2D` take cooked products from this module.

This design reconciles the code on main; it is not a blind study. Read for it: `thrum-asset`, `thrum-cook`,
`thrum-asset-gpu` (`lib.rs`, `load.rs`, `staging.rs`), `thrum-render`'s `contribution.rs`, `synthetic.rs` and
`synthetic/{mesh_arena,mesh_route,camera_culling}.rs`, `runtime.rs`, `examples/v1-game/src/cooked.rs`, the G5
decision and [its evidence](../../evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md), the designs of
`CULLING`, `EFFECT-GRAPH` and `APPLE-PRESENTATION`, the `SEA` and `CLOUDS` briefs, and the Broadsides cloud
asset contract (`notes/clouds/ASSET-CONTRACT.md` at `b4d8250e6`, read only).

## Purpose and scope

A creator saves a source file and the running game shows the new product, and no frame is late because of it.
Shipping builds map the same products and never link the cooker.

**What exists.** The loop is built and runs in the V1 game on the Mac and in Chrome. The
[G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md) stays the reference for the mechanisms it
describes; where it and this design disagree, this design wins, and M3 brings its status lines up to date.

- `thrum-asset` (shipping): the store layout, `HEAD`, the manifest, mapped mesh and KTX2 views, validation, and
  the dev-server client for targets that cannot map a store.
- `thrum-cook` (authoring only): stat cache, action cache, mesh, texture and volume cookers, atomic publication,
  watcher, dev server.
- `thrum-asset-gpu` (shipping): **residency**. One loader follows `HEAD`, plans a publication as bounded copies,
  fills a ring of at most four staging chunks off the render thread, and the render thread records at most one
  budget of copies per frame inside the scene's own graph. A set becomes drawable in the submitted frame that
  records its last copy. It draws **placements**: cooked meshes culled per meshlet (or one full-range draw where
  the device lacks indirect execution) with the scene's own lighting, and one diagnostic volume.

**Commissioned.**

1. **Leases** (M1): another engine module is lent resident cooked textures and volumes to sample in its own
   passes, with identity and a change signal. The staging failure path gets its missing test.
2. **Cooked mesh rows** (M2): the instance field's entities draw meshes read from cooked products, and an edit
   replaces a row's geometry while the game runs.
3. **The close of G5** (M3): the edits proved in the running valley on the Mac and in Chrome under
   reconstruction, the gate's clauses checked one at a time, current maturity rows, and the closing review.

**Not built.** CAS garbage collection and reader leases; layered EAC volumes; any iPhone volume encoding (owed,
see [open questions](#open-questions)); parameter files as a product kind; sprite-sheet and font products (track
3 adds them when it starts); cooked textures on entity materials (entities keep the scene's material table); a
mesh-shader meshlet path; source admission, and with it a boundary for an external save and build provenance in
the manifest (both wait for T1; see [what a publication promises](#what-a-publication-promises-m3)); a browser
client that verifies and stages off the page's thread. None of these is needed by the exit or by the first sea or cloud
frame.

**Consumers.** The V1 game and `SHOWCASE` (placements and mesh rows), `CREATOR`'s sky kit (placements), `SEA` (a
foam texture), `CLOUDS` (density volumes), `RENDER-2D` and `GAME-UI` after V1 (sheet pages and fonts).

## What a creator and a consumer must get

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **C1 An edit appears.** Saving a source changes the running game | glTF meshes with separate buffers, PNG and JPEG textures, `TVOL` volumes; drawn as a placement, as an entity's mesh row, or through a lease; a mapped store on the Mac and a fetched one in Chrome | A reload that restarts the scene or re-uploads everything. Rejected by uploaded bytes that equal the changed products, unchanged textures kept by identity, and no pipeline or graph creation |
| **C2 Nothing stalls.** No frame is late because of a first load or an edit | On the Mac, any product the device admits (a 149 MiB 512³ volume is on record), several consumers loading at once, frames in flight. In Chrome, products of the size the exit scene and the cloud library use | An offscreen proof that hides a stall in the game. Rejected by per-frame copy bytes asserted in the game, staging held to its bound, and the game's own frame records across edit windows |
| **C3 It appears whole.** One frame shows the old content and the next the new, through every reader at once | Every product kind; a product read through one path or through several (a mesh that is placed and is an entity's row, a texture on a placement and in a lease); a discarded frame at the switch; reconstruction on | A half-uploaded texture, a mesh drawn with new indices over old vertices, a mesh new where it is placed and old on the entities, or a ghost of the old content under TAAU. Rejected by frames compared with all-old and all-new references, and by an inspected sequence |
| **C4 A bad save is harmless.** The last good content stays on screen and the reason is named | Every save the cooker or the runtime can tell is bad: a source that does not parse, a truncated file, a glTF whose buffer is missing or too short, a file changed while it cooks, a product the device cannot hold, a failed upload, a name the manifest lacks. An incomplete save that still validates is outside this range and is shown | An error that blanks the scene or repeats forever. Rejected by tests that inject each failure |
| **C5 It survives the lifecycle.** Resize, capture frames and a replaced device | With an upload in flight | A view from the old device. Rejected by generation checks and equal output after restaging |
| **C6 Other modules use products.** A sea or cloud pass samples a cooked texture or volume | Twelve 128³ volumes and a texture at once; an edit to one | A consumer with its own loader, or one that sees a product before its upload is committed. Rejected by the commit rule's test and a question on the code |
| **C7 Shipping excludes authoring.** | Every shipping package | A clean dependency graph over a dirty binary. Rejected by the symbol scan with its positive control |

The bar for C1 to C3 is live reimport in Unreal Engine or Unity, without their hitch, at the smoothness of
streaming in a shipped open-world game such as Marvel's Spider-Man 2. It is judged in words, in the running game.

## Research and selection

- **Amortised uploads are current practice, counted in items or bytes.** Unreal caps texture streaming per frame
  (`r.Streaming.MaxNumTexturesToStreamPerFrame`, `r.Streaming.AmortizeCPUToGPUCopy`) and virtual-texture page
  uploads per frame (`r.VT.MaxUploadsPerFrame`); reports of hitches there are about uploads that escape those
  caps ([Epic forum, 2025](https://forums.unrealengine.com/t/how-to-setup-asynchronous-streaming-and-buffer-pool-to-avoid-frame-drop/2724861)).
  Thrum's belt is the same idea with a byte budget and a bounded staging ring, so it stays.
- **Persistent resources are owned outside a pass and lent to it.** Unreal's
  [RDG](https://dev.epicgames.com/documentation/en-us/unreal-engine/render-dependency-graph-in-unreal-engine)
  registers external textures for exactly this. A lease is that shape: residency owns the texture, the consumer
  binds a view.
- **Meshes packed into shared buffers are replaced in place, and only what changed is rebuilt.** Bevy's mesh
  allocator packs meshes into slabs, defers a new mesh's copy until its final place is known, and tells
  dependants which meshes moved when a slab grows
  ([`allocator.rs`](https://docs.rs/crate/bevy_render/0.17.3/source/src/mesh/allocator.rs)). Thrum's arena is one
  such slab, so row replacement follows it and adds the tentative-frame rule Bevy does not need.
- **A changed asset is swapped for all its users at one point, after it is prepared.** Unreal's reimport tears
  down and recreates the render state of every component that uses a mesh under one scope
  ([`FStaticMeshComponentRecreateRenderStateContext`](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/FStaticMeshComponentRecreateRend-)),
  and pays a hitch for it. Bevy moves changed assets to the renderer at one extraction point per frame and holds
  back what exceeds a byte budget until a later frame
  ([`render_asset`](https://docs.rs/bevy_render/0.18.1/bevy_render/render_asset/index.html)). Thrum prepares
  under its budgets first and then switches every reader in one frame, which is C05's rule for a preview.
- **Compressed 3D textures are available in Chrome.** BC and ASTC sliced-3D features shipped in Chrome 139
  ([release notes](https://developer.chrome.com/blog/new-in-webgpu-139)), with the fallback order ASTC, BC,
  uncompressed. Residency already selects by the capability record, so Chrome draws BC4 volumes where the record
  allows and R8 otherwise.
- **What would change the selection.** If a consumer's cost account needs many volumes in one pass and the
  portable limit of 16 sampled textures per stage binds, the answer is a cook-time atlas (one volume product), not
  a different lease. If Chrome loses frames on products of showcase size, the dev-server client moves its
  verification and staging off the page's thread, designed here first.

## Mechanisms

### Leases (M1)

A **lease** is a named request, by an engine module, for a set of cooked 2D textures and volumes. The scene's
residency makes them resident and lends them as read-only texture views. Its names are fixed when residency is
configured, as placements are.

1. **One residency uploads.** Leases and placements share the loader, the staging ring and the per-frame byte
   budget. A consumer never opens the store, starts a loader or records an upload.
2. **Commit before lend.** A lease's set is offered only after the frame that recorded its last copy was
   submitted. Consumers bind it from the next frame. So no order is needed between contributions inside a frame,
   and a discarded frame cannot expose a half-uploaded product. The cost is one frame of latency, which
   placements share when a scene has a lease.
3. **Whole sets, with every other reader.** A lease's products switch together, in the frame the scene's
   placements and rows switch, under [one publication, one switch](#one-publication-one-switch-m1-m2). That
   section also says what supersedes a publication and what refuses one.
4. **Identity.** Each lent product carries its manifest name, content hash, profile, format, dimension, extent
   and mip count. A product whose hash did not change keeps its texture and view across generations and is not
   uploaded again.
5. **The change signal is a comparison, not an event.** Each lent product carries a serial that moves when its
   view does. The consumer records the serials a frame used when that frame is submitted and advances history
   (`submitted(true)`); a product whose offered serial differs from the recorded one has changed. So a change
   cannot be lost: when the first frame that used the new view is discarded, or is a capture that does not advance
   history, the next frame still sees the difference. The consumer treats the change as pending in every frame it
   records until one that used the new view is submitted and advances history, and not after. This is the
   contribution seam's commit rule applied to the consumer, and residency keeps no state per consumer. What the
   change means is the consumer's: it applies the history invalidation it declared (none, its own history, or
   the view's reconstruction history) and writes reactive coverage under
   [the temporal contribution](../APPLE-PRESENTATION/DESIGN.md#seams-for-thrum-core-proposals).
6. **A stable binding shape.** For one kind, every profile samples as filterable float with the same view
   dimension (R8 and BC4 volumes; RGBA8, BC7 and ASTC textures). A swap or a different profile needs a new bind
   group and never a new pipeline or layout.
7. **Lifetime.** Views belong to a device generation. On a new generation the lease is empty until the same
   generation of content is staged again; the consumer drops its old views in `rebuild`. An old texture lives
   until its last holder lets go, so the overlap is old plus new for the changed products only.
8. **Absent is a state.** Before the first set is resident and after device loss the lease says so, and the
   consumer draws without it.
9. **Placement is not in the product.** A product is texels or voxels. Transform, physical extent, extinction
   scale and every other meaning are the consumer's inputs, from the game.
10. **CPU readers** read the portable product through `thrum-asset`. It is always cooked, and its level bytes
    are the decoded data. A CPU reader takes the product the lease names, by its content hash, and never
    whatever `HEAD` names, so it changes in the frame the lease does.

### Cooked mesh rows (M2)

[The entity-visibility interface](../ARCHITECTURE.md#interfaces) says an entity draws a whole mesh row from the
scene's mesh selection. Today those rows are authored in code and "changing a slot's mesh means preparing again".

- **Rows from products.** Residency converts a cooked mesh product to the route's vertex format off the render
  thread and gives the game the geometry for the scene's selection. Positions, UVs and indices are copied
  exactly; normals are the product's decoded normals. The conversion applies the arena's own admission (finite
  data, indices in range, the coordinate limit, capacity), so a product the arena would refuse never reaches it.
- **The same rows either way.** Preparation takes rows from the store's current generation; later publications
  replace them live. A live-edited scene draws exactly what a scene prepared fresh from the edited store draws.
  That equality, byte for byte in Direct, Frustum and Occlusion, is the oracle.
- **A row keeps its identity.** The row's number, each slot's choice and bucket membership do not change. Only
  the row's geometry, index range, base vertex and bounds do.
- **Write, then switch.** The new geometry is written into arena space no drawn row uses, at most a fixed number
  of bytes per frame. The write is the row's part of preparing a publication, and the route reports it done once
  the frame carrying its last bytes is submitted. The row does not switch then. It switches in the publication's
  switch frame, with every other row, the placements and the leases
  ([one publication, one switch](#one-publication-one-switch-m1-m2)). That frame records the switch of every
  reader of the row: the forward draw ranges, the compaction's mesh table (range and bounds), the depth prepass,
  and the Sun casters' draws and fits. No frame mixes old and new.
- **The switch is tentative, like every recording.** It commits when its frame is submitted. A discarded switch
  frame commits nothing, and the next frame records the switch again. A buffer write queued by a discarded frame
  still reaches the device with the next submission, so a table written for a switch that did not commit must
  never be drawn with the old ranges; the frame after a discarded switch frame is the test of that.
- **No switch overtakes a recorded frame.** The arena write, the switch and the retirement all go through the
  frame host, and the host's encoded-frame token borrows it exclusively until the frame is submitted or dropped
  ([`runtime/tokens.rs`](../../../crates/thrum-render/src/runtime/tokens.rs),
  [EFFECT-GRAPH](../EFFECT-GRAPH/DESIGN.md)'s rule that recording is tentative). When a switch is recorded, every
  earlier frame has been submitted or dropped. A frame encoded before a switch and submitted after it cannot
  occur, so nothing fences it and no test builds it. Allowing it would be a change to the host contract, made in
  `EFFECT-GRAPH`'s design first.
- **Retired space.** The old geometry's arena space is retired when the switch frame is submitted and reused only
  after that. Frames submitted earlier are safe by queue order. A replacement whose publication is superseded or
  refused gives back its own space, never the space the drawn row still reads.
- **History.** The first frame that advances history, at the switch or after it, carries the frame's
  history-reset flag; a capture frame that commits the switch does not use it up. The instance path keys history
  by slot, with no per-mesh marker, and an edit is an authoring event, so one reset of reconstruction is accepted.
- **Refusal.** A replacement the arena refuses, or a broken source, leaves the old row drawing with the reason
  recorded. An arena refusal refuses its whole publication, for every reader.
- **The retained scene follows.** Resize and device replacement prepare from the newest committed rows.
- **Nothing is compiled.** An edit may create buffers and bind groups, as placements do today. It never creates a
  pipeline or compiles a graph.

The arena, its rows and the route belong to `CULLING`. By
[the seam ruling](../ARCHITECTURE.md#interfaces), this module's builder adds row replacement to them after
`CULLING:M3` is Done, and the route's bounded write is its own, beside residency's belt. The route allows 64
buckets, three per selected mesh, so a scene selects at most 21 distinct meshes; `SHOWCASE` sizes its entity
meshes inside that.

### One publication, one switch (M1, M2)

A **publication** is one generation of the store as residency resolves it for the scene: every product that
differs from what a reader shows, for every reader the scene has. There are three kinds of reader: the placement
set (the cooked meshes and the diagnostic volume residency draws, with their textures), each lease, and each
cooked mesh row. This is the "entire affected presentation closure" of C05's qualified visual preview. The rules
below are that clause as this module builds it: prepare the replacements, publish at extraction, change no
gameplay. M1 builds them for placements and leases, and M2 adds rows as a third reader.

1. **One owner.** Residency owns the publication: what it contains and whether it is preparing, ready or shown.
   The route's row writes are part of its preparation and report to it. No reader decides alone when to show new
   content. `thrum-render` gains no dependency on `thrum-asset-gpu`; the game connects the two, as it connects
   residency's other handles.
2. **Prepare without showing.** Each reader's replacement is made resident beside what is drawn, under the
   bounds it already has: residency's copies under the upload budget, a row's geometry under the route's bound.
   No frame reads prepared content.
3. **Ready is the slowest reader.** A placement set can be shown in the frame that records its last copy, because
   copies come before draws in a frame. A lease can be shown once that frame is submitted. A row can be switched
   once the frame carrying its last write is submitted. A publication is ready when every reader the scene has
   can show it. So in a scene with a lease or a cooked row it becomes ready at a submit, and the earlier readers
   wait for the slowest: placements one frame for a lease, and every reader as many frames as the longest row
   write takes. In a scene with only placements nothing changes from today.
4. **One switch frame.** In a scene with a lease or a cooked row, from the submit that makes a publication
   ready, every frame the host records shows it through every reader: placements draw the new set, leases offer
   it, and each changed row's readers switch. Which generation a frame shows is fixed before the frame is
   extracted, so every reader takes the same answer whatever the order of contributions in the frame.
5. **Tentative, like every recording.** The switch commits for every reader when its frame is submitted, a
   capture frame included, and the old content is retired then. A discarded switch frame commits nothing for any
   reader, and the next frame records the same switch.
6. **Newest wins until ready.** A publication superseded while it prepares is abandoned for every reader:
   nothing of it is shown, and its staging and arena space are given back. A ready publication is not superseded.
   It switches in the next submitted frame, and the newer one prepares after it. This is what lets rule 4 hold
   without an order between contributions. With only placements, residency keeps today's rule: the newest wins
   until the frame that draws the set is submitted.
7. **Refused whole.** When the running game cannot take one reader's part (a product the manifest lacks, an
   invalid one, one the device cannot hold, a failed upload, a row the arena refuses), the publication is refused
   for every reader. All of them keep the last good generation, and the record names the product, the reader and
   the reason. The store's manifest has still moved; what stays is residency's last good resident generation.
   Each later publication is tried the same way, so once the cause is fixed everything saved since appears in one
   switch. A fault the cooker detects never gets this far: the manifest keeps that asset's previous product and
   the rest of the save publishes.
8. **First load, resize and a new device.** A reader with nothing resident shows nothing: no placements, an
   absent lease. That is loading, not two versions of a product. Rows are prepared with the scene, from one
   generation; when the first publication on a device is of another generation, it carries the row replacements
   and switches once. A resize or a device replacement that discards prepared content sends the publication back
   to preparing.

The cost is latency, never a frame's time: with a lease attached a texture edit shows one frame later than it
does today, and a mesh that entities draw holds its publication for the frames its row write takes. The switch
frame itself records no copy and no row write.

### Failure (M1)

A staging chunk that cannot be filled frees its slot, records the error once, and the render thread drops the
pending set; the drawn and lent sets stay. wgpu cannot be made to fail a mapped range on demand, which is why
this path has no test. M1 adds a fault probe, compiled only on request as `draw-probe` is, at the point where a
chunk is mapped or written, so the production branch is the code that runs.

### What a publication promises (M3)

G5 publications are **previews**: the watcher cooks working files, not an admitted source revision. Three
promises hold for every publication and one does not. The items claim only the first three.

**Held: presentation only.** Nothing in simulation, collision, navigation or region activation reads a cooked
product, so no publication can activate a gameplay product. This is the reading of C05's "Filesystem watcher
events cannot activate a partly written external change set" that
[the G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md#contract-questions-recorded-reversible-option-taken)
put in force, and it stays in force.

**Held: a bad save keeps the last good content.** Every case in C4's range leaves the last good content drawn
and lent, with the reason recorded. Where the cooker finds the fault, the asset's previous product stays in the
manifest. Where the running game finds it (a device refusal, a failed upload, a missing name, an arena refusal),
the manifest has moved and residency keeps its last good resident generation
([rule 7](#one-publication-one-switch-m1-m2)).

**Held: one switch for every reader.** A publication's products change through placements, leases and mesh rows
in the same submitted frame ([one publication, one switch](#one-publication-one-switch-m1-m2)). This is the
reader half of C05's qualified visual preview, met as the contract words it, and C05 is not changed. It concerns
cooked products only, so it holds whether or not their sources were admitted.

**Not held: a save has no boundary.** The watcher closes a batch after 8 ms of quiet, 250 ms at most, and neither
it nor the cooker knows where an external save ends. An incomplete save that still validates is cooked and shown:
a changed glTF over its old buffer when the accessors still fit, or a file an editor rewrites in place and pauses
on. The creator sees that intermediate content, whole in C3's sense, until the rest of the save lands. The next
cook then publishes exactly what a fresh cook of the finished sources publishes, because a product is keyed by
the bytes of every input. Refusing such a save needs source admission, which is cooking from T1's captured tree,
and that is not built here. Readers confined to presentation, an atomic manifest and whole resident sets do not
supply it, and no item claims it.

The limit is an assumption in [the plan](PLAN.md#open-for-the-owner) for the owner to overturn.

### Closing G5 (M3)

The built evidence is offscreen with no present on the Mac and function-only in Chrome. M3 runs three edits in
the running valley with reconstruction on, on the Mac and in Chrome, and proves C2 and C3 there: a mesh that is
both placed and an entity's mesh row, a texture that a placement samples and a lease lends, and a leased volume.
Two of the three change a product read through two paths, so the cross-reader switch is judged where the exit
shows it. The game's frame record names the generation each reader showed in that frame, and the edit windows
must never show two. "Without a stall" is judged three ways, none of them a
budget: deterministic bounds asserted in the game (copy bytes per frame, staging peak); the existing tripwires
against their load-class history; and the game's own frame records, edit windows beside steady windows of the
same run. Panel cadence stays with `PLATFORM-RUNTIME-MAC`.

### Check that it can work

- **Lease commit.** The contribution seam already commits a set only in `submitted`, and the encoded-frame token
  allows one unsubmitted frame at a time, so "the last committed set" has one meaning. Lending it adds a reader,
  not a new state.
- **Twelve volumes.** A 128³ R8 volume with mips is 2.3 MiB; twelve are 27 MiB, about four frames at the game's
  8 MiB budget, and 14 MiB as BC4. Staging stays at its 32 MiB bound whatever the set's size.
- **Binding shape.** `R8Unorm` and `Bc4RUnorm` are both filterable float formats, as are the three texture
  profiles, so one layout serves every profile.
- **Row replacement.** Every reader of a row reads data, never pipeline state: `BucketRoute`'s ranges, the mesh
  records, the arena buffers and the caster set are all buffers or CPU tables. Occlusion's proof needs only that
  a mesh lies inside its row's bounds, which admission recomputes. A grown mesh with stale bounds would lose
  pixels, which M2.2's oracle shows.
- **Frames in flight.** A submitted frame executes before any later write, by queue order. An unsubmitted one
  holds the host, so no write or switch can be recorded behind it. That leaves three orderings to prove, and M2.3
  proves them: a discarded switch frame, a newer edit that arrives behind a discarded switch frame, and a newer
  edit that supersedes a replacement still being written.
- **One switch.** Each reader's switch only changes which resident data a frame reads: a bind group for
  placements, a view for a lease, ranges and tables for a row. None needs a copy in the switch frame, so one
  frame can make all of them once everything is resident. Readiness changes only in `submitted`, which the host
  calls on every contribution after its own commit (`runtime.rs`), and the host holds one unsubmitted frame at a
  time, so "ready before this frame" has one meaning for every reader. A reader that could show sooner only
  waits; it holds prepared content, which the overlap account below already counts.
- **An incomplete save converges.** The action key hashes every input's bytes, so the cook after the last file
  lands cannot reuse a product of the intermediate state. M3.7 checks it against a fresh cook.
- **Chrome.** A fetched feed has no loader thread: verification, conversion and staging run on the page's thread
  between frames. That is cheap at showcase sizes (the valley's whole first load is under 3 MiB) and grows with
  size, which is why C2's range in Chrome is narrower than on the Mac.

## The system as it runs

| State | Owner |
|---|---|
| Store, `HEAD`, manifest, product formats, the cooker, watcher and dev server | This module (`thrum-asset`, `thrum-cook`) |
| Resident textures, volumes and placement meshes; the loader, staging ring and upload budget; leases; conversion of mesh products to row geometry | This module (`thrum-asset-gpu`) |
| The publication: which generation each frame shows, through every reader, and whether the next is preparing, ready or refused | This module (`thrum-asset-gpu`) |
| The mesh arena, rows, buckets and their culling; the bounded row write, its report of done, and the row's switch in the frame the publication names | `CULLING` (`thrum-render`), extended under the seam ruling |
| The contribution list and the tentative-frame protocol | `EFFECT-GRAPH` |
| The view record, motion, reactive target and history acceptance | `APPLE-PRESENTATION` |
| A lease's meaning: placement, extent, optics, the consumer's own history | The consumer (`SEA`, `CLOUDS`) and the game |

**An edit, in order.** The watcher sees the save, the stat cache rehashes only that file, the cooker cooks only
the changed asset for each profile, and `HEAD` moves. The loader's next poll maps the new manifest, resolves
every placement, lease and row by name, keeps each product whose hash is unchanged, and plans copies for the
rest. It fills staging chunks off the render thread. Each frame records at most a budget of copies, and a changed
row's geometry goes to the host, which writes it under its own bound. When a submitted frame has carried the
last copy and the last row write, the publication is ready. The next frame draws the new placements, offers the
new set to leases and switches the rows, and the switch commits for all of them when that frame is submitted
([one publication, one switch](#one-publication-one-switch-m1-m2)).

## What later modules will ask

| Module | Takes from M1 to M3 | Will ask for, not built now |
|---|---|---|
| `CLOUDS` | A lease of density volumes with mips, identity and the change signal; CPU reads of the portable product | A cook-time atlas if its cost account wants one texture; an iPhone encoding |
| `SEA` | A lease of a linear 2D texture for the foam pattern (a `.linear` source) | A single-channel 2D format, if four channels are wasteful |
| `RENDER-2D`, `GAME-UI` | Leases of 2D pages | Sprite-sheet and font product kinds (frames, pivots, glyphs) read through `thrum-asset`, array textures, and a profile preference that keeps pixel art uncompressed |
| `SHOWCASE` | Placements, and cooked mesh rows for its entities | Nothing more |
| `CREATOR` | Unchanged placements | The sky kit passes a lease through, when a sea or cloud demo scene needs one |

Each ask becomes a milestone here, planned with the module that needs it.

## Shared files and exact baselines

`thrum-asset-gpu` is also edited by `APPLE-PRESENTATION` M2 (the reactive target in `draw.rs`), which is why M1
waits for it. M2 edits `CULLING`'s route files and waits for `CULLING:M3`. `scripts/check_v1_browser.mjs` is
shared with every module that has a Chrome case; add cases, change none.

[The module list](../ARCHITECTURE.md#exact-baselines) says how a change to another module's exact images is
caught. This module must move none: a scene with no lease and no cooked rows compiles to the same passes and
draws the same bytes. Every milestone carries the verifier as its claim-time check. `CULLING`'s relative
comparisons (each mode against its oracle) and `APPLE-PRESENTATION`'s temporal witness, which drives the cooked
volume, belong to those modules and are not loosened.

## Cost account

- **No lease, no cooked rows:** nothing is added to any frame.
- **A steady frame with a lease:** one comparison of a serial per lent product.
- **An upload frame:** at most one budget of GPU copies from residency (8 MiB in the game) and at most the
  route's bound of row bytes, written on the render thread. An earlier render-thread write of 16 MiB cost 1.4 to
  1.9 ms on this Mac, so the row bound is set from that and recorded.
- **Staging:** at most four chunks of one budget, whatever is loading.
- **A switch frame:** bind groups for the changed placements and the row's table writes; no copy.
- **Latency:** an edit shows when its slowest reader is ready. Recorded per publication as frames from first
  copy to switch.
- **Memory:** live products, plus old and new for a publication's changed products from its first copy until its
  switch commits, plus retired arena space until it is reused. Reported from descriptors; physical residency is
  not measured.

Recorded as tripwires, never gates: the two G5 benches against their load-class history, with a leased scenario
added; render-thread time on upload frames beside steady frames; and about 25% paired regression on the valley
route with nothing attached.

## Risks

1. **The lease is the wrong shape for a real consumer** (it wants a product before commit, or partial sets).
   Early sign: the `CLOUDS` or `SEA` cost account. The rules above reopen here before anyone builds around them.
2. **Row replacement breaks an exact culling equality.** Early sign: M2.2 in any mode. The oracle is a fresh
   scene, so there is nothing to tune; stop and diagnose.
3. **Chrome loses frames on an edit.** Early sign: M3's interval record. The fallback is named under research.
4. **The fault probe tests itself.** A probe that returns an error above the failure branch proves nothing; the
   item asks a reviewer where it sits.
5. **Tripwires on a shared machine flap.** They are judged per load class and catch doublings; a failure is
   investigated, never re-recorded to pass.
6. **A cooked-draw change moves a retained image.** The proof gate refuses it at every landing.
7. **Saves arrive faster than a publication prepares, so each is superseded and nothing shows.** Residency has
   this rule for placements today, and a row write lengthens the window. Early sign: superseded publications
   counted in the game's record during M3's edits. The answer is to finish the one being prepared before taking
   the next, decided here first.
8. **A creator takes an intermediate preview for the result.** It lasts until the save finishes and is replaced
   without a restart. M3.9's documentation says so; source admission removes it.

## Proof and builder handoff

- **Targets.** `mac-metal`, and `chrome-webgpu` through `scripts/check_v1_browser.mjs`, which needs a browser
  launch the Codex sandbox grants per run. `ios-sim` is not a target here: the simulator's cooked function was
  accepted in `COOKED-DRAW` M1 and nothing in this plan changes its draw path.
- **Capture.** Inspected items use T2 engine capture in `realtime` mode and judge consecutive frames at 1x.
  Frame records for C2 are taken with capture off; the proof records whether a drawable was presented and does
  not depend on it.
- **Fixtures.** Tests cook their own products with `thrum-cook`, as the in-game G5 tests do. The twelve-volume
  fixture is generated. Nothing is lifted from Broadsides: its cloud library waits on the owner's licence.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/content-loop-<milestone>/`; selected
  evidence in `docs/evidence/CONTENT-LOOP/`.
- **Maturity.** The ledger's G5 row is brought up to date in M3, with implementation, correctness, visuals and
  device results kept apart.

## Open questions

| Question | Settled by |
|---|---|
| Which encoding serves volumes on iPhone: R8 as today, ASTC sliced 3D where the record allows, or the parked layered EAC draft with its recorded defect? | Owed. Recorded in the ledger at M3; built when a physical device or `CLOUDS` asks |
| Does `CLOUDS` want separate volumes or one atlas? | Its cost account; an atlas is a cook-time ask here |
| At what product size does Chrome first lose a frame on an edit? | M3 records it |
| How large may the route's per-frame row write be? | M2 measures render-thread time and records the bound |
