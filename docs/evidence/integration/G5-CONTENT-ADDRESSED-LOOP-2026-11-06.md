# G5 content-addressed loop — 6 November 2026

Governing design: [G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md). Measured on an Apple M3 Max
(macOS, Metal) with rustc 1.99.1, release bench, on a machine other agents were using. The timings are tripwires, not
budgets.

## What was built

- **`thrum-asset`** is the shipping runtime. It provides:
  - the CAS layout and the 64-byte `HEAD`;
  - an aligned container with typed zero-copy views;
  - the manifest, a mesh product with meshlets, and a validated KTX2 reader (R8, RGBA8, BC4, BC7, ASTC 4×4; 2D and 3D);
  - `mmap` with an owned fallback on Wasm;
  - `HeadWatcher`.
- **`thrum-cook`** is authoring-only. It provides:
  - a stat cache with the racy-clean rule, plus a BLAKE3 CAS and an action cache;
  - a glTF → meshopt meshlet cooker;
  - a texture cooker: linear-light mips, BC7 (ISPC) and ASTC 4×4 (Arm astc-encoder);
  - a volume cooker: BC4 slices and R8;
  - atomic publication with recovery on open;
  - an FSEvents watcher, a loopback dev server and a `thrum-cook` CLI.
- **`examples/asset-loop`** is a shipping-shaped viewer. A loader thread maps and uploads, and the render thread swaps
  the new set without waiting. It chooses profiles from `DeviceCapabilities`.

## Tests (all in `./dev check`)

| Test | Proves |
|---|---|
| `cook_loop` (13) | Every kind and profile is cooked and accepted by the runtime. Unchanged sources are reused without republishing, across process restarts. An edit recooks only its asset, and old readers stay valid. A broken source keeps the previous product, and a deletion is published. Damaged products are removed and recooked on open. An atomic save with the same size and mtime is detected. Seven more tests cover the review findings below. |
| `edit_to_visible` (GPU) | Texture and mesh edits each become resident and visible. Products are mapped, and unchanged products are not re-uploaded (texture-only and mesh-only upload sizes). The image changes as each edit predicts. |
| `serve_and_watch` (2) | `/head` returns 204 then 200; served CAS bytes verify against their hash. Traversal and unknown paths return 404. A long poll is woken by publication. The watcher publishes an edit and a new file in a new directory. |
| `hostile_products` | More than 20k deterministic truncations, word overwrites and byte flips of real products. Every one is rejected or parsed without a panic. The `fuzz_products` example exposes the same parsers to an external fuzzer. |

## Measurements (`cargo bench -p thrum-cook --bench edit_to_visible`, base 614156e plus the G5 change)

**Source scans** use 4 KiB files and are trusted by stamp:

| Files | Cold | Warm, no change (median of 7) | One change | Watcher-targeted |
|---|---|---|---|---|
| 1,000 | 13.5 ms | 2.9 ms | 2.9 ms | 0.04 ms |
| 10,000 | 127 ms | 27.3 ms | 28.2 ms | 0.06 ms |

A full rescan is linear in the number of files. The watcher path re-checks only the named paths.

**Edit to visible.** The run made 12 edits 500 ms apart while the viewer rendered at 120 Hz. The source set is a
129×129-vertex grid, a 1024² texture cooked for three profiles, and a 64³ volume. The drawn profiles were ASTC 4×4 for
the texture and BC4 for the volume. A texture product is 1.3 MiB, so it crosses the 1 MiB upload chunk. Medians:

| Stage | Texture edit (n=8) | Mesh edit (n=4) |
|---|---|---|
| Write → watcher event | 19 ms | 11 ms |
| Event → publish, including 8 ms debounce | 59 ms | 35 ms |
| ↳ Cook (3 profiles, or 1 mesh) | 49 ms | 24 ms |
| Publish → loader sees `HEAD` | 0.6 ms | 0.7 ms |
| Map, validate, upload | 0.56 ms | 0.49 ms |
| Prepared → swapped (waits for next frame) | 5 ms | 7 ms |
| Swapped → GPU completion of first frame using it | 8.4 ms | 8.6 ms |
| **Write → visible** | **93 ms (max 97)** | **62.5 ms (max 67.6)** |

The write → event column is measured from the start of writing, so it includes the PNG encode.

All 12 edits published and swapped, with no loader errors. Uploads were 1,365 KiB per texture edit and 774 KiB per
mesh edit; nothing else was re-uploaded. An earlier run with a 512² texture, entirely under the chunk size, measured
58 ms median.

**Hitch tripwire:**

| Measure | Steady (n=828 / 760) | Swap frames (n=12) or within 50 ms of a swap (n=79) |
|---|---|---|
| Render-thread CPU per frame | p50 0.27, p99 1.23, max 3.44 ms | p50 0.13, p99 0.15, max 0.33 ms |
| GPU completion interval | p50 8.31, p99 10.61, max 13.5 ms | p50 8.30, p99 9.61, max 10.2 ms |

Swap frames cost no more than ordinary frames. The steady maxima came from frames with no swap, which is consistent
with scheduler noise on a shared machine. Limits: the run is offscreen with no present, and the queue's staging copy at
submit falls outside the render-CPU span. The GPU completion cadence covers that copy.

**Startup** to the first resident frame was 23.5 ms with the Metal device warm, and 199.7 ms on the first run of the
session. Most of that is adapter and device creation.

## Shipping excludes authoring

`python3 scripts/g5_shipping_links.py`:

- **Dependency graphs.** The normal graphs of `asset-loop` (71 packages), `v1-game` (85) and `thrum-asset` (13) contain
  no `thrum-cook`, meshopt, intel_tex_2, astcenc-rs, gltf, image or notify.
- **Symbol scan.** Searching the release `asset-loop` binary (12,683 symbols) for those crates' mangled names and C
  entry points finds 0 matches.
- **Positive control.** The same scan finds 2,858 matches in the `thrum-cook` binary.

The architecture registry forbids the dependency edge, and iOS rejects those packages.

## Visual review

Before, after the texture edit and after the mesh edit:
![before](../render/G5-BEFORE-2026-11-06.png) ![after texture](../render/G5-AFTER-TEXTURE-2026-11-06.png)
![after mesh](../render/G5-AFTER-MESH-2026-11-06.png)

To regenerate the stills and an 8-frame orbit sequence, run:
`THRUM_G5_CAPTURE_DIR=<dir> cargo test -p thrum-cook --test edit_to_visible`.

Review status: **agent_reviewed**. Both the author and an independent reviewer agent looked at the three stills and
the 8-frame orbit at native 320×180. These were stills, not video, and no pixel diff was run in review; the test's own
colour and difference checks cover that.

Findings:
- **Each edit changes only what it should.** The texture edit is colour only: the red/white checker becomes blue/white,
  and the geometry and inset are unchanged. The mesh edit is height only: the bumps rise, while the texture and inset
  are unchanged.
- **Colour and filtering look correct.** sRGB is correct, with no seams, mip moiré or visible block artifacts. The texture drawn
  was ASTC 4×4, not BC7 as the brief to the reviewer stated, and at this size the block check is weak.
- **Lighting is consistent across the orbit.** Shading is smooth and the light stays fixed in the world.
- **The inset is plausible.** It reads as a soft cloud-density slice, and its depth animates across the frames.
- **The apparent half-turn repeat is explained.** The reviewer saw frames 0/4, 1/5, 2/6 and 3/7 looking alike. The
  orbit steps 0.4 rad per frame, so those pairs are about 92° apart, and the demo surface cos 6x·cos 6z is 4-fold
  symmetric.
- **There is silhouette aliasing,** because the diagnostic viewer has no MSAA. This is accepted for a loader
  diagnostic and is not a G5 product defect.
- **Motion shimmer remains unreviewed,** because the review covered stills only.

## Independent design review and dispositions

A fresh reviewer agent derived the requirements from the brief and contracts, then read the design, code and bench.
Fixed, each with a regression test:

1. **Action-cache poisoning by a save during a cook.** The cook now reads exactly the hashed bytes, and meshes re-verify
   their inputs. Test: `a_source_edited_during_its_cook_is_not_cached_under_the_old_key`.
2. **Hostile or oversized KTX2.** Size arithmetic now saturates. The runtime rejects partial compressed blocks, and the
   loader checks device limits and formats and records GPU validation errors. Tests: `hostile_products` (wrapped
   extent) and `block_compressed_profiles_refuse_partial_blocks_but_the_portable_product_publishes`.
3. **Two sources for one name.** The asset fails and keeps its previous products. Test:
   `two_sources_for_one_name_fail_and_keep_the_previous_product`.
4. **Planning failure dropped the asset.** It now keeps the previous products. Test:
   `a_planning_failure_keeps_the_previous_mesh_published`.
5. **Damaged action-cache product, and damaged `HEAD`.** Reused products are verified once per process, `put`
   re-verifies an existing file, and a damaged `HEAD` resumes after the newest manifest. Tests:
   `a_damaged_product_behind_an_action_record_is_recooked_not_republished` and
   `a_damaged_head_does_not_restart_generations`.
6. **Two writers.** The store now has an OS file lock. Test: `only_one_cooker_may_write_a_store`.
7. **Watcher accepted hidden files.** It now matches the full walk. Test:
   `hidden_files_named_by_the_watcher_are_not_sources`.
8. **Loader skipped reference validation; the SAFETY comment overstated syncing.** Both are corrected.
9. **Unsupported claims.**
   - The "BC7 drawn" claim was wrong. This Mac draws ASTC/BC4, and the drawn profile is now asserted and recorded.
   - Chunked upload was never exercised. The bench now uses a 1.3 MiB texture.
   - The encoder versions and meshlet limits are now part of the action key.

Recorded rather than fixed:
- Dev server any-origin CORS and per-connection threads.
- No tripwire assertions.
- No preview marking or build binding.
- No device generation on prepared sets.
- No residency admission.

These are in the design record's contract questions and open items.

## Open items and contract questions

These are reported plainly, and the design record tracks them:

- **Not built:**
  - EAC iPhone volume products.
  - Meshlet draw path; the meshlets are cooked and validated, but drawn through the indexed path.
  - Browser dev-server client.
  - V1-game integration, which must keep the G3s lighting.
  - CAS garbage collection.
  - Texture PSNR quality check.
- **Not measured:** Chrome, physical iPhone, sustained runs, and physical residency during overlap.
- **Contract questions for C05/T1, recorded with recommendations in the design:**
  - Preview versus admitted publication of a partially written multi-file change set.
  - Moving GPU residency to `thrum-render`.
  - Device ownership in the game.
  - Retention leases.
