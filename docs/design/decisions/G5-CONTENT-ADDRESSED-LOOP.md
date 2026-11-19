# G5 decision: content-addressed development loop

**Selected and implemented.** CONTENT-LOOP M3 exercised live mesh, texture and volume edits in the V1 valley with
TAAU on this Mac (Metal) and hardware Chrome (WebGPU), both on the portable recipe. The Metal tests used offscreen
submissions. Chrome timings cover page callbacks at the tested product sizes. Earlier COOKED-DRAW evidence covers
indexed-path parity and simulator cooked drawing, rotation and lifecycle as function only. Physical iPhone
measurements remain open. [CONTENT-LOOP M3](../../../.orchestrate/evidence/CONTENT-LOOP/M3.md) ·
[COOKED-DRAW M1](../../plans/COOKED-DRAW/DONE.json)

The original decision was recorded on 6 November 2026 and revised that night for the in-game seam. Implementation
began before the first written checkpoint. An independent reviewer derived requirements from the brief and
contracts before reading the record or code. The [original G5 report](../../../.orchestrate/evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md)
records the findings and their dispositions. This decision governs G5 under
[C05](../CONTRACTS.md#c05-assets-streaming-and-retirement--f4-with-w2-v1-and-p7) and the
[hardware-native decision](HARDWARE-NATIVE-ENGINE.md), and covers the cooked-product half of
[TOOLS-DELIVERY](../modules/TOOLS-DELIVERY.md).

## Commission and boundary

A creator edits a source file, and the running game shows the new product without stalling its frame. Shipping builds
map the same products, and they never link the cooker. Three engine packages own the content loop.

- **`thrum-asset`** is the runtime product reader. It provides the CAS layout, `HEAD`, the manifest,
  mesh and KTX2 views, validation, `mmap` and `HeadWatcher`. It needs only blake3, bytemuck and memmap2 (no memmap2 on
  Wasm), and it has no workspace dependencies.
- **`thrum-cook`** is authoring-only. It provides the stat cache, CAS writer, action cache, mesh, texture and volume
  cookers, atomic publish, watcher, dev server and CLI. No shipping package may depend on it: the architecture registry
  enforces this, and iOS also forbids its packages.
- **`thrum-asset-gpu`** is shipping GPU residency: a loader thread, a staged upload belt, meshlet culling and the
  cooked mesh/volume draws. It depends on `thrum-asset` and `thrum-render` (never `thrum-cook`) and plugs into the
  game's scene through `thrum_render::contribution::SceneContribution`.
- **`examples/asset-loop`** stays a standalone shipping-shaped viewer with its own device. It does not replace the V1
  game's G3s sky or frame clock.
- **V1 game.** `THRUM_ASSET_STORE=<store>` attaches `CookedAssets` to the window binary's host
  (`examples/v1-game/src/cooked.rs`); without it the game is unchanged. The macOS device requests the adapter's
  compressed-texture families through `DeviceCapabilities::requested_content_features`.

## In-game seam (the scene contribution)

The contribution never owns a device, queue, submission, lighting mode or clock. It runs inside the scene's own frame
graph, in passes the graph declares, orders and times:

1. `rebuild(SceneSurface)` on every scene preparation, visual republication and device reconstruction. The surface
   lends the composed scene shader source, its pipeline/material layouts, the scene's fragment entry and the target
   formats. Generation-bound pipelines and motion history restart here.
2. `contribution upload`, a graph Copy pass declared before forward, records staged copies under a byte budget.
3. `contribution compute`, a graph Compute pass with automatic named timestamps, runs the meshlet cull, then copies
   out the visible count.
4. The forward pass calls `draw` after the scene's own geometry, with groups 0, 1 and 3 still bound. Cooked meshes
   compile the scene's shader source plus a `cooked_vertex` entry and use the scene's fragment entry, so they get the
   selected G3s lighting (Sun, SH sky, local lights, received shadows, GGX reflections, aerial perspective) with no
   second lighting authority. The scene source is an internal, versioned seam, not a stable WGSL ABI.
5. `submitted()` after the host submits the encoder.

**Recording is not publication.** A frame can be encoded and then discarded (no drawable, a timing or encode
failure). Copies, the swap to a new set, history validity and the readback state are tentative until `submitted`.
The next upload starts from the last committed copy, so a discarded frame's copies are recorded again. The host
calls `submitted` only after `queue.submit` (frame and capture paths); targets and timings are recreated when a
contribution declares passes, and attaching refuses while frames are in flight.

**Upload belt.** Each device generation uploads through one bounded ring of at most `STAGING_CHUNKS` (4) reusable
staging chunks of one frame's budget each. The loader plans a publication as copies of at most 1 MiB (rows re-strided
to 256 bytes; 3D textures split by slices, 2D by block rows) packed in order into chunks, posts it at once, and then
fills chunks off the render thread as they free up, waiting (never the render thread) when all are in use. It fills in
slices of one HEAD poll interval and polls between them, so a newer publication is posted while an older one is still
filling. A remote feed has no thread (the browser) and fills whatever is free each frame on the thread that renders,
without waiting. A chunk that cannot be filled fails the upload: its slot is freed, the loader records the error and the
render thread drops the set. The render thread records up to `upload_budget`
bytes of copies per frame (the game uses 8 MiB) from filled chunks only; a frame whose next chunk is not filled records
nothing. Once a chunk's copies are submitted it is mapped again, and the map completes, returning it to the ring, when
the GPU has read it. A set becomes drawable in the submitted frame that records its last copy, because those copies
precede forward in the same command buffer. The newest publication wins; a superseded pending set returns its chunks.
Resources are reused by content hash and recycled only after the render thread reports the set resident.

**Meshlets.** The cooker stores global `u32` indices in meshlet order alongside meshlet-local vertex and triangle
tables. The loader stages the product's existing index slice. For the indirect path it derives draw ranges from
meshlet triangle counts. A compute pass tests each meshlet's bounds against the frustum planes and its normal cone
against the camera, then writes `DrawIndexedIndirect` arguments that forward consumes with
`multi_draw_indexed_indirect`. Rejected meshlets get zero-instance arguments.

Automatic selection uses the indirect path only with known adapter-read `INDIRECT_EXECUTION`. When the fact is
absent or unknown, each placed mesh uses one full-range indexed draw without cull/range/bounds/count/readback
resources. Shared history and submission state serve both paths. The opt-in `draw-probe` feature checks direct
against independent full-range indirect arguments, with missing-mesh controls.
[COOKED-DRAW evidence](../../../.orchestrate/evidence/render/cooked-draw/README.md).

An explicit `MeshStages` request can prepare the Metal object and mesh pipelines on the Apple tile recipe when
the device has enabled the stages. Eligible cooked placements use those pipelines in forward and in the Sun's
maps when shadows are enabled. The loader stages their meshlet tables within the upload budget. An unsupported
request or failed preparation falls back to the selected indexed path. A mesh that exceeds a stage limit falls
back individually. The separate [MESH-SHADING M1](../../../.orchestrate/evidence/MESH-SHADING/M1.md) and
[M2](../../../.orchestrate/evidence/MESH-SHADING/M2.md) records cover this opt-in path on this Mac.

**Motion history.** Each placement's instance record carries a history flag. It is zero for the first submitted frame
of a new or replaced product, after a rebuild (device or visual republication) and on a scene history reset
(`ContributionFrame::history_reset`). The vertex entry then reports `history_valid = 0` with zero motion.

**Materials.** Each mesh placement carries its own ABI revision 2 base-colour factor, roughness and metalness.

**Volume.** A diagnostic content consumer, not a sky or cloud provider: single scattering with albedo one, an
isotropic phase, a short Sun light march and sky light approximated as uniform radiance `E_up/π`. It reads darker than
a real cloud (no multiple scattering). It raymarches back faces with depth test `Greater` and no depth write, so it is
not occluded by scene depth inside its box.

## Capability inventory

| Capability | Basis | Mechanism and owner | Status |
|---|---|---|---|
| Fast rescan | G5 text | Stat cache (length, mtime ns, ctime ns, inode). Entries younger than 1 s are rehashed (Git's racy-clean rule). Parallel stat and BLAKE3; a hash counts only if the stamp is the same after reading. | Implemented and tested; scan latency in bench |
| Content-addressed products | G5, C05 | `cas/<hh>/<blake3>`, written to a temporary file and renamed, read-only, deduplicated. The cooker re-verifies the published closure on open and removes damaged products. | Implemented and tested |
| Incremental cook | C05 "dependency-hashed" | Action key = BLAKE3(cooker version, encoder versions and meshlet limits, kind, profile, quality, colour space, each input path and hash). A hit skips the cook. The cook reads exactly the hashed bytes (meshes re-verify every input afterwards), so a save during a cook cannot poison the persistent cache. A reused product is verified once per process. One writer per store (OS file lock). | Tested: edit during cook, damaged action product, second writer |
| Atomic publication | C05, G5 | A manifest product, then the 64-byte `HEAD` (magic, generation, manifest hash, check), replaced by rename. Earlier generations stay readable. | Tested |
| Aligned mmap products | G5 | 64-byte-aligned sections with Pod little-endian elements; zero-copy typed slices; owned 64-aligned fallback on Wasm. | Tested; `mapped` asserted in the GPU test |
| glTF meshlets | G5 | glTF import, then meshopt weld, cache/overdraw/fetch optimisation and 64-vertex/124-triangle meshlets with cone bounds. One cooked product supplies global indices to the indexed paths and meshlet-local tables to the opt-in Metal stages. | Runtime layout tested. Indexed draws covered by COOKED-DRAW and stage draws by MESH-SHADING. |
| KTX2 ASTC/BC7 | G5, hardware-native | Own KTX2 writer and validated reader. BC7 via intel_tex_2 ISPC; ASTC 4×4 via the Arm astc-encoder (astcenc-rs); RGBA8 portable. Mips are built in linear light. | Tested. This Mac draws ASTC by default and BC7 when preferred (asserted in the game); BC4 volumes drawn |
| Baked volumes | Brief, cloud port | `.tvol` source cooks to 3D KTX2: BC4 sliced-3D for desktop, R8 for portable. EAC R11 for iPhone is not built. | Tested; drawn in the game as a diagnostic raymarch; 512³ R8 uploaded in-game |
| Profile selection | Hardware-native, C06 | The consumer orders profiles from one `DeviceCapabilities` record (ASTC → AppleAstc, BC → DesktopBc, else Portable). No private probes. The cooker always adds Portable. The runtime rejects partial compressed blocks; the loader rejects extents over device limits and unenabled formats, and records GPU validation errors instead of aborting. | Tested |
| Watcher | G5 | notify/FSEvents and an adaptive reconciliation sweep. An 8 ms quiet window closes a notice batch, up to 250 ms. The sweep checks the stat cache even without notices and defers dispatch on very recent unstable files. | Missed notices, the idle path and settling saves covered in [CONTENT-LOOP M4](../../../.orchestrate/evidence/CONTENT-LOOP/M4.md). |
| Dev server | G5 | Loopback HTTP: `/head`, a `/head?after=N` long poll, and immutable `/cas/<hash>`; `Host` checked; a loopback `Origin` or one listed with `--allow-origin` (exact `http(s)://host[:port]`, none by default) is echoed, any other is refused with 403. Client: `thrum-asset::DevClient`, target-independent protocol over a `Fetch` transport (`BrowserFetch`, the global `fetch`, on Wasm). It decodes `HEAD` with the native parser, never goes back a generation within an epoch, fetches the manifest and only the unheld products its per-kind profile preferences resolve, issues them together, and checks length and BLAKE3 before parsing. Products are 64-aligned owned bytes shared by reference count. | Server tested; client protocol unit-tested natively; Wasm client run end to end in Node and a Chrome page against `thrum-cook watch --serve`; allow-list near misses refused in tests |
| No-stall swap | G5 | Bounded staging ring and graph copies on the frame host. Placements, mesh rows and leases switch on one submission after the copies and row writes are ready. | In-game GPU tests, discard test and `cooked_edit` bench. [CONTENT-LOOP M1](../../../.orchestrate/evidence/CONTENT-LOOP/M1.md) and [M2](../../../.orchestrate/evidence/CONTENT-LOOP/M2.md) retain the lease and row proofs. |
| Drawn in the game | G5 gate | `SceneContribution` in the scene graph, with the scene fragment shader for placements, cooked entity rows and read-only texture and volume views for other passes. | Metal and Chrome T2 realtime submitted images show the three whole switches in the moving valley under TAAU. [CONTENT-LOOP M3](../../../.orchestrate/evidence/CONTENT-LOOP/M3.md) |
| Indexed cooked draws | G5 | Automatic selection uses GPU frustum/cone culling and indirect multi-draw when the adapter record confirms indirect execution. Otherwise it issues one full-range indexed draw per placement. | COOKED-DRAW M1 recorded exact Mac direct/reference parity in five views with negative controls, and exact Mac/Chrome indirect output and repeats. Simulator direct drawing, lifecycle and rotation passed as function, with held and sampled visual evidence. |
| Metal object and mesh stages | Hardware-native | Explicit `MeshStages` request, enabled device stages and the Apple tile recipe. Eligible cooked placements draw through the stages in forward and the Sun's maps. Indexed fallback covers unavailable pipelines and ineligible meshes. | MESH-SHADING M1 and M2 independently verified the opt-in path on this Mac, including culling, frame outputs and casters. Automatic selection remains indexed. |
| Shipping excludes authoring | G5 | Workspace dependency boundaries, normal package dependency graphs and a symbol scan of the release starter with the release cooker as the positive control. | `python3 scripts/g5_shipping_links.py` |

## Cost account

Ordinary work is designed around the frequent operations:

- **Scans run on every save.** A clean file costs one `stat`. Only changed or racy files are read and hashed: BLAKE3
  runs at gigabytes per second per core, and a 1 MB texture hashes in well under 1 ms.
- **Cooks run per changed asset.** A cook is the only expensive step (BC7/ASTC encoding dominates), and identical
  actions are never repeated across processes.
- **Publishing is O(entries).** It writes one manifest (64 bytes per entry) and one 64-byte `HEAD`.
- **The runtime polls cheaply.** A `HEAD` poll is one 64-byte read. Loading maps the manifest and does a binary search
  per resolve; mapping is O(1) and validation is O(sections), with reference validation linear and optional.
- **Remote loads fetch only changed products.** The dev client holds the resident generation's products by hash, so a
  publication costs one `HEAD` reply, one manifest and the changed products, each copied once from the JS heap into
  aligned memory and hashed once.
- **Uploads are only for changed products.** The loader keeps products by content hash, so unchanged products are
  reused (asserted in the GPU test).
- **The render thread pays almost nothing.** Per swap it does one `try_recv` and drops an `Arc`; it never runs
  `poll(Wait)`.
- **In-game frames.** On the indirect path, each placement records a compute cull dispatch (one thread per meshlet)
  and an indirect draw. A direct placement issues one indexed draw without the cull dispatch. Eligible mesh-stage
  placements cull in the object stage. An upload frame records at most `upload_budget` bytes of GPU copies through
  the residency belt. The `cooked_edit` bench records the upload costs.
- **Staging peak.** At most `STAGING_CHUNKS` × `upload_budget` (32 MiB in the game) whatever the publication's size,
  measured at 32 MiB for a 149 MiB 512³ R8 replacement that previously needed one 149 MiB staging buffer. Chunks are
  created on first need and reused, so steady uploads allocate nothing. The cost is that filling is paced by the GPU
  returning chunks; the 512³ replacement still lands in 19–21 frames at 8 MiB per frame.
- **Peak residency during a swap** is live plus the replacement for changed products only. This matches the
  `M_replacement_overlap` term in C05; the account is logical, and physical residency is not measured.

Alternatives were rejected for these reasons:

- A database such as SQLite would add size and a Wasm cost to the shipping runtime, and the boundary notes forbid it.
- Serialisation frameworks give no zero-copy aligned view.
- Basis Universal transcoding spends runtime work and loses quality compared with native BC7/ASTC encoding at cook time.
- Polling mtimes alone misses atomic saves with the same size and time; a test covers this case.

## Worked journeys

- **First use.** `thrum-cook demo src`, then `thrum-cook watch src store`, then `asset-loop store`. The viewer polls
  `HEAD`, resolves its three names under its profile order, maps, uploads, swaps and draws.
- **Texture edit.** The editor saves `checker.png`, and FSEvents delivers it within the debounce window. The stat cache
  sees a changed stamp, rehashes and finds a new hash. Three action keys miss (one per profile), so three cooks run;
  the mesh and volume hit the cache. The new manifest is generation N+1, and `HEAD` is renamed. The loader's next poll
  sees it, maps only the new texture, uploads it, reuses the mesh and volume, and builds a new bind group. The render
  thread swaps it in on the next frame. The old texture is retired by wgpu after its last use, and its CAS file remains.
- **Broken edit.** A cook or planning failure (unreadable glTF, missing buffer, name collision) keeps the previous
  products of that asset bound; other assets that changed in the same cycle still publish. When the source is fixed,
  the next cycle publishes normally.
- **Crash during publish.** The products are already complete files and `HEAD` still names the old manifest, so the
  next open re-verifies and continues. With `Durability::Power`, files and directories are also fsynced. A damaged
  `HEAD` resumes after the newest manifest in the store, so readers never see a generation go backwards.

## Plausible wrong substitutes and rejecting observations

- **"Reload" that re-uploads everything.** Rejected because per-swap `uploaded_bytes` must fall for single-asset edits.
- **A frame-time stall hidden by an offscreen proof.** Rejected by the in-game bench: per-frame copy bytes never exceed
  the budget, render-thread CPU on copy frames stays near steady state, and a render-thread `write_texture` contrast
  shows the cost being avoided (12.4 ms of render-thread CPU for 128 MiB).
- **A seam that loses or duplicates uploads when a frame is discarded.** Rejected by the discard test: two hosts on one
  device and one clock, one discarding encoded frames mid-upload, swap on the same frame and capture identical pixels.
- **Cooked meshes lit by a second, simpler lighting model.** Rejected because they use the scene's own fragment entry;
  visible in captures as sky reflections and aerial perspective on the cooked mesh.
- **A swap that stalls the GPU or CPU.** Rejected by render-thread CPU p99/max during reload windows compared with
  steady state, and by GPU completion cadence, with a 1.3 MiB texture that crosses the 1 MiB upload chunk. Limits:
  offscreen with no present, and the staging copy at submit is outside the render-CPU span.
- **A cache that misses atomic saves.** Rejected by the same-size, same-time rename test.
- **An image that did not actually change.** Rejected by mean-colour and pixel-difference checks on captured frames,
  plus agent visual review.
- **A shipping binary that only appears clean.** Rejected by a symbol scan of the release binary as well as its
  dependency graph.

## Contract questions (recorded; reversible option taken)

1. **C05 partial change sets.** The watcher cooks working files, which T1 calls a non-admitted dirty tree. The partial
   write risk is reduced by the debounce, stamp-stable hashing and product validation. However, a glTF and its `.bin`
   saved separately can publish one intermediate state.
   - **Safe default in force:** G5 publications are *preview* publications. They activate presentation products
     only, including placement inputs, mesh rows and lent textures and volumes. Simulation, collision, navigation
     and region activation have no readers on this path. A valid intermediate preview can remain visible until the
     remaining source files arrive. The finished preview converges to the product hashes of a fresh cook. An invalid
     or unresolvable product keeps the previous good content.
   - **Proposed for the owners (F4 with W2, V1, P7):** admitted builds cook from T1's captured tree through the same
     cooker API (`cook_changed` takes explicit paths); a multi-file glTF stays keyed as one asset through its input
     list; and the manifest gains a provenance section (source revision or `preview`) before any required product is
     published through G5. Recorded in the boundary note for the V1/render owner; no owner has objected.
2. **Renderer-owned residency.** Resolved with the render owner as the `SceneContribution` seam: residency lives in
   `thrum-asset-gpu`, its GPU work runs in the scene graph, and the root review's three required changes (submit-time
   commit, graph passes with timestamps, submitted history) are implemented.
3. **C06 device ownership.** In the game the contribution uses the host's device, queue and `DeviceCapabilities`
   record and never submits. The standalone viewer still creates its own device.
4. **Retention.** No CAS garbage collection is landed. A draft with OS-locked reader leases is parked on branch
   `claude/g5-parked-gc-leases`; review found two defects to fix before it lands: the collector's lease snapshot is
   not synchronized with new acquisitions (an older-generation lease taken after the snapshot can lose its
   products), and manifests are indexed by generation alone (a failed publication retried at the same generation
   can leave two manifests, and the wrong closure may be kept while live `HEAD`'s products are removed). As C05
   requires, cleanup must wait for leases to be released.

5. **Preview marking and build binding.** Publications record no source revision and are not marked preview. C05 asks
   for a build binding per admitted revision. Recommendation: add a provenance section to the manifest when T1
   admission exists.
6. **Residency admission.** The running loader validates formats and dimensions against its device. Swap observations
   account for logical bytes and each lease identifies its owning device generation. Full C05 world admission and
   physical residency measurements remain open.

## Open items

- EAC R11 iPhone volume products. A layered (2D-array) draft is parked on branch `claude/g5-parked-eac-layered`;
  review found that ETC2 alone selects it and a volume deeper than `max_texture_array_layers` is then refused instead
  of falling back to the R8 3D product. Portable R8 already serves the native consumer.
- The volume is not occluded by scene depth inside its box and is a diagnostic consumer only.
- CAS garbage collection remains parked. The retention note above records the outstanding reader-lease issues.
- Graph Copy passes are untimed engine-wide, so the upload's GPU cost is measured by the bench, not pass timestamps.
- Physical iPhone, sustained runs and physical residency measurements remain open. The iPhone simulator proves function.
- Parameter files as a cooked product and source admission remain parked. External saves have no admitted source boundary.
- Tripwires are asserted only by `scripts/check_g5_tripwires.sh`, not `./dev check` (they need the GPU and minutes).
  Timings are judged per load class. A metric needs three usable records before the check can compare it in
  that class. The reports identify metrics still seeding. Limits cover the shared machine's measured spread
  and catch large regressions.
  Recording appends to the baseline, so record only runs of accepted code, or the limits ratchet upward.
- The dev server opens one thread per connection (at most 32). A page from another device needs its exact origin listed
  and the server bound to the address that page reaches (listing origins refuses a wildcard bind, and a host name in
  `Host` is not accepted); there is no TLS or authentication, so it is for trusted
  local networks only.
- Texture levels below 16 texels a side are measured (21–36 dB) but not floored; only the portable product is a
  reference, so the quality check measures encoding loss, not source fidelity.

## Historical draw observations

The 8 November G5 follow-up closed mirrored-transform winding and normal sign, refusal of partial-block
compressed products, per-format texture quality floors and the Chrome page exercise of the Wasm dev-server client.
[Original G5 report](../../../.orchestrate/evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md)

On 8 November a fetched Chrome mesh edit lengthened the shadow as the geometry changed. Cooked caster bounds
are rebuilt with each resident set. Later COOKED-DRAW M1 simulator views filled both PLATFORM-RUNTIME-IOS M7
oriented extents. The sixteen sampled pan images had a largest gap of 1.118 seconds.
[COOKED-DRAW evidence](../../../.orchestrate/evidence/render/cooked-draw/README.md)

The COOKED-DRAW paired release Mac runs used the valley at 1920×1080 under quiet load, with four alternating
indirect/direct pairs and 128 samples per arm. Per-run median forward time was 1.240–1.244 ms indirect and
1.283–1.330 ms direct. Forward already includes the indirect compute contribution of 0.01025–0.01048 ms.
Encode CPU was 0.270–0.289 ms indirect and 0.149–0.188 ms direct. These measurements describe the indexed
paths in that run. They confer no promotion or cadence qualification.
