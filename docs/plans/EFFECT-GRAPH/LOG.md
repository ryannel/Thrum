# EFFECT-GRAPH log

- 2026-11-11 Planned from the owner's task, quoted in the design, and the brief in the module list. Three
  milestones: texture kinds in the graph (after `APPLE-TILE:M2`), persistent state and cadence through a diagnostic
  effect, and the stage after opaque (after `APPLE-TILE:M3` and `APPLE-PRESENTATION:M2`). The port's graph condition
  is now met at M3, and the module list and the `SEA` and `CLOUDS` briefs say so.
- 2026-11-11 Planner's decisions: effects extend the existing contribution seam, and a scene holds an ordered list
  of contributions; cadence is a runtime skip under one compile-time rule, not a recompiled graph; state commits
  only with a submitted frame. Researched: Unreal's RDG, Unity's render graph, WebGPU's limits and storage-texture
  access. The storage-texture count and view rules were not re-read from the specification; M1.3 records them.
- 2026-11-11 The heavy checks (release build, GPU, Chrome) were not dry-run by the planner, for lack of disk and
  because the GPU is shared. Each check was parsed as shell and the done-when file validated with the loop's own
  validator.
- 2026-11-11 [Design review](../../evidence/EFFECT-GRAPH/design-review.md): changes requested, four Must Fix. MF1:
  the tick rule reset state on a repeated tick. MF2: a skipped ping-pong writer left readers on the older side, and
  time-sliced writes had no initialisation. MF3: M3.1 removed stored depth with the refraction request. MF4: M3's
  bar promised stability under reconstruction with no reconstructor in its `needs`.
- 2026-11-11 Design revised. MF1: a state table (equal tick keeps, later advances, earlier or a new timeline
  resets), and M2.2 asserts its rows directly. MF2: each history pair has its own committed side with `previous`,
  `next` and `result` bindings and flips only when its producer ran; three cadence classes; full frames on a new
  epoch and on reset; time-sliced passes may not write a history. M2.3's witness uses two histories and a consumer.
- 2026-11-11 MF3: depth storage follows the union of its readers and the colour copy its own request; M3.1 covers
  four cases. MF4: M3's bar and name now stop at reconstruction's inputs, and reconstructed appearance is judged
  in `SEA` and `CLOUDS`; `APPLE-PRESENTATION:M3` was not added to `needs`, so M3 is still built beside TAAU.
- 2026-11-11 Suggestions applied. S1: M2.5 names the cube, volume and array-mip readbacks in Chrome. S2: M2.7 uses
  the executor's named pass timings and leaves the host's sample fields to `APPLE-TILE:M3`. S3: list order is an
  execution order only; C08's segment admission stays with `SEA`, `CLOUDS` and V3.
- 2026-11-11 Planner's additions while revising: a multi-tick advance is one execution with a count, never a
  producer run twice in a frame; a same-generation `rebuild` keeps state unless the effect's descriptors changed.
  Exact baselines, from `BASELINE-GUARD`'s review: M2 gained `BASELINE-GUARD:M1` in `needs`, M2 and M3 carry the
  verifier, and M1 keeps the native matrix and does not wait (an assumption in the plan).
- 2026-11-11 M1 implementation checkpoint `3d56bf21`: explicit 2D/volume/cube descriptors, natural or declared
  binding views, exact-dimension transient allocation and capability-record validation. Existing declarations keep
  their 2D kind. [Partial proof](../../evidence/EFFECT-GRAPH/M1-proof.md): all 14 Metal tests and the Chrome
  capability recorder pass; pre-M1 day/night/capture compilation is pinned unchanged. Clippy and check-docs pass.
  M1.5 tried its named release/cook/native matrix command and refused before capture with less than 16 GiB free
  (about 9.4 GiB available). The full repository check refuses the missing exact-baseline proof. M1 remains open,
  unclaimed and unintegrated; free disk with build/capture headroom, then rerun exact proofs and required checks.
- 2026-11-11 The clean-checkout registered exact verifier was also attempted under the GPU lock and refused
  `LOW_DISK_SPACE: verifier requires 16 GiB free`, even after removing owned debug outputs. Saved its refusal
  with the M1 proof and cleaned the remaining owned build products. No integration was attempted without the
  required exact proof; M1.5 stays open.
- 2026-11-12 Resumed M1 with disk headroom and rebased onto main. Both native exact matrices pass
  (38 plus four comparisons), and all four Chrome baseline sets pass (481 records each), under the GPU lock.
  [Target proof](../../evidence/EFFECT-GRAPH/M1-proof.md) records the source and artifact identities. All five
  items pass; M1 is claimed for review. The required milestone check passes all five named commands and `./dev check`.
- 2026-11-12 The required repository gate exposed eight existing Hi-Z consumers rejected by the new
  view validation. Their logical pass aggregates successive single-mip views in one binding slot. Corrected
  the validator to preserve that contract and keep explicit storage views single-mip; added the aggregate
  range witness and tightened the explicit-view refusal. All 242 renderer library tests and all-target Clippy
  pass. Fresh exact proofs and the complete milestone check pass before integration.
- 2026-11-12 M2 implementation checkpoint: ordered public contributions, named simulate declarations and cadence
  refusals, source-clock frames, independently committed histories and wrapping anchors, and an opt-in integer
  diagnostic with native/Chrome qualification entry points. Pinned native all-target Clippy and Wasm Clippy pass.
  CPU state-table and cadence-refusal checks passed before the Metal scene-snapshot witness was added. Target
  scripts, realtime trail inspection, exact baselines and integration remain unqualified; no M2 item is claimed.
- 2026-11-12 M2.6 and M2.7 have [final-source realtime evidence](../../evidence/EFFECT-GRAPH/M2-proof.md).
  The builder inspected the complete sequence and native-size consecutive crops: the trail persists and fades,
  stays on the ground through the wrap, and does not jump at the cadence change. Named executor times, descriptor
  bytes and load are recorded alongside the no-effect arm. Six automated item checks remain in progress under
  their GPU locks; the milestone is not yet claimed for review or integrated.
