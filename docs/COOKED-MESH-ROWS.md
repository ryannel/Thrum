# Cooked meshes on entities

The V1 valley's existing `THRUM_VALLEY_MESHES=on` selection draws cooked `rock`, `sphere` and `rock`
rows. In Chrome, select `?assets=<dev-server>&meshes=on`. Materials remain the scene's material table.
The moving-route overlay supplies visible entity subjects at valley metres; native captures select it with
`THRUM_ROUTE_OVERLAY=proof`. The ordinary grid's entities remain at their original coordinates.

An engine consumer prepares `thrum_asset_gpu::rows::CookedRows` from a mapped store or fetched generation,
then builds its `SyntheticMeshes` with `selection`, supplying one stable choice per slot. Pass the same
`CookedRows` to `CookedConfig::rows`. The game connects the generic renderer handle to residency; the
renderer has no asset-store dependency. `FrameHost::new_with_rows` demonstrates initial preparation.
A browser host prepared before its fetch connects its loading selection with `with_reader`.

Residency resolves every configured row through its existing loader. Conversion checks the arena's admission,
copies positions, UVs and indices exactly, decodes normals and packs geometry before the renderer takes it.
Unchanged products retain their CPU geometry identity, including across device reconstruction.

The host writes at most 1 MiB of row geometry per recording into space no drawn row uses. Submission advances
the write cursor. Residency waits for both its own copies and all row writes before offering one publication.
Every reader switches in its next frame; a discarded frame retries the same switch. Submission retires the
old geometry. The first history-advancing frame after a switch resets reconstruction, including when a capture
committed the switch first. Missing or invalid products and arena refusals retain the previous publication whole.

Same-device visual replacements retain the live arena together with its allocations, write cursor and pending
switch. At publication the host refreshes the new scene's draw ranges, caster bounds, culling tables and vertex
bindings against that arena. This also applies to detached extent candidates and candidates prepared before a
later row edit commits. Preparing a candidate does not change the active reader's readiness.
A replacement-device candidate cannot inherit buffers. If a row committed after its CPU snapshot was prepared,
publication refuses that stale candidate and leaves the active host intact; take fresh `device_prepare_inputs`
and prepare again. Device reconstruction otherwise retains the newest committed CPU geometry as before.

Live arenas reserve two buffers up front, each bounded by the device's storage-binding limit; ordinary selections
retain their existing allocation path. On the portable Metal profile this reserves 268,435,424 payload bytes.
`MeshRows::with_binding_limit` selects a smaller scene capacity before preparation. Initial rows, replacements
and overlap must all fit. Free ranges coalesce and retired ranges are reused; edits create no pipelines or graphs.
`MeshRows::report` records the bound, bytes, high-water endpoints and up to 1,024 row-write/steady CPU samples.
These timings come from a shared-machine run and do not qualify display cadence or physical GPU residency.

Reproduce native function with the `cooked_rows_gpu` game test and browser function with
`node scripts/check_v1_browser.mjs --case mesh-rows`. Build the web game first with
`sh examples/v1-game/web/build.sh`. For the realtime inspection, build `v1-game` in release mode and run
`python3 scripts/capture_mesh_rows.py --output tmp/content-loop-m2/new-capture` under the GPU lock.
It edits only its copied sources, doubles the rock's height, then shrinks it to 40%, while T2 captures a camera pan.
