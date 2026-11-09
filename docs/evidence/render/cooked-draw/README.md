# COOKED-DRAW M1 blocked checkpoint

9 November 2026. **M1 is not claimed.** The proposed one-draw fallback fails M1.3's exact-byte oracle on
Apple M3 Max / Metal. The capability record, optional cull resources, shared history work, direct whole-range draw,
iOS cooked attachment and both-path cooked GPU suite are implemented in this worktree. No reviewer was launched.

## Decisive result

The [named parity command](../../../../scripts/cooked_draw_parity.py) renders ordinary paused-world submissions
through the T2 submitted-output tap at 1920×1080, with one cooked showcase store. The first required view (valley,
noon, yaw 0°) fails. Both arms repeat exactly; the two paths differ at **1,506 pixels**, maximum channel delta 18.
The difference occupies x=23–1898, y=359–827, along terrain triangle edges. Original final images:
[indirect](indirect.png), [direct](direct.png), [difference mask](difference.png).
[Run record](paths-failure.json.gz) pins the binary, content products, adapter facts, paths and hashes.

Two scratch-only shader controls isolate draw partitioning:

- [Retain all meshlets](cull-diagnostic.json): indirect keeps 680/680 rather than 171/680, but its final bytes are
  unchanged. The mismatch is **not explained by rejected meshlets**.
- [One full-range indirect command per mesh](single-diagnostic.json): output is byte-identical to direct, still
  differing from the meshlet-partitioned indirect result at 1,506 pixels.

This establishes a draw-partition-dependent result on this pipeline/driver, not its low-level numerical cause.
Both controls restored `cull.wgsl`; neither changes the production indirect path. Their reproducers are
[keep-all](diagnose.py) and [single-command](diagnose-single.py), run from the repository root under the GPU lock
*after* the named parity command has populated `tmp/cooked-draw/paths-failure/`.

**Planner decision required:** reconcile the single whole-range draw with exact parity against meshlet-partitioned
indirect drawing. Investigate a partition-invariant shared shader/raster mechanism, or explicitly re-plan the draw
strategy/oracle. Do not silently substitute per-meshlet direct draws, alter indirect rendering, or loosen equality.

## Other proof and limits

The fresh owned iPhone 18 Pro / iOS 27.2 simulator reports adapter flags without `INDIRECT_EXECUTION`, selects
Direct, draws cooked content and advances presented/cooked counters while the console is locked. The final [function/lifecycle record](ios-function.json.gz) proves 12 advancing samples over
22.82 seconds, a suspended hold without presented/cooked counter or generation changes, foreground resume and
device generation 1→2 with `cooked_device_generation=Some(2)`, content generation 1 drawn and no errors.
Both console lock witnesses are true; the owned simulator was deleted. M1.7 and M1.8 pass.
The [selected simulator screenshot](ios-valley.png) is function evidence only. The full [repository check](check.json) passes: 651 tests in 62 result
groups; the 13 cooked GPU cases cover both draw paths. The existing material suite remains indirect-only. Simulator screenshots show textured ground, metallic
sphere, matte rock and Sun shadows. They also show the existing 640×360 render image letterboxed in portrait.
No rotation, low-sun/diagnostic view or sampled pan acceptance is claimed. Simulator function only; no speed result.

Remaining: complete allocation/command audit, both-path material-promotion qualification, all other M1.3 views, Mac/Chrome base comparison, paired cost
runs, G5 tripwires, rotation and the complete simulator inspection/pan. The parity command fails closed after the
first view; base/Chrome arms explicitly refuse as unimplemented. T2-CAPTURE M2 remains blocked on M1 acceptance.

This checkpoint is retained locally, without integration or a milestone claim, because its acceptance oracle fails.

## Reproduce and retention

Use the profile GPU lock around `python3 scripts/cooked_draw_parity.py paths --target mac-metal --output
 tmp/cooked-draw/paths-failure` (new directory; expected exit 1). The recorded pre-implementation [BASE](BASE) is
not a before/after parity result. `scripts/cooked_draw_ios_sim.py --lifecycle --output <new-dir>` builds, cooks,
creates/cleans an owned simulator and fails closed. `--rotation` is implemented but unqualified.

Owned scratch is `tmp/cooked-draw/` (owner record: `tmp/cooked-draw/OWNER.md`); selected PNGs and compact
records above are retained. Reconstructible scratch remains only until the planner resolves this blocker and the
next builder has inspected it. No source asset or unique master is held there.
