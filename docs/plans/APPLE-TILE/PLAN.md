# APPLE-TILE plan

The Apple tile recipe and backend dataflow (gate G3t), governed by
[the boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07, C08 and C12.
[Design](DESIGN.md) · [brief](BRIEF.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Work packages run in the order WP0, WP1, WP2, WP3, WP10, WP11, WP12, WP13; the IDs are kept for consumers, and M4–M9 were removed
as deferred work. Read [Sequence and dependencies](DESIGN.md#sequence-and-dependencies) and
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) before starting any work package.

<a id="m0-proof-route-counters-and-capture-probe-pending"></a>

## WP0 A repeatable offscreen valley proves the comparison path: done
[review](../../../.orchestrate/evidence/APPLE-TILE/WP0-review.md)

## WP1 The valley selects a recipe and stores only actual future inputs

Criteria: D1.1–D1.5 in the done-when file.

- [x] Add truthful registry/override, per-variant storage and ready fallback; preserve COOKED-DRAW facts/selection. Prove Mac/Chrome exact bytes and simulator diagnostic function.
- [x] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## WP2 The existing frame lifecycle runs through one split graph

Criteria: D2.1–D2.5 in the done-when file.

- [x] Resolve review MF1 by rerunning both exact matrices on the jitter-gated release build, retaining revision, clean-diff and binary identity with unchanged WP0 baselines.
- [x] Merge graph and late surface binding; preserve refusal/loss/background/history rules. Complete attended OS-window matched-state reference comparisons and functional recovery.
- [x] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## WP3 G4 can replace the pre-tone linear input without changing SDR output

Criteria: D3.1–D3.5 in the done-when file.

- [x] Settle review MF1 with fresh both-recipe OS-window observations on the exact reviewed binary in an unlocked session, retaining matched paused references, input, resize and replacement recovery.
- [x] Expose one colour/exposure input with exact identity substitution; keep tone/intermediate/present arithmetic. Migrate pass-name timing and prove attended window function.
- [x] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## WP10 A stateful native segment composes inside one submission

Criteria: D10.1–D10.4 in the done-when file.

- [ ] Start with exact same-submit copy/select fixture and twin. Then prove route/history, held lifetime, discard/reset and diagnostic simulator background race.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## WP11 Native providers survive replacement and expose truthful timing

Criteria: D11.1–D11.3 in the done-when file.

- [ ] Use bounded extent fixtures and prepared twins for replacement/rejection/errors, complete generation retention and explicit clock-domain timing.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## WP12 An integrated record states qualified wins and remaining targets

Criteria: D12.1–D12.6 in the done-when file.

- [ ] Rerun retained mechanisms and full workload, actual final Mac window, exact Chrome baseline, and COOKED-DRAW-dependent simulator function; record sustained costs and owed qualifications.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## WP13 The native Metal step runs any provider through one interface, and the copy/select witness is the first

[The design](DESIGN.md#the-native-segment-and-its-providers-wp13) says what the segment keeps and what a provider
supplies. The lifecycle to lift out is in `crates/thrum-render/src/synthetic/native_witness.rs`, the raw recording
in `crates/thrum-render/src/apple_tile.rs`, and their callers in `runtime.rs` and `synthetic/scene_graph.rs`.
`APPLE-PRESENTATION` WP8 looks for the interface in `crates/thrum-render/src/apple_tile/`. The frame sample's
native outcome and the witness's report are read by the governor, submitted capture, `thrum-app`'s qualification,
the game's capture and the iOS shell: keep each working. `APPLE-PRESENTATION` WP8's unmerged checkpoint
(`build/apple-presentation` at `2252c56af`) changes `examples/v1-game-ios/src/ios/render.rs` and `witness.rs`, so
change there only what a rename forces. D13.2's second provider is test data: make it as small as its four cases
allow, and keep it out of ordinary builds where the code lets you.

Stop rules, for every work package:
- D2.5, D3.5 and D12.3 need attended setup: an unlocked active Mac session and Screen Recording permission for the
  OS capture process. Missing setup leaves those rows open while unblocked work proceeds; never automate a grant.
- WP12's cooked simulator scene waits on `COOKED-DRAW`; never build a substitute.
- Never silently refresh a baseline, and never widen a comparison metric in place of exact bytes.
