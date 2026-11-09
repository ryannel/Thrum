# APPLE-TILE (G3t) design review

9 November 2026. Independent `/review-milestone` design review of revision `74be9f89`: the
[brief](../../design/modules/APPLE-TILE-BRIEF.md), the [design](../../design/modules/APPLE-TILE.md), the
[plan](../../design/modules/APPLE-TILE-PLAN.md) and the [done-when file](../../design/modules/APPLE-TILE-DONE.json),
against C04/C06/C07/C08/C12 and the current consumers on that revision. Reviewer: Claude Opus 5.5. No milestone is
judged, so the done-when file is unchanged. Nothing was built or run on a GPU. This is a document and source read,
including the locked wgpu 30.0.1 sources in the cargo registry.

**Verdict: DESIGN CHANGES, 9 findings.** The design is well researched, and most of its mechanisms check out against
the code and the locked wgpu sources. But four of its proofs can pass while the work draws less or does something
different, and two mechanisms conflict with existing owners or decisions. These need settling before M0 is built
past its proof tooling.

## Capabilities derived from the brief (written before reading the design)

1. The same cooked valley, with sky, Sun, stars, local lights, shadows and mirrored geometry, survives moving views,
   a cut, day and night, dropped views, slot reuse, resize and device replacement.
2. One immutable generation registry. Prepared portable fallbacks. Every required resource and pipeline exists
   before readiness.
3. Attachment load/store and transient eligibility follow actual readers, including occlusion, capture and future
   reconstruction. Logical bytes are never reported as physical bandwidth.
4. Uploads cannot race GPU reads. Dirty data survives skips and slot reuse. Compact transport keeps every value a
   supported shader reads, including supplied normals and history.
5. Light overflow and mirrored/degenerate rasterization keep established semantics. Cost rows expose CPU fallback
   and validation work.
6. G4 gets one prepared, generation-owned native boundary with graph resources, a fallback, reset, completion,
   errors and timing. No rival queue, raw drawable or mixed encoder.
7. Correctness, agent visual review and lifecycle evidence are mandatory. Numbers are baselines and tripwires.
   Promotion only on a measured win.
8. Same cook, same contribution seam, no new schema. Extend existing crates. Register dependencies and justify unsafe
   code.
9. Offscreen proof proceeds while Mac M2 waits. Live/display qualification is recorded separately.
10. Proofs must not pass while less required work is drawn. Milestone dependencies must resolve.

## What checks out

- **Compact row.** 48 + 12 + 4 + 48 + 16 = 128 B, with the stated offsets. Every supported shader reads only
  `previous[3].xyz` from the previous matrix (`synthetic.wgsl:125`, `visibility.wgsl:129,166`,
  `history_gpu.wgsl:35`, `shadow.wgsl:53`). Current transforms are admitted affine (`upload.rs:363-377`). Keeping
  the supplied normal columns satisfies capability 4.
- **Overflow ranking.** The CPU oracle in `cluster.rs` uses the grid, capacity and tie rules the design states:
  32×18×16, capacity 32, worst = lowest score and then highest index, replaced on a greater score or an equal score
  with a lower index, and IDs sorted ascending. The current GPU builder is the four-source, fixed-stride path
  (`cluster_gpu.rs`, `cluster.wgsl`), as the design says.
- **Locked wgpu facts.** wgpu-types 30.0.1 has `TRANSIENT_ATTACHMENT` and `transient_saves_memory`. wgpu-core
  restricts transient usage to `TRANSIENT_ATTACHMENT | RENDER_ATTACHMENT` (`device/resource.rs:1519`). wgpu-hal
  Metal maps it to `MTLStorageModeMemoryless` when supported (`metal/device.rs:561`). wgpu-core's
  `EncodingApi::{Wgpu, Raw}` forbids mixing on one encoder (`command/mod.rs:311,552`). So the whole-raw-encoder
  segment is the right shape, and the design rightly qualifies Metal hazard tracking rather than assuming it.
- **Dependencies.** `PLATFORM-RUNTIME-MAC:M1` is `done`. G4's dependency table points only forward at G3t
  M0/M1/M3/M11/M12, so there's no cycle.
- **Checks.** The cited checks exist (`metal_motion_history_*`, the `creation-counters` feature, the `moving`
  subcommand), and the GPU-using checks carry `lock: gpu`.
- **Determinism.** Byte-identical same-target repeats are already established on this route
  (G3S/G7 evidence), so D0.2's exact-repeat rule is achievable.

## Findings, worst first

### 1. The Chrome-only items can pass while both targets draw less (verified)

D1.4, D3.4, D7.4 and D12.4 are proved only by `python3 scripts/run_v1_showcase.py moving`. In
`scripts/run_v1_showcase.py:801-880`, that command compares Chrome against native from the same revision under the
32×18 coarse-grid `THRESHOLDS`, plus a repeat check. It doesn't compare against a pre-change baseline. So:

- D1.4's "unchanged route images" is never checked.
- A change that drops content on both targets passes.
- D7.4 ("Chrome shaders consume the same private transport") passes if Chrome stays on the old 192-byte row.
- D1.4's "no native dependency" isn't examined.
- D12.4's "retains lighting/visibility/present semantics" is a 32×18 grid judgement.

The brief explicitly asks reviewers to reject "proof that could pass while drawing less required work".

*Change.* Give each of these items an `ask` or `inspect` that settles its own statement:
- D7.4: the WGSL stride/offset test runs on the WebGPU path, or the shader source traces to the 128-B struct.
- D1.4: the wasm dependency tree is free of `apple_tile`/native crates.
- D1.4 and D3.4: the finer G3t comparator against a recorded pre-change portable baseline, at full resolution.
- D12.4: the light/visibility witnesses.

### 2. M9's mirrored ranges have no existing owner on the supported path (verified)

On the supported legacy route, every instance is drawn with one scene-wide pipeline, and that pipeline is two-sided
(cull none) whenever any instance is mirrored or degenerate: `needs_two_sided` is `determinant <= 1e-12`
(`upload.rs:285-289`), and `forward.rs:87-96` uses it. The only ordinary/mirrored/two-sided range split is the
experimental G3c M3b bucket route (`buckets.rs:1-47`, `mesh_route.rs:1-70`), which is "private until root reviews
the wiring". The design forbids promoting it.

So D9.1's "existing V2 membership/facing authority supplies ordinary/mirrored indirect ranges" names ranges that
don't exist on the supported path. M9 must either qualify the bucket route, which the design forbids, or build a
second class/range mechanism beside it, which is a rival to V2's own experimental implementation.

The change also replaces the established cull-none result for mirrored rows with back-face culling. That's
image-identical only for closed meshes. An open or single-sided mesh loses its back side, against the brief's
"preserve established renderer semantics".

The valley's mirrored geometry is also narrower than the cue table implies. Cooked placements refuse mirrored
transforms (`thrum-asset-gpu/src/placement.rs:196`), and the cook bakes mirrored winding (`thrum-cook/src/mesh.rs:5`).
So the only runtime mirror is overlay slot 5060 (`examples/v1-game/src/scene.rs:36`).

*Change.* Choose and state one owner for class ranges:
- either M9 qualifies the bucket route's class split with the G3c/V2 owner's review;
- or M9 defines the shared range mechanism that the bucket route will also use.

State the admission rule for open meshes, keeping two-sided where closure isn't known. Name the synthetic overlay
as the mirror fixture.

### 3. Per-slot buffers on portable contradict the boundary decision and add unaccounted cost (verified)

D4.1 and the Uploads mechanism give both recipes per-slot instance, light, cluster and bounds buffers. The
[boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) (line 15) puts per-slot upload rings in the
`apple_tile` backend.

Today, portable uses single buffers with queue-ordered `write_buffer` (`synthetic.rs:1794-1825`), which is already
race-free with two frames in flight. On portable, per-slot copies double the resident bytes. Under steady motion
they also roughly double the dirty upload bytes, because each change is written into both slots as each one
reconciles. Per-slot copies also add the dirty-cursor complexity of D4.2 where there was none.

The cost account counts the extra copy but not the doubled upload, and it states no portable benefit.

*Change.* Either restrict per-slot storage to the recipe whose pointer writes need it, or justify the portable cost.
Add the portable upload-byte delta to D4.3's recorded rows as a tripwire.

### 4. The recipe tolerance isn't derived and can absorb small omissions (suspected)

CIEDE2000 p99≤2 and max≤6 is called provisional, but the design never derives it from expected sources of
difference. The mechanisms that legitimately separate Apple tile from portable shouldn't change the output:
- memoryless and transient storage;
- shared uploads;
- compact rows that copy the same f32 values;
- exact light membership;
- the same shaders.

So the expected difference is near zero. A max of 6 ΔE could hide a missing faint star, one dropped weak light in a
saturated cell, or a few pixels of a far cube. D0.2's negative controls (a mismatched phase, a missing contribution)
are gross failures that any metric catches.

*Change.* For each milestone, state where Apple-vs-portable differences come from, and require exact equality
where none is expected. Add small-omission negative controls to D0.2 that must exceed whatever tolerance is
selected: drop one local light, one mirrored cube, and the star layer at night.

### 5. D8.1 can close M8 with every overflow case on the CPU fallback (verified from the item text)

D8.1 says "for qualified 0/4/16/32/64-light cases", and a ranking disagreement selects the CPU path and "cannot
count as GPU-qualified". As written, the 32 and 64 overflow cases can all fall back while 0/4/16 match. D8.1 would
then pass, and M8 ("GPU light lists preserve the established lighting choice") would close without any
GPU-qualified overflow, which is the point of the milestone.

*Change.* Require that the named cases, overflow included, are GPU-qualified on mac-metal. If a case can't be
qualified, D8.1 records it as an explicit unqualified outcome that doesn't satisfy the item, and that is a re-plan
decision.

### 6. Promotion evidence omits C12's thermal row and the brief's validation-work row (verified)

C12 says a dedicated recipe's comparison records "thermal state after a sustained run; without comparison it is not
performance-qualified". Neither the design, D5.3, D12.1 nor D12.2 mentions thermal state or a sustained run. The
brief asks cost rows to expose "CPU fallback and validation work". D8.2 and D12.1 record fallback but not validation
work. So D12.2 could record a "win" that C12 says isn't qualified.

*Change.* Add the sustained run, the thermal state and validation/admission CPU cost to D12.1's recorded rows, and
make promotion in D5.3 and D12.2 require them.

### 7. G3t M2/M3 replace the live present path that Mac M2 is qualifying, and no item runs it on a Mac window (verified)

M2 (late acquire, merged graph) and M3 (tone moved into present, intermediate removed) change the surface path that
`PLATFORM-RUNTIME-MAC` M2 measures. Mac M2.1 already passes on the current path. Neither plan says whether landing
G3t M2/M3 invalidates that evidence, or which revision Mac M2 qualifies.

G3t's own proofs of the merged surface path are an acquired-format texture (`examples/v1-game/tests/surface_gpu.rs`,
which says "a real window/swapchain is exercised separately") and the iOS simulator. D12.3 lets the module close
with the real macOS windowed path "recorded as owed". That's right for cadence, but it means a functional break in
the Mac window could ship unseen.

*Change.* State the order in both plans: Mac M2 is either qualified before G3t M2/M3 land, or re-run after them.
Add a functional windowed-run item for mac-metal at M2/M3 or M12, kept separate from cadence. Without a display
slot it stays open rather than waived.

### 8. Prepared fallback allocations aren't costed and could cancel the memory saving (suspected)

"All selected and fallback pipelines/bind groups/variant allocations are prepared before readiness." If the
prepared portable fallback includes a stored twin of every memoryless attachment, Apple tile saves no memory. The
cost account doesn't say which fallback resources are allocated and which are rebuilt through the existing
generation-replacement path.

*Change.* Say which resources are resident for the fallback, count them in the cost account, and have D1.2/D1.3
report the resident total under both recipes.

### 9. D0.3 carries `status: pass` with plan prose as its evidence (verified, low)

D0.3's `evidence` points at the plan's Carried work section rather than a check output, and the plan itself says to
rerun it on the integrated M0 revision. The reviewer will rerun the check anyway. But the builder's claim should
point at actual output from the M0 revision when M0 is claimed. D0.4 is acceptable as an `ask` over the retained
probe document.

## Done-when judgeability

Apart from the findings above, I could judge each item. Most items pair an `ask` with an `inspect` that names the
adverse cases, and several carry useful falsifiers: D4.2's failing cursor, D6.2's early overwrite, D9.2's
cull-none, and D10.2's no-op native segment. The weak ones are the Chrome-only checks (finding 1), D8.1's
"qualified" escape (finding 5), and every image item that relies on the underived tolerance (finding 4).

DESIGN CHANGES: 9 findings
