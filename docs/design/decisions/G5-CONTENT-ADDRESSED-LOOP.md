# G5 decision: content-addressed development loop

6 November 2026. **Selected and implemented; native correctness qualified on this Mac; browser and iPhone clients
unqualified.** This is the governing design for build-plan item G5. Disclosure: implementation began before this record
was saved. An independent reviewer agent derived the requirements from the brief and contracts before reading this
record or the code; its findings and their dispositions are in the evidence report. The design was held in the working session and written down at the first checkpoint, so this is not a blind
design study. It sits under [C05](../CONTRACTS.md#c05-assets-streaming-and-retirement--f4-with-w2-v1-and-p7) and the
[hardware-native decision](HARDWARE-NATIVE-ENGINE.md), and covers the cooked-product half of
[TOOLS-DELIVERY](../modules/TOOLS-DELIVERY.md). Evidence: [G5 report](../../evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md).

## Commission and boundary

A creator edits a source file, and the running game shows the new product without stalling its frame. Shipping builds
map the same products, and they never link the cooker. The work splits into three packages:

- **`thrum-asset`** is the runtime, and the only package a game links. It provides the CAS layout, `HEAD`, the manifest,
  mesh and KTX2 views, validation, `mmap` and `HeadWatcher`. It needs only blake3, bytemuck and memmap2 (no memmap2 on
  Wasm), and it has no workspace dependencies.
- **`thrum-cook`** is authoring-only. It provides the stat cache, CAS writer, action cache, mesh, texture and volume
  cookers, atomic publish, watcher, dev server and CLI. No shipping package may depend on it: the architecture registry
  enforces this, and iOS also forbids its packages.
- **`examples/asset-loop`** is a shipping-shaped viewer that uses `thrum-asset` and `thrum-render` only. Its GPU
  upload/residency code is the consumer seam that a future renderer-owned loader replaces. It does not replace the V1
  game's G3s sky or frame clock.

## Capability inventory

| Capability | Basis | Mechanism and owner | Status |
|---|---|---|---|
| Fast rescan | G5 text | Stat cache (length, mtime ns, ctime ns, inode). Entries younger than 1 s are rehashed (Git's racy-clean rule). Parallel stat and BLAKE3; a hash counts only if the stamp is the same after reading. | Implemented and tested; scan latency in bench |
| Content-addressed products | G5, C05 | `cas/<hh>/<blake3>`, written to a temporary file and renamed, read-only, deduplicated. The cooker re-verifies the published closure on open and removes damaged products. | Implemented and tested |
| Incremental cook | C05 "dependency-hashed" | Action key = BLAKE3(cooker version, encoder versions and meshlet limits, kind, profile, quality, colour space, each input path and hash). A hit skips the cook. The cook reads exactly the hashed bytes (meshes re-verify every input afterwards), so a save during a cook cannot poison the persistent cache. A reused product is verified once per process. One writer per store (OS file lock). | Tested: edit during cook, damaged action product, second writer |
| Atomic publication | C05, G5 | A manifest product, then the 64-byte `HEAD` (magic, generation, manifest hash, check), replaced by rename. Earlier generations stay readable. | Tested |
| Aligned mmap products | G5 | 64-byte-aligned sections with Pod little-endian elements; zero-copy typed slices; owned 64-aligned fallback on Wasm. | Tested; `mapped` asserted in the GPU test |
| glTF meshlets | G5 | gltf import, then meshopt weld, cache/overdraw/fetch optimisation, and 64-vertex/124-triangle meshlets with cone bounds. Meshlet-ordered indices let the portable indexed path and a future mesh-shader path share one product. | Tested; meshlet draw path not built |
| KTX2 ASTC/BC7 | G5, hardware-native | Own KTX2 writer and validated reader. BC7 via intel_tex_2 ISPC; ASTC 4×4 via the Arm astc-encoder (astcenc-rs); RGBA8 portable. Mips are built in linear light. | Tested. This Mac draws ASTC textures and BC4 volumes (asserted and recorded); BC7 is cooked and validated but not drawn here |
| Baked volumes | Brief, cloud port | `.tvol` source cooks to 3D KTX2: BC4 sliced-3D for desktop, R8 for portable. | Tested; cook and map only, as commissioned |
| Profile selection | Hardware-native, C06 | The consumer orders profiles from one `DeviceCapabilities` record (ASTC → AppleAstc, BC → DesktopBc, else Portable). No private probes. The cooker always adds Portable. The runtime rejects partial compressed blocks; the loader rejects extents over device limits and unenabled formats, and records GPU validation errors instead of aborting. | Tested |
| Watcher | G5 | notify/FSEvents. Events are only hints: an 8 ms quiet window closes a batch (250 ms maximum), `need_rescan` triggers a full scan, and the stat cache stays the authority. | Implemented; latency in bench |
| Dev server | G5 | Loopback HTTP: `/head`, a `/head?after=N` long poll, and immutable `/cas/<hash>`; `Host` checked, loopback `Origin` echoed. Client: `thrum-asset::DevClient`, target-independent protocol over a `Fetch` transport (`BrowserFetch`, the global `fetch`, on Wasm). It decodes `HEAD` with the native parser, never goes back a generation within an epoch, fetches the manifest and only the unheld products its per-kind profile preferences resolve, issues them together, and checks length and BLAKE3 before parsing. Products are 64-aligned owned bytes shared by reference count. | Server tested; client protocol unit-tested natively; Wasm client run end to end in Node against `thrum-cook watch --serve`; no browser page run |
| No-stall swap | G5 | A loader thread maps, validates and uploads into new resources in 1 MiB chunks. The render thread exchanges a pointer between frames and never waits; wgpu retires old resources after their last submission. | GPU test plus hitch bench |
| Shipping excludes authoring | G5 | Registry edges plus a dependency-graph and symbol check of the release viewer binary. | Recorded in evidence |

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
   - Recommendation: treat the G5 loop as *preview* publication.
   - Admitted builds cook from T1's captured tree through the same cooker API (`cook_changed` takes explicit paths).
   - A multi-file glTF should be keyed as one asset, which it already is through its input list.
2. **Renderer-owned residency.** The viewer's upload code belongs in `thrum-render`, behind a small seam that takes a
   mapped product and returns a generation-bound resource. Moving it needs agreement from the render owner (G3b/G3s),
   so it stays in the example for now.
3. **C06 device ownership.** The viewer creates its own device because it is a standalone process. In-game use must
   take the coordinator's device and its `DeviceCapabilities` record.
4. **Retention.** No CAS garbage collection exists yet. It needs leases from every reader generation. As C05 requires,
   cleanup must wait for leases to be released, and this is deferred until a consumer exists.

5. **Preview marking and build binding.** Publications record no source revision and are not marked preview. C05 asks
   for a build binding per admitted revision. Recommendation: add a provenance section to the manifest when T1
   admission exists.
6. **Residency admission.** The loader uploads replacement sets with no logical byte account against C05 `M_peak`.
   Recommendation: report replacement bytes to the future F4 owner. Prepared sets carry no device generation (C06); the
   renderer-owned loader must add it.

## Open items

- EAC R11 iPhone volume products.
- Mirrored-transform winding and normal sign in glTF import.
- Rejection of compressed textures whose base size is not a multiple of 4.
- A Chrome page run of the Wasm dev-server client (the CORS path); it is tested natively and in Node only.
- V1-game integration that keeps the G3s lighting (see the boundary note).
- Meshlet draw path (mesh shaders / GPU-driven, tied to G3c).
- Texture quality comparison (PSNR against the source).
- Physical iPhone, Chrome and sustained measurements.
- Tripwires are recorded, not asserted; no regression history yet.
- The dev server opens one thread per connection (at most 32). Only loopback origins may read it; a page served from
  another device needs an explicit origin allow-list before wider use.
- An asset whose compressed profile fails permanently (for example a partial-block size) keeps serving its last good
  compressed product while its portable product updates.
