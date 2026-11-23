# REVIEW-RENDER plan

The 14 November review's findings in `thrum-render` are fixed: its confirmed bugs, hand-packed GPU records, integer
pass identities, string errors and wasted work, with no retained pixel moved. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms), [the rulings](../ARCHITECTURE.md#seam-rulings-for-track-4) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. Exact images are proved once, on main
after the merge, by [the shared run](../ARCHITECTURE.md#proofs-run-once-on-main): don't run the verifier yourself.
Set each milestone's verifier item `pass` with the evidence `main-branch check exact-images`.

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: nothing here starts until the five milestones other builders hold in the renderer are Done, so the renderer's confirmed bugs are fixed after them, not at once. The other choice interrupts those builders.
- Assumption: two cost findings wait: many local lights built on the CPU each frame (M24), for the performance phase after the V1 exit, and the sky rebuilt on every Sun change (M31), for the quality governor's cadence controls.

## M1 Each light buffer has one writer, a failed surface encode says why, and the same fault is reported the same way wherever it is raised

The light split is in `synthetic.rs` (`cpu_light_bytes`), `atmosphere/graph.rs`, `atmosphere/diffuse.wgsl`,
`atmosphere/products.wgsl` and `atmosphere/products.rs`. `thrum-asset-gpu`'s cooked shader and `thrum-sea` read the
light group through the contribution seam, so they follow its layout. The mode with no atmosphere is
`FrameLighting::Photometric` and `Diagnostic`: `pack_directional` writes the constant ambient at bytes 48 to 60,
and `ambient_irradiance` in `synthetic.wgsl` picks it when the mark is clear. Take the photometric scene's bytes
before you change anything. M1.5 splits today's `ambient_irradiance(n)` into the source read `ambient_source` and
the surface query, and changes no arithmetic ([Mechanisms](DESIGN.md#mechanisms), mechanism 1). The query's
callers are `shade` and the two composed variants (`synthetic_environment.wgsl`, `synthetic_plain.wgsl`); the
reflection test shader in `atmosphere/reflection/tests.rs` stubs it. The cooked volume in `thrum-asset-gpu`'s
`cooked.wgsl` calls `ambient_irradiance` once before its march today and moves to `ambient_source`, as the stub in
`thrum-asset-gpu/src/draw/tests.rs` does. The atmosphere's lent `environment_diffuse` and
`environment_reflection` are not part of it. The dropped encode is in `runtime.rs`
(`encode_surface(...).is_ok()`), and the two blanket conversions are beside `FrameError` there.

## M2 Every record a renderer shader reads is one Rust struct and one WGSL declaration, held equal by a test

The derive to copy is in `crates/thrum-asset/src/layout.rs` and the naga test at the end of
`synthetic/view.rs`. `REVIEW-ENGINE` M2 builds the same reflection test for the sea; use one shape for both.

## M3 Passes, stages and pipelines in the renderer are named by typed keys, so a rename or a new pass cannot silently drop a timing bucket or a barrier

The cleanest existing declaration is the graph's `Pass` and `Access`. `CREATOR`'s profiler, the frames report and
the Mac qualification read pass names and buckets from the frame timing feed: they must see what they saw.

## M6 The atmosphere stage registry is checked on a small fixture

Name each case of the stage registry regression with the `stage_registry_` prefix in the
`atmosphere_stage_table` test target. M6.1 selects the whole group whether the proof uses one case or
separate CPU and Metal cases, and fails on an empty selection.

Record each listed test’s disposition and the coverage that replaces it if split. For each fast case,
record before and after per-test times on the Mac in the default nextest test build used by the workspace
check, excluding compilation. Measure the revised cases alone and run them as a focused batch alongside
normal builder work. Use the supplied run for the starting times where available. The batch passes the
existing ten-second limit on its first run.

These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

- 9.200 s — `thrum-render::atmosphere_stage_table every_stage_reads_its_table_and_takes_its_role_bit_and_bucket_from_its_key`

## M4 A renderer failure can be matched and printed, and the crate's error no longer lives in the scene renderer

Every crate that names the render error follows: `thrum-app`, `thrum-asset-gpu`, `thrum-sea`, the examples. Convert
at their edge and leave their own errors to `HOST` and `REVIEW-ENGINE`.

## M5 A frame's GPU timing is valid, belongs to its frame and allocates nothing, and a resize compiles one graph

The frame-allocation test is `examples/v1-game/tests/frame_alloc.rs`. `GpuTimings::resolve_if_ready` in
`timing.rs` holds the completion boundary and `try_collect` already refuses zero and inverted counters: keep both,
and do not move the resolve into the frame's encoder. Take the paired frame records at the commit you claim and
say what else the machine was doing.
